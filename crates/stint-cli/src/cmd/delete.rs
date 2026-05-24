use anyhow::{anyhow, Result};
use stint_core::store::entries::Entries;
use stint_core::verbs;

use super::open_store;

#[derive(clap::Args)]
pub struct Args {
    /// Entry UUID (or its 8-character prefix).
    pub id: String,
}

pub async fn run(args: Args, json: bool) -> Result<()> {
    let store = open_store().await?;

    // `verbs::delete_entry` is idempotent (no-ops on missing rows). The CLI
    // contract is stricter — `stint delete <missing>` reports the row as
    // not found — so we probe before delegating.
    let entries = Entries::new(store.clone());
    if entries.get(&args.id).await?.is_none() {
        return Err(anyhow!("entry {} not found", args.id));
    }

    verbs::delete_entry(&store, &args.id).await?;

    // No structured payload — `delete` returns unit. Emit a JSON ack for
    // consistency, otherwise the legacy human line. The ack carries both
    // the bool (so callers can `.deleted == true`-check) and the id (so
    // they can verify which row was acted on).
    let ack = serde_json::json!({ "deleted": true, "id": args.id });
    crate::render::render(&ack, json, |_| println!("Deleted {}.", args.id));
    Ok(())
}
