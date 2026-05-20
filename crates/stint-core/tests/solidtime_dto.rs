use stint_core::solidtime::dto::RemoteTimeEntry;

#[test]
fn deserializes_active_entry_with_null_end_and_updated_at() {
    let json = r#"{
        "id": "remote-1",
        "description": "writing tests",
        "project_id": null,
        "task_id": null,
        "start": "2026-05-20T17:00:00Z",
        "end": null,
        "billable": false,
        "updated_at": "2026-05-20T17:01:00Z"
    }"#;
    let e: RemoteTimeEntry = serde_json::from_str(json).unwrap();
    assert_eq!(e.id, "remote-1");
    assert!(e.end.is_none());
    assert_eq!(e.updated_at.as_deref(), Some("2026-05-20T17:01:00Z"));
}

#[test]
fn deserializes_completed_entry_without_updated_at_field() {
    let json = r#"{
        "id": "remote-2",
        "description": "done",
        "start": "2026-05-20T10:00:00Z",
        "end": "2026-05-20T11:00:00Z",
        "billable": true
    }"#;
    let e: RemoteTimeEntry = serde_json::from_str(json).unwrap();
    assert_eq!(e.end.as_deref(), Some("2026-05-20T11:00:00Z"));
    assert!(e.updated_at.is_none());
}
