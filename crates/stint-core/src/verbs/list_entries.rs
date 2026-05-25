use crate::store::entries::Entries;
use crate::store::Store;
use crate::verbs::types::{EntryFilter, EntryView};
use crate::Result;

/// Sentinel bounds used when the caller does not constrain `since`/`until`.
/// `list_between` requires both ends; these widen the window to "any entry".
const MIN_TS: &str = "1970-01-01T00:00:00Z";
const MAX_TS: &str = "9999-01-01T00:00:00Z";

/// List entries matching `filter`. Filtering by `project_id` and truncation
/// to `limit` happen in-memory after the SQL window query — fine for the
/// expected scale (one user's local history). If the row count ever grows
/// past tens of thousands, push these into a `Entries::list_filtered` SQL.
pub async fn list_entries(store: &Store, filter: EntryFilter) -> Result<Vec<EntryView>> {
    let entries = Entries::new(store.clone());
    let since = filter.since.as_deref().unwrap_or(MIN_TS);
    let until = filter.until.as_deref().unwrap_or(MAX_TS);
    let mut rows = entries.list_between(since, until).await?;

    if let Some(pid) = filter.project_id.as_deref() {
        rows.retain(|r| r.project_id.as_deref() == Some(pid));
    }
    if let Some(limit) = filter.limit {
        // `Entries::list_between` returns rows ASC by `start_at`, so
        // `truncate(limit)` keeps the *oldest* N within the window.
        // Callers that want the newest N should reverse before truncating
        // or migrate to a SQL-pushdown filter.
        rows.truncate(limit as usize);
    }

    Ok(rows.into_iter().map(EntryView::from).collect())
}
