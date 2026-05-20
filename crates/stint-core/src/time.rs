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

/// Normalize any RFC 3339 timestamp to Solidtime's required form: UTC with a
/// literal `Z` suffix and second precision. Solidtime's API validates inputs
/// against `Y-m-d\TH:i:s\Z` and 422s on offset form (e.g. `-04:00`). All-day
/// dates (`YYYY-MM-DD`) and other unparseable inputs pass through unchanged.
pub fn to_solidtime_z(ts: &str) -> String {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| format(&dt.with_timezone(&Utc)))
        .unwrap_or_else(|_| ts.to_string())
}
