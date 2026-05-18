use crate::{
    store::{
        entries::{Entries, NewTimeEntry},
        queue::{Queue, QueueOp},
        running::RunningTimer,
        Store,
    },
    time, Error, Result,
};
use serde::Serialize;

pub struct TimerService {
    store: Store,
}

#[derive(Debug, Clone)]
pub struct StartArgs {
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub source: String,
}

#[derive(Serialize)]
struct CreatePayload<'a> {
    local_uuid: &'a str,
    description: &'a str,
    project_id: Option<&'a str>,
    task_id: Option<&'a str>,
    start_at: &'a str,
    billable: bool,
}

impl TimerService {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn start(&self, args: StartArgs) -> Result<String> {
        let running = RunningTimer::new(self.store.clone());
        if running.get().await?.is_some() {
            return Err(Error::Invariant(
                "a timer is already running; stop it first".into(),
            ));
        }

        let entries = Entries::new(self.store.clone());
        let queue = Queue::new(self.store.clone());

        let start_at = time::now_utc();
        let local_uuid = entries
            .create(NewTimeEntry {
                description: args.description.clone(),
                project_id: args.project_id.clone(),
                task_id: args.task_id.clone(),
                start_at: start_at.clone(),
                billable: false,
                source: args.source.clone(),
            })
            .await?;

        running.set(&local_uuid).await?;

        let payload = serde_json::to_string(&CreatePayload {
            local_uuid: &local_uuid,
            description: &args.description,
            project_id: args.project_id.as_deref(),
            task_id: args.task_id.as_deref(),
            start_at: &start_at,
            billable: false,
        })?;
        queue
            .enqueue(QueueOp::CreateEntry, &payload, Some(&local_uuid))
            .await?;

        Ok(local_uuid)
    }

    pub async fn stop(&self) -> Result<String> {
        let running = RunningTimer::new(self.store.clone());
        let r = running
            .get()
            .await?
            .ok_or_else(|| Error::Invariant("no timer running".into()))?;
        let local_uuid = r.local_uuid;

        let entries = Entries::new(self.store.clone());
        let queue = Queue::new(self.store.clone());

        let end_at = time::now_utc();
        entries.set_end(&local_uuid, &end_at).await?;
        running.clear().await?;

        // If the entry is still pending_create, no extra queue op is needed —
        // the create payload will be regenerated from current state at push time.
        // If it's already synced (re-edited), enqueue an update.
        let state = entries.get(&local_uuid).await?.unwrap().sync_state;
        if state == "dirty" {
            let row = entries.get(&local_uuid).await?.unwrap();
            let payload = serde_json::to_string(&row)?;
            queue
                .enqueue(QueueOp::UpdateEntry, &payload, Some(&local_uuid))
                .await?;
        }

        Ok(local_uuid)
    }
}
