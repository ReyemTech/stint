use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::{entries::Entries, queue::Queue};
use stint_core::sync::drain_once;

use super::open_store;

#[derive(Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub sub: Option<SyncCmd>,
}

#[derive(Subcommand)]
pub enum SyncCmd {
    /// Drain the sync queue once (default when no subcommand given).
    Drain,
    /// Resurrect queue rows previously parked far in the future by the
    /// abandon-on-4xx path. Their attempts counter resets so the worker
    /// gives the new code a fresh try.
    RetryAbandoned,
    /// Manually link a local pending_create entry to an existing remote
    /// id. For unsticking entries where adopt-on-overlap couldn't
    /// auto-resolve (e.g. the remote's start differs from local). Drops
    /// any queued create_entry op for this uuid.
    ForceAdopt(ForceAdoptArgs),
    /// Print every currently-running entry Solidtime sees for the
    /// configured member, regardless of project / filter. Diagnostic
    /// only — useful when overlap rejections happen but the Solidtime
    /// web UI doesn't show what's blocking.
    Active,
    /// Dump every Solidtime entry whose time range intersects a local
    /// entry's [start, end] — the real overlap set. Solidtime forbids
    /// any range overlap (running OR completed), so a stuck `overlap`
    /// 400 with no visible active entry usually means a completed
    /// entry is the actual blocker.
    Diagnose(DiagnoseArgs),
}

#[derive(Args)]
pub struct DiagnoseArgs {
    /// Local UUID of the entry to investigate.
    pub local_uuid: String,
}

#[derive(Args)]
pub struct ForceAdoptArgs {
    /// Local UUID of the entry to mark synced.
    pub local_uuid: String,
    /// Solidtime remote ID (UUID) to link.
    pub remote_id: String,
}

/// `stint sync` with no subcommand defaults to drain — keeps the old
/// invocation working.
pub async fn run(args: SyncArgs) -> Result<()> {
    match args.sub.unwrap_or(SyncCmd::Drain) {
        SyncCmd::Drain => drain().await,
        SyncCmd::RetryAbandoned => retry_abandoned().await,
        SyncCmd::ForceAdopt(a) => force_adopt(a).await,
        SyncCmd::Active => active().await,
        SyncCmd::Diagnose(a) => diagnose(a).await,
    }
}

async fn diagnose(args: DiagnoseArgs) -> Result<()> {
    let store = open_store().await?;
    let entries = Entries::new(store.clone());
    let entry = entries
        .get(&args.local_uuid)
        .await?
        .ok_or_else(|| anyhow!("entry {} not found locally", args.local_uuid))?;

    println!("Local entry:");
    println!("  uuid:         {}", entry.local_uuid);
    println!("  description:  {:?}", entry.description);
    println!("  start_at:     {}", entry.start_at);
    println!(
        "  end_at:       {}",
        entry.end_at.as_deref().unwrap_or("(running)")
    );
    println!("  sync_state:   {}", entry.sync_state);
    println!(
        "  solidtime_id: {}",
        entry.solidtime_id.as_deref().unwrap_or("(none)")
    );

    let member_id = Settings::new(store.clone())
        .get("solidtime.member_id")
        .await?
        .ok_or_else(|| anyhow!("solidtime.member_id not set"))?;
    let client = build_client(&store).await?;

    let our_start = stint_core::time::parse(&entry.start_at)?;
    let our_end = match entry.end_at.as_deref() {
        Some(e) => stint_core::time::parse(e)?,
        None => stint_core::time::now(),
    };

    // Query a wide-enough window that we catch any entry whose start
    // could plausibly land inside our [start, end] — 24h before our
    // start covers same-day stale running timers; 1s past our end picks
    // up adjacent rows.
    let from = stint_core::time::format(&(our_start - chrono::Duration::hours(24)));
    let to = stint_core::time::format(&(our_end + chrono::Duration::seconds(1)));
    println!("\nQuerying Solidtime for entries in [{from}, {to})…");
    let candidates = client.list_time_entries(&member_id, &from, &to).await?;
    let active = client.list_active_time_entries(&member_id).await?;

    // Range-intersect filter: a remote overlaps our [our_start, our_end]
    // when remote.start < our_end AND (remote.end IS NULL OR remote.end >
    // our_start). Walk both lists; dedupe by id.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut overlapping = Vec::new();
    for r in candidates.iter().chain(active.iter()) {
        if !seen.insert(r.id.clone()) {
            continue;
        }
        let r_start = stint_core::time::parse(&r.start).unwrap_or(our_start);
        let r_end = match r.end.as_deref() {
            Some(e) => stint_core::time::parse(e).unwrap_or(our_end),
            None => stint_core::time::now(),
        };
        if r_start < our_end && r_end > our_start {
            overlapping.push((r, r_start, r_end));
        }
    }

    println!("\nSolidtime active timers: {} entry/entries.", active.len());
    for e in &active {
        println!("  {} | start={} | {:?}", e.id, e.start, e.description);
    }

    println!(
        "\nSolidtime entries in window (by start column): {}",
        candidates.len()
    );
    for e in candidates.iter().take(20) {
        println!(
            "  {} | {} → {} | {:?}",
            e.id,
            e.start,
            e.end.as_deref().unwrap_or("(running)"),
            e.description,
        );
    }
    if candidates.len() > 20 {
        println!("  …and {} more", candidates.len() - 20);
    }

    println!(
        "\n→ Overlapping local entry's range [{}, {}]: {} match(es)",
        entry.start_at,
        entry.end_at.as_deref().unwrap_or("now"),
        overlapping.len()
    );
    for (e, _s, _e2) in &overlapping {
        println!(
            "  {} | {} → {} | {:?}",
            e.id,
            e.start,
            e.end.as_deref().unwrap_or("(running)"),
            e.description,
        );
    }
    if overlapping.is_empty() {
        println!(
            "  (none — yet Solidtime is still rejecting the POST. \
             That points to a Solidtime-side soft-delete or bug. \
             Safe path: `stint delete {}` to drop the stuck local row.)",
            entry.local_uuid
        );
    } else {
        println!(
            "\nSuggested action: stop / delete the conflicting entry in \
             Solidtime, then `stint sync retry-abandoned`. Or `stint \
             sync force-adopt {} <one-of-the-ids-above>` if it's actually \
             the same entry under a stale id.",
            entry.local_uuid
        );
    }
    Ok(())
}

async fn active() -> Result<()> {
    let store = open_store().await?;
    let settings = Settings::new(store.clone());
    let member_id = settings
        .get("solidtime.member_id")
        .await?
        .ok_or_else(|| anyhow!("solidtime.member_id not set"))?;
    let client = build_client(&store).await?;
    let actives = client.list_active_time_entries(&member_id).await?;
    if actives.is_empty() {
        println!("No active (running) remote time entries for member {member_id}.");
        return Ok(());
    }
    println!(
        "Solidtime has {} active (running) timer(s) for member {member_id}:",
        actives.len()
    );
    for e in actives {
        println!(
            "  {} | start={} | project={} | description={:?}",
            e.id,
            e.start,
            e.project_id.as_deref().unwrap_or("-"),
            e.description,
        );
    }
    Ok(())
}

async fn force_adopt(args: ForceAdoptArgs) -> Result<()> {
    let store = open_store().await?;
    let entries = Entries::new(store.clone());
    let row = entries
        .get(&args.local_uuid)
        .await?
        .ok_or_else(|| anyhow!("entry {} not found locally", args.local_uuid))?;
    entries
        .mark_synced(&args.local_uuid, &args.remote_id)
        .await?;
    let cleared = Queue::new(store.clone())
        .delete_for_entry(&args.local_uuid)
        .await?;
    println!(
        "Linked {} (\"{}\") → remote {}. Cleared {cleared} queued op(s).",
        args.local_uuid, row.description, args.remote_id
    );
    Ok(())
}

async fn drain() -> Result<()> {
    let store = open_store().await?;
    let client = build_client(&store).await?;
    let n = drain_once(&store, &client).await?;
    println!("Drained {n} item(s) from the sync queue.");
    Ok(())
}

async fn retry_abandoned() -> Result<()> {
    let store = open_store().await?;
    let n = Queue::new(store.clone()).resurrect_abandoned().await?;
    println!("Reset {n} abandoned queue row(s); next drain will retry them.");
    if n == 0 {
        return Ok(());
    }
    // Drain immediately so the user sees the result without waiting for
    // the background worker tick.
    let client = build_client(&store).await?;
    let drained = drain_once(&store, &client).await?;
    println!("Drained {drained} item(s).");
    Ok(())
}

async fn build_client(store: &stint_core::store::Store) -> Result<SolidtimeClient> {
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or_else(|| anyhow!("solidtime.url not set"))?;
    let org = settings
        .get("solidtime.org")
        .await?
        .ok_or_else(|| anyhow!("solidtime.org not set"))?;
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    Ok(SolidtimeClient::new(&url, provider).with_org(org))
}
