use anyhow::Result;
use stint_core::store::entries::Entries;
use stint_core::verbs::{self, EntryFilter};

use super::open_store;
use crate::format;

#[derive(clap::Args)]
pub struct Args {
    /// ISO 8601 start (UTC), e.g. 2026-05-10T00:00:00Z
    pub from: String,
    /// ISO 8601 end (UTC).
    pub to: String,
}

pub async fn run(args: Args, json: bool) -> Result<()> {
    let store = open_store().await?;
    let views = verbs::list_entries(
        &store,
        EntryFilter {
            since: Some(args.from.clone()),
            until: Some(args.to.clone()),
            ..Default::default()
        },
    )
    .await?;

    if json {
        crate::render::render(&views, true, |_| {});
        return Ok(());
    }

    // Human output preserves the legacy `[sync_state]` column for debugging.
    // `EntryView` intentionally omits `sync_state` from the wire-stable shape,
    // so we re-fetch rows for that one field. The duplication is bounded — a
    // single user's window query — and keeps the verbs façade narrow.
    let entries = Entries::new(store);
    let rows = entries.list_between(&args.from, &args.to).await?;
    for row in rows {
        let dur = format::duration_hms(&row.start_at, row.end_at.as_deref());
        println!(
            "{}  {}  {}  [{}]",
            &row.local_uuid[..8],
            dur,
            &row.description,
            &row.sync_state
        );
    }
    Ok(())
}
