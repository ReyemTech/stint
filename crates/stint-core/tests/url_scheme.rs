use stint_core::url_scheme::{parse, Action};

#[test]
fn parse_start_with_description() {
    let action = parse("stint://start?description=hello%20world").unwrap();
    match action {
        Action::Start { description, .. } => assert_eq!(description, "hello world"),
        _ => panic!("expected Start"),
    }
}

#[test]
fn parse_start_with_project() {
    let action = parse("stint://start?description=x&project=p-1").unwrap();
    if let Action::Start { project_id, .. } = action {
        assert_eq!(project_id.as_deref(), Some("p-1"));
    } else {
        panic!("expected Start");
    }
}

#[test]
fn parse_start_with_billable() {
    let action = parse("stint://start?description=x&billable=true").unwrap();
    if let Action::Start { billable, .. } = action {
        assert!(billable);
    } else {
        panic!("expected Start");
    }
}

#[test]
fn parse_stop() {
    assert!(matches!(parse("stint://stop").unwrap(), Action::Stop));
}

#[test]
fn parse_current() {
    assert!(matches!(parse("stint://current").unwrap(), Action::Current));
}

#[test]
fn parse_open_entry() {
    let action = parse("stint://entry/abc-123").unwrap();
    assert!(matches!(action, Action::OpenEntry { local_uuid } if local_uuid == "abc-123"));
}

#[test]
fn parse_unknown_returns_err() {
    assert!(parse("stint://nope").is_err());
}

#[test]
fn parse_wrong_scheme_returns_err() {
    assert!(parse("https://example.com").is_err());
}

#[test]
fn parse_start_without_description_errors() {
    assert!(parse("stint://start").is_err());
}

#[test]
fn parse_percent_decodes_special_chars() {
    let action = parse("stint://start?description=a%2Bb%20c%26d").unwrap();
    if let Action::Start { description, .. } = action {
        assert_eq!(description, "a+b c&d");
    }
}

#[test]
fn parse_open_project() {
    let action = parse("stint://project/proj-uuid-1").unwrap();
    assert!(matches!(action, Action::OpenProject { project_id } if project_id == "proj-uuid-1"));
}

#[test]
fn parse_open_task() {
    let action = parse("stint://task/task-uuid-1").unwrap();
    assert!(matches!(action, Action::OpenTask { task_id } if task_id == "task-uuid-1"));
}

#[test]
fn parse_open_project_missing_id_errors() {
    assert!(parse("stint://project").is_err());
    assert!(parse("stint://project/").is_err());
}

#[test]
fn parse_open_task_missing_id_errors() {
    assert!(parse("stint://task").is_err());
    assert!(parse("stint://task/").is_err());
}

#[test]
fn parse_percent_decodes_multibyte_utf8() {
    // `café` in UTF-8 is c, a, f, é → 63 61 66 c3 a9. The é must round-trip
    // as a single grapheme, not two corrupted chars. Same for emoji (🎯 →
    // f0 9f 8e af = 4 bytes).
    let action = parse("stint://start?description=caf%C3%A9%20%F0%9F%8E%AF").unwrap();
    if let Action::Start { description, .. } = action {
        assert_eq!(description, "café 🎯");
    } else {
        panic!("expected Start");
    }
}
