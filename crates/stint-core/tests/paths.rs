use stint_core::paths;

#[test]
fn data_dir_on_macos_is_application_support_stint() {
    let dir = paths::data_dir().unwrap();
    let s = dir.to_string_lossy();
    assert!(
        s.ends_with("Application Support/stint"),
        "expected Application Support/stint suffix, got {s}"
    );
}

#[test]
fn database_path_is_inside_data_dir() {
    let db = paths::database_path().unwrap();
    let parent = db.parent().unwrap();
    assert_eq!(parent, paths::data_dir().unwrap());
    assert_eq!(db.file_name().unwrap(), "stint.db");
}
