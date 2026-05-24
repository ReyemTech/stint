use anyhow::Result;
use chrono::{Local, TimeZone, Utc};
use stint_core::store::entries::Entries;
use stint_core::verbs::{self, EntryFilter};

use super::open_store;
use crate::format;

pub async fn run(json: bool) -> Result<()> {
    let store = open_store().await?;

    let today = Local::now().date_naive();
    let start_local = Local
        .from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap())
        .unwrap();
    let end_local = start_local + chrono::Duration::days(1);
    let start_utc = start_local.with_timezone(&Utc).to_rfc3339();
    let end_utc = end_local.with_timezone(&Utc).to_rfc3339();

    let views = verbs::list_entries(
        &store,
        EntryFilter {
            since: Some(start_utc.clone()),
            until: Some(end_utc.clone()),
            ..Default::default()
        },
    )
    .await?;

    if json {
        crate::render::render(&views, true, |_| {});
        return Ok(());
    }

    if views.is_empty() {
        println!("No entries today.");
        return Ok(());
    }

    // See `cmd::list` for why we re-fetch rows for human output (sync_state
    // is intentionally not part of the wire-stable EntryView shape).
    let entries = Entries::new(store);
    let rows = entries.list_between(&start_utc, &end_utc).await?;
    println!("{:>10} {:<40} status", "duration", "description");
    for row in &rows {
        let dur = format::duration_hms(&row.start_at, row.end_at.as_deref());
        let status = if row.end_at.is_some() {
            row.sync_state.as_str()
        } else {
            "RUNNING"
        };
        println!("{:>10} {:<40} {}", dur, &row.description, status);
    }
    Ok(())
}
