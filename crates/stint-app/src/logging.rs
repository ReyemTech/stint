use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{prelude::*, EnvFilter};

/// Path to the rolling log directory on macOS:
/// `~/Library/Logs/tech.reyem.stint/`. Files rotate daily with prefix `stint`
/// and suffix `log` — e.g. `stint.log.2026-05-22`.
fn log_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("Library/Logs/tech.reyem.stint"))
}

/// Initialize tracing with both stderr and a rolling daily file appender.
///
/// stderr stays so `scripts/dev-app.sh` runs still see logs inline. The file
/// layer is what gives GUI-launched users (whose stderr goes nowhere) a
/// diagnostic trail when something goes silently wrong — e.g. the updater
/// install path that previously had no observable failure mode.
///
/// Returns the non-blocking worker guard which must outlive the program;
/// callers should bind it to a variable that lives until shutdown.
pub fn init() -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_env("STINT_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);

    let (file_layer, guard) = match log_dir() {
        Some(dir) if std::fs::create_dir_all(&dir).is_ok() => {
            let appender = tracing_appender::rolling::Builder::new()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("stint")
                .filename_suffix("log")
                .max_log_files(14)
                .build(&dir)
                .ok();
            match appender {
                Some(a) => {
                    let (nb, guard) = tracing_appender::non_blocking(a);
                    let layer = tracing_subscriber::fmt::layer()
                        .with_writer(nb)
                        .with_ansi(false);
                    (Some(layer), Some(guard))
                }
                None => (None, None),
            }
        }
        _ => (None, None),
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer)
        .with(file_layer)
        .init();

    if let Some(dir) = log_dir() {
        tracing::info!(path = %dir.display(), "log file directory");
    }

    guard
}
