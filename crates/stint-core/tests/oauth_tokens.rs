use chrono::{Duration, Utc};
use stint_core::oauth::tokens::TokenSet;

#[test]
fn from_response_computes_expires_at_from_expires_in() {
    let now = Utc::now();
    let t = TokenSet::from_response(
        "access-1".into(),
        Some("refresh-1".into()),
        3600, // expires_in seconds
        Some("read".into()),
        now,
    );
    assert_eq!(t.access_token, "access-1");
    assert_eq!(t.refresh_token.as_deref(), Some("refresh-1"));
    assert!(
        (t.expires_at - now - Duration::seconds(3600)).num_milliseconds().abs() < 10,
        "expires_at should be now + 3600s"
    );
    assert_eq!(t.scope.as_deref(), Some("read"));
}

#[test]
fn is_expired_with_skew_is_true_inside_safety_window() {
    let now = Utc::now();
    let t = TokenSet::from_response("a".into(), Some("r".into()), 30, None, now);
    // 30s expiry, default skew is 60s, so it's already "expired" for safety.
    assert!(t.is_expired_with_skew(now), "should be expired due to skew");
}

#[test]
fn is_expired_with_skew_is_false_when_plenty_of_time_left() {
    let now = Utc::now();
    let t = TokenSet::from_response("a".into(), Some("r".into()), 3600, None, now);
    assert!(!t.is_expired_with_skew(now));
}

#[test]
fn refresh_preserves_refresh_token_when_response_omits_it() {
    // Some providers (e.g., Solidtime/Passport) include refresh_token in every
    // response; others (e.g., Google) only on initial issue. If a refresh
    // response omits it, we MUST keep the existing one.
    let original = TokenSet::from_response("a1".into(), Some("r1".into()), 60, None, Utc::now());
    let merged = original.merge_refresh_response("a2".into(), None, 120, None);
    assert_eq!(merged.access_token, "a2");
    assert_eq!(merged.refresh_token.as_deref(), Some("r1"));
}

#[test]
fn refresh_overwrites_refresh_token_when_response_includes_one() {
    let original = TokenSet::from_response("a1".into(), Some("r1".into()), 60, None, Utc::now());
    let merged = original.merge_refresh_response("a2".into(), Some("r2".into()), 120, None);
    assert_eq!(merged.access_token, "a2");
    assert_eq!(merged.refresh_token.as_deref(), Some("r2"));
}
