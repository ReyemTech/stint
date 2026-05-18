use chrono::{DateTime, Utc};

pub fn duration_hms(start: &str, end: Option<&str>) -> String {
    let s: DateTime<Utc> = match DateTime::parse_from_rfc3339(start) {
        Ok(d) => d.with_timezone(&Utc),
        Err(_) => return "??:??:??".into(),
    };
    let e: DateTime<Utc> = match end {
        Some(e) => match DateTime::parse_from_rfc3339(e) {
            Ok(d) => d.with_timezone(&Utc),
            Err(_) => return "??:??:??".into(),
        },
        None => Utc::now(),
    };
    let secs = (e - s).num_seconds().max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let ss = secs % 60;
    format!("{h:02}:{m:02}:{ss:02}")
}
