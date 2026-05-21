use std::sync::{Mutex, OnceLock};

use stint_core::paths;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvRestore {
    previous: Option<std::ffi::OsString>,
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var("STINT_DATA_DIR", value),
            None => std::env::remove_var("STINT_DATA_DIR"),
        }
    }
}

fn with_stint_data_dir_override<T>(value: Option<&std::path::Path>, f: impl FnOnce() -> T) -> T {
    let _guard = env_lock().lock().unwrap();
    let restore = EnvRestore {
        previous: std::env::var_os("STINT_DATA_DIR"),
    };

    match value {
        Some(path) => std::env::set_var("STINT_DATA_DIR", path),
        None => std::env::remove_var("STINT_DATA_DIR"),
    }

    let result = f();
    drop(restore);
    result
}

#[test]
fn data_dir_on_macos_is_application_support_stint() {
    with_stint_data_dir_override(None, || {
        let dir = paths::data_dir().unwrap();
        let s = dir.to_string_lossy();
        assert!(
            s.ends_with("Application Support/stint"),
            "expected Application Support/stint suffix, got {s}"
        );
    });
}

#[test]
fn database_path_is_inside_data_dir() {
    with_stint_data_dir_override(None, || {
        let db = paths::database_path().unwrap();
        let parent = db.parent().unwrap();
        assert_eq!(parent, paths::data_dir().unwrap());
        assert_eq!(db.file_name().unwrap(), "stint.db");
    });
}

#[test]
fn data_dir_uses_env_override_when_present() {
    let override_dir = std::env::temp_dir().join("stint-paths-test-override");

    with_stint_data_dir_override(Some(&override_dir), || {
        assert_eq!(paths::data_dir().unwrap(), override_dir);
    });
}

#[test]
fn database_path_uses_env_override_when_present() {
    let override_dir = std::env::temp_dir().join("stint-paths-test-db-override");

    with_stint_data_dir_override(Some(&override_dir), || {
        assert_eq!(
            paths::database_path().unwrap(),
            override_dir.join("stint.db")
        );
    });
}

#[test]
fn empty_env_override_falls_back_to_default_data_dir() {
    let _guard = env_lock().lock().unwrap();
    let restore = EnvRestore {
        previous: std::env::var_os("STINT_DATA_DIR"),
    };

    std::env::set_var("STINT_DATA_DIR", "");

    let dir = paths::data_dir().unwrap();
    let s = dir.to_string_lossy();
    assert!(
        s.ends_with("Application Support/stint"),
        "expected Application Support/stint suffix, got {s}"
    );

    drop(restore);
}

#[test]
fn ensure_data_dir_returns_io_error_when_override_points_to_file() {
    let temp_root = std::env::temp_dir().join(format!(
        "stint-paths-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_root).unwrap();

    let file_path = temp_root.join("not-a-directory");
    std::fs::write(&file_path, "occupied").unwrap();

    with_stint_data_dir_override(Some(&file_path), || {
        let err = paths::ensure_data_dir().unwrap_err();
        assert!(
            matches!(err, stint_core::Error::Io(_)),
            "unexpected error: {err}"
        );
        let io = match err {
            stint_core::Error::Io(err) => err,
            other => panic!("unexpected error: {other}"),
        };
        assert_eq!(io.kind(), std::io::ErrorKind::AlreadyExists);
    });

    std::fs::remove_file(&file_path).unwrap();
    std::fs::remove_dir(&temp_root).unwrap();
}
