use chrono::{DateTime, SecondsFormat, Utc};

pub type Timestamp = DateTime<Utc>;

pub fn now() -> Timestamp {
    Utc::now()
}

pub fn now_utc() -> String {
    format(&now())
}

pub fn format(ts: &Timestamp) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn parse(s: &str) -> crate::Result<Timestamp> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| crate::Error::Invariant(format!("bad timestamp {s:?}: {e}")))
}
