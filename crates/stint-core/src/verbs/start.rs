use crate::config::Settings;
use crate::store::entries::Entries;
use crate::store::Store;
use crate::timer::{StartArgs, TimerService};
use crate::verbs::types::{EntryView, StartParams};
use crate::Result;

/// Start a new running entry. Stopping any in-progress timer first is the
/// caller's responsibility — this verb is strict and returns an error if
/// a timer is already running. (Restart-style behavior lives in a separate
/// helper at the transport layer.)
///
/// **Focus default fallback:** when `params.project_id` is `None`, this
/// looks up `focus.default_project` in settings — written by Swift's
/// `ProjectFocusFilter` when a macOS Focus filter activates. The stored
/// value is `"<focus_id>\t<project_id>"`; the project_id is only applied
/// when the stored focus_id matches the currently-active focus (so a stale
/// default from a previous focus doesn't leak across focus mode changes).
pub async fn start(store: &Store, params: StartParams) -> Result<EntryView> {
    let project_id = match params.project_id.clone() {
        Some(id) => Some(id),
        None => resolve_focus_default(store).await,
    };

    let timer = TimerService::new(store.clone());
    let id = timer
        .start(StartArgs {
            description: params.description,
            project_id,
            task_id: params.task_id,
            billable: params.billable,
            source: params.source,
            start_at: params.start_at,
        })
        .await?;

    let entries = Entries::new(store.clone());
    let row = entries
        .get(&id)
        .await?
        .expect("just-inserted entry must exist");

    let view: EntryView = row.into();
    if let Ok(payload) = serde_json::to_string(&view) {
        crate::ffi::notify_indexer(crate::ffi::IndexerKind::EntryStarted, &payload);
    }
    Ok(view)
}

async fn resolve_focus_default(store: &Store) -> Option<String> {
    let settings = Settings::new(store.clone());
    let raw = settings.get("focus.default_project").await.ok().flatten()?;
    let (stored_focus, project_id) = raw.split_once('\t')?;
    let current = crate::focus::current_id()?;
    if current == stored_focus {
        Some(project_id.to_string())
    } else {
        None
    }
}
