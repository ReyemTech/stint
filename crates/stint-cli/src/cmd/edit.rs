use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, NaiveTime, SecondsFormat, TimeZone, Utc};
use stint_core::store::entries::Entries;
use stint_core::verbs::{self, EntryPatch};

use super::open_store;

/// Clap can't natively express "clear vs unchanged" for an `Option<Option<T>>`
/// field, so the CLI exposes two flags per nullable field:
///   * `--project ID` — set the project to `ID`
///   * `--clear-project` — explicitly clear the project
/// Absent both flags = no change. The same shape applies to task. The two
/// flags are mutually exclusive — passing both is rejected at parse time.
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
    /// Set the project (Solidtime UUID). Mutually exclusive with --clear-project.
    #[arg(long, conflicts_with = "clear_project")]
    pub project: Option<String>,
    /// Clear the project association. Mutually exclusive with --project.
    #[arg(long)]
    pub clear_project: bool,
    /// Set the task (Solidtime UUID). Mutually exclusive with --clear-task.
    #[arg(long, conflicts_with = "clear_task")]
    pub task: Option<String>,
    /// Clear the task association. Mutually exclusive with --task.
    #[arg(long)]
    pub clear_task: bool,
    /// Set the billable flag (true/false).
    #[arg(long)]
    pub billable: Option<bool>,
}

pub async fn run(args: Args, json: bool) -> Result<()> {
    let store = open_store().await?;
    let entries = Entries::new(store.clone());

    // Materialize HH:MM into RFC 3339 against the entry's existing date. We
    // need to read the row up-front anyway to apply the date; the verb does
    // its own existence check afterwards.
    let mut start_at: Option<String> = None;
    let mut end_at_set: Option<String> = None;
    let times_change = args.start.is_some() || args.end.is_some();
    if times_change {
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
        start_at = Some(new_start.to_rfc3339_opts(SecondsFormat::Secs, true));
        end_at_set = Some(new_end.to_rfc3339_opts(SecondsFormat::Secs, true));
    }

    let project_id = match (args.project.as_deref(), args.clear_project) {
        (Some(v), false) => Some(Some(v.to_string())),
        (None, true) => Some(None),
        _ => None,
    };
    let task_id = match (args.task.as_deref(), args.clear_task) {
        (Some(v), false) => Some(Some(v.to_string())),
        (None, true) => Some(None),
        _ => None,
    };

    let description_change = args.description.is_some();
    let metadata_change = project_id.is_some() || task_id.is_some() || args.billable.is_some();
    let acted = description_change || times_change || metadata_change;

    if !acted {
        if json {
            // No-op — emit an empty object so JSON consumers can detect.
            crate::render::render(&serde_json::json!({}), true, |_| {});
        } else {
            println!(
                "Nothing to update. Pass --description / --start / --end to change something."
            );
        }
        return Ok(());
    }

    let patch = EntryPatch {
        description: args.description,
        project_id,
        task_id,
        billable: args.billable,
        start_at,
        end_at: end_at_set.map(Some),
    };

    let view = verbs::update_entry(&store, &args.id, patch).await?;

    crate::render::render(&view, json, |_v| {
        if description_change {
            println!("Updated description for {}.", &args.id);
        }
        if times_change {
            println!("Updated times for {}.", &args.id);
        }
        if metadata_change && !description_change && !times_change {
            println!("Updated {}.", &args.id);
        }
    });
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
