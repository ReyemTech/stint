//! Interactive harness picker for `stint skill install` with no `<harness>` arg.

use crate::skill::{all_harnesses, harness::Harness};
use anyhow::{anyhow, Result};

pub fn pick() -> Result<Box<dyn Harness>> {
    let harnesses = all_harnesses();
    let options: Vec<String> = harnesses
        .iter()
        .map(|h| {
            let marker = if h.detect() { "" } else { " (not detected)" };
            format!("{}{marker}", h.display())
        })
        .collect();
    let idx = dialoguer::Select::new()
        .with_prompt("Pick a harness")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|e| anyhow!("picker cancelled: {e}"))?;
    Ok(harnesses.into_iter().nth(idx).unwrap())
}
