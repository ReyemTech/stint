//! Parse `stint://` URLs into a typed `Action`.
//!
//! Supported forms (Phase 6a):
//! - `stint://start?description=…&project=…&task=…&billable=true`
//! - `stint://stop`
//! - `stint://entry/<local_uuid>` (open in app)
//! - `stint://current` (focus current entry view)

use crate::{Error, Result};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Action {
    Start {
        description: String,
        project_id: Option<String>,
        task_id: Option<String>,
        billable: bool,
    },
    Stop,
    OpenEntry {
        local_uuid: String,
    },
    Current,
}

pub fn parse(input: &str) -> Result<Action> {
    let rest = input
        .strip_prefix("stint://")
        .ok_or_else(|| Error::Invariant(format!("not a stint url: {input}")))?;

    let (path, query) = match rest.find('?') {
        Some(i) => (&rest[..i], Some(&rest[i + 1..])),
        None => (rest, None),
    };

    let params = query.map(parse_query).unwrap_or_default();

    let mut segments = path.split('/');
    let head = segments.next().unwrap_or("");

    match head {
        "start" => {
            let description = params
                .get("description")
                .cloned()
                .ok_or_else(|| Error::Invariant("start requires description".into()))?;
            Ok(Action::Start {
                description,
                project_id: params.get("project").cloned(),
                task_id: params.get("task").cloned(),
                billable: params.get("billable").map(|v| v == "true").unwrap_or(false),
            })
        }
        "stop" => Ok(Action::Stop),
        "current" => Ok(Action::Current),
        "entry" => {
            let local_uuid = segments
                .next()
                .ok_or_else(|| Error::Invariant("entry requires local_uuid".into()))?
                .to_string();
            Ok(Action::OpenEntry { local_uuid })
        }
        other => Err(Error::Invariant(format!("unknown stint action: {other}"))),
    }
}

fn parse_query(q: &str) -> HashMap<String, String> {
    q.split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            Some((k.to_string(), percent_decode(v)))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(byte as char);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(' ');
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
}
