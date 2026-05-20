use stint_core::calendar::google::resolve_account_identifier;
use stint_core::calendar::provider::RemoteCalendar;

#[test]
fn resolve_identifier_prefers_primary_calendar() {
    let cals = vec![
        RemoteCalendar {
            id: "en.canadian#holiday@group.v.calendar.google.com".into(),
            name: "Holidays in Canada".into(),
            color: None,
            primary: false,
        },
        RemoteCalendar {
            id: "me@example.com".into(),
            name: "me@example.com".into(),
            color: None,
            primary: true,
        },
    ];
    assert_eq!(
        resolve_account_identifier(&cals, "fallback-uuid"),
        "me@example.com"
    );
}

#[test]
fn resolve_identifier_falls_back_when_no_primary() {
    let cals = vec![RemoteCalendar {
        id: "abc@example.com".into(),
        name: "Inbox".into(),
        color: None,
        primary: false,
    }];
    // Name takes precedence over id when both are present.
    assert_eq!(resolve_account_identifier(&cals, "fallback"), "Inbox");
}

#[test]
fn resolve_identifier_falls_back_to_default_when_empty() {
    assert_eq!(
        resolve_account_identifier(&[], "fallback-uuid"),
        "fallback-uuid"
    );
}
