use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, NaiveTime, SecondsFormat, TimeZone, Utc};
use stint_core::store::entries::Entries;
use stint_core::timer::TimerService;

use super::open_store;

#[derive(clap::Args)]
pub struct Args {
    /// Entry UUID (or its 8-character prefix).
    pub id: String,
    /// New description.
    #[arg(long)]
    pub description: Option<String>,
    /// New start time, HH:MM (interpreted in local timezone, day = entry's existing date).
    #[arg(long)]
    pub start: Option<String>,
    /// New end time, HH:MM (interpreted in local timezone, day = entry's existing date).
    #[arg(long)]
    pub end: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let store = open_store().await?;
    let entries = Entries::new(store.clone());
    let timer = TimerService::new(store);

    let mut acted = false;

    if let Some(d) = args.description {
        timer.update_description(&args.id, &d).await?;
        println!("Updated description for {}.", &args.id);
        acted = true;
    }

    if args.start.is_some() || args.end.is_some() {
        let row = entries
            .get(&args.id)
            .await?
            .ok_or_else(|| anyhow!("entry {} not found", args.id))?;
        let existing_start = DateTime::parse_from_rfc3339(&row.start_at)?.with_timezone(&Utc);
        let existing_end_str = row
            .end_at
            .as_deref()
            .ok_or_else(|| anyhow!("cannot edit times on a running entry"))?;
        let existing_end = DateTime::parse_from_rfc3339(existing_end_str)?.with_timezone(&Utc);

        let new_start = match args.start.as_deref() {
            Some(hhmm) => combine_local_hhmm(existing_start, hhmm)?,
            None => existing_start,
        };
        let new_end = match args.end.as_deref() {
            Some(hhmm) => combine_local_hhmm(existing_end, hhmm)?,
            None => existing_end,
        };

        let start_str = new_start.to_rfc3339_opts(SecondsFormat::Secs, true);
        let end_str = new_end.to_rfc3339_opts(SecondsFormat::Secs, true);
        timer.update_times(&args.id, &start_str, &end_str).await?;
        println!("Updated times for {}.", &args.id);
        acted = true;
    }

    if !acted {
        println!(
            "Nothing to update. Pass --description / --start / --end to change something."
        );
    }
    Ok(())
}

fn combine_local_hhmm(reference_utc: DateTime<Utc>, hhmm: &str) -> Result<DateTime<Utc>> {
    let parsed = NaiveTime::parse_from_str(hhmm, "%H:%M")
        .map_err(|e| anyhow!("invalid HH:MM '{hhmm}': {e}"))?;
    let local_ref = reference_utc.with_timezone(&Local);
    let date = local_ref.date_naive();
    let local_dt = Local
        .from_local_datetime(&date.and_time(parsed))
        .single()
        .ok_or_else(|| anyhow!("ambiguous local time {hhmm} on {date}"))?;
    Ok(local_dt.with_timezone(&Utc))
}
