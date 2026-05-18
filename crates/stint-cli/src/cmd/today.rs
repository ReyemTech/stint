use anyhow::Result;
use chrono::{Local, TimeZone, Utc};
use stint_core::store::entries::Entries;

use super::open_store;
use crate::format;

pub async fn run() -> Result<()> {
    let store = open_store().await?;
    let entries = Entries::new(store);

    let today = Local::now().date_naive();
    let start_local = Local
        .from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap())
        .unwrap();
    let end_local = start_local + chrono::Duration::days(1);
    let start_utc = start_local.with_timezone(&Utc).to_rfc3339();
    let end_utc = end_local.with_timezone(&Utc).to_rfc3339();

    let rows = entries.list_between(&start_utc, &end_utc).await?;
    if rows.is_empty() {
        println!("No entries today.");
        return Ok(());
    }
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
