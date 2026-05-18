use crate::{
    store::{entries::Entries, running::RunningTimer, Store},
    time, Result,
};

#[derive(Debug)]
pub enum RecoveryDecision {
    KeepRunning,
    StopAtLastHeartbeat,
    Discard,
}

#[derive(Debug)]
pub struct StaleInfo {
    pub local_uuid: String,
    pub description: String,
    pub start_at: String,
    pub last_heartbeat_at: String,
    pub age_secs: i64,
}

#[derive(Debug)]
pub enum RecoveryOutcome {
    NothingToDo,
    AttachInPlace { local_uuid: String },
    Recovered { local_uuid: String },
    StoppedAtHeartbeat { local_uuid: String },
    Discarded { local_uuid: String },
}

pub async fn recover_on_startup<F>(store: &Store, prompt: F) -> Result<RecoveryOutcome>
where
    F: FnOnce(StaleInfo) -> RecoveryDecision,
{
    let running = RunningTimer::new(store.clone());
    let Some(r) = running.get().await? else {
        return Ok(RecoveryOutcome::NothingToDo);
    };

    let hb = time::parse(&r.heartbeat_at)?;
    let now = time::now();
    let age = (now - hb).num_seconds();

    if age <= 60 {
        return Ok(RecoveryOutcome::AttachInPlace {
            local_uuid: r.local_uuid,
        });
    }

    if age <= 600 {
        running.heartbeat().await?;
        return Ok(RecoveryOutcome::Recovered {
            local_uuid: r.local_uuid,
        });
    }

    let entries = Entries::new(store.clone());
    let row = entries.get(&r.local_uuid).await?;
    let info = StaleInfo {
        local_uuid: r.local_uuid.clone(),
        description: row.as_ref().map(|x| x.description.clone()).unwrap_or_default(),
        start_at: row.as_ref().map(|x| x.start_at.clone()).unwrap_or_default(),
        last_heartbeat_at: r.heartbeat_at.clone(),
        age_secs: age,
    };

    match prompt(info) {
        RecoveryDecision::KeepRunning => {
            running.heartbeat().await?;
            Ok(RecoveryOutcome::Recovered { local_uuid: r.local_uuid })
        }
        RecoveryDecision::StopAtLastHeartbeat => {
            entries.set_end(&r.local_uuid, &r.heartbeat_at).await?;
            running.clear().await?;
            Ok(RecoveryOutcome::StoppedAtHeartbeat { local_uuid: r.local_uuid })
        }
        RecoveryDecision::Discard => {
            entries.delete(&r.local_uuid).await?;
            running.clear().await?;
            Ok(RecoveryOutcome::Discarded { local_uuid: r.local_uuid })
        }
    }
}
