//! Single JSON / human renderer for every CLI command.
//!
//! Commands call `render(&value, json, |v| println!(…))`. When `json` is true
//! the value is serialized with `serde_json::to_string_pretty`; otherwise the
//! human closure runs.

use serde::Serialize;

pub fn render<T: Serialize, F: FnOnce(&T)>(value: &T, json: bool, human: F) {
    if json {
        let out =
            serde_json::to_string_pretty(value).expect("serde::Serialize must produce valid JSON");
        println!("{out}");
    } else {
        human(value);
    }
}
