//! Parses the `--at` argument for `stint start`. Accepts:
//!   - relative ago: "5min ago", "30 min ago", "1h ago", "1hr ago", "1 hour ago"
//!   - bare HH:MM (interpreted as today local time, day-shifted to yesterday
//!     if the resulting moment would be in the future)
//!   - RFC 3339 absolute timestamp
//! Returns a UTC RFC 3339 string at second precision, suitable for stint-core.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Local, NaiveTime, SecondsFormat, Utc};

pub fn parse_at_arg(input: &str) -> Result<String> {
    let s = input.trim();
    if s.is_empty() {
        return Err(anyhow!("--at value is empty"));
    }

    // 1. Absolute RFC 3339?
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true));
    }

    // 2. Bare HH:MM today (local)?
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M") {
        let local_now = Local::now();
        let candidate = local_now
            .with_time(t)
            .single()
            .ok_or_else(|| anyhow!("ambiguous local time {s}"))?;
        let resolved = if candidate > local_now {
            candidate - Duration::days(1)
        } else {
            candidate
        };
        return Ok(resolved
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true));
    }

    // 3. Relative "<n><unit> ago"
    let lower = s.to_ascii_lowercase();
    let stripped = lower
        .strip_suffix(" ago")
        .or_else(|| lower.strip_suffix("ago"))
        .map(str::trim)
        .ok_or_else(|| anyhow!("could not parse '{s}'; try '15 min ago' or '09:30'"))?;
    let (num_str, unit_str) = split_num_unit(stripped)?;
    let n: i64 = num_str
        .parse()
        .map_err(|e| anyhow!("bad number '{num_str}': {e}"))?;
    let dur = match unit_str {
        "m" | "min" | "mins" | "minute" | "minutes" => Duration::minutes(n),
        "h" | "hr" | "hrs" | "hour" | "hours" => Duration::hours(n),
        other => return Err(anyhow!("unknown unit '{other}' (try min or hour)")),
    };
    let when = Utc::now() - dur;
    Ok(when.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn split_num_unit(s: &str) -> Result<(&str, &str)> {
    let s = s.trim();
    let idx = s
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| anyhow!("missing unit after number in '{s}'"))?;
    let (num, rest) = s.split_at(idx);
    let unit = rest.trim();
    Ok((num, unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_5min_ago() {
        let out = parse_at_arg("5min ago").unwrap();
        let parsed = DateTime::parse_from_rfc3339(&out).unwrap();
        let diff = Utc::now()
            .signed_duration_since(parsed.with_timezone(&Utc))
            .num_seconds();
        assert!((290..=310).contains(&diff), "expected ~300s, got {diff}");
    }

    #[test]
    fn parses_30_min_ago() {
        parse_at_arg("30 min ago").unwrap();
    }

    #[test]
    fn parses_1h_ago() {
        let out = parse_at_arg("1h ago").unwrap();
        let parsed = DateTime::parse_from_rfc3339(&out).unwrap();
        let diff = Utc::now()
            .signed_duration_since(parsed.with_timezone(&Utc))
            .num_seconds();
        assert!((3590..=3610).contains(&diff));
    }

    #[test]
    fn parses_1hr_ago() {
        parse_at_arg("1hr ago").unwrap();
    }

    #[test]
    fn parses_1_hour_ago() {
        parse_at_arg("1 hour ago").unwrap();
    }

    #[test]
    fn parses_rfc3339() {
        let out = parse_at_arg("2026-05-20T09:00:00Z").unwrap();
        assert_eq!(out, "2026-05-20T09:00:00Z");
    }

    #[test]
    fn parses_hhmm() {
        let out = parse_at_arg("09:30").unwrap();
        DateTime::parse_from_rfc3339(&out).unwrap();
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_at_arg("yesterday").is_err());
        assert!(parse_at_arg("").is_err());
        assert!(parse_at_arg("5xyz ago").is_err());
    }
}
