// Inject Info.plist into the binary's __TEXT,__info_plist Mach-O section so
// macOS hardened runtime accepts the binary when embedded in Stint.app —
// without this, taskgated kills the CLI on launch with "Invalid Signature".
#[cfg(target_os = "macos")]
embed_plist::embed_info_plist!("../Info.plist");

mod at_parse;
mod cmd;
mod format;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "stint",
    version,
    about = "Time tracker that syncs with Solidtime"
)]
struct Cli {
    /// Emit machine-readable JSON instead of human text.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start a new timer
    Start(cmd::start::Args),
    /// Stop the running timer
    Stop,
    /// Start a new timer using an existing entry's description/project/billable
    Restart(cmd::restart::Args),
    /// Show today's entries
    Today,
    /// List entries between two dates
    List(cmd::list::Args),
    /// Edit an entry
    Edit(cmd::edit::Args),
    /// Delete an entry
    Delete(cmd::delete::Args),
    /// View and modify configuration
    #[command(subcommand)]
    Config(cmd::config::ConfigCmd),
    /// Refresh and list projects/tasks/tags
    #[command(subcommand)]
    Projects(cmd::projects::ProjectsCmd),
    /// Sync queue commands (drain, retry-abandoned)
    Sync(cmd::sync::SyncArgs),
    /// Pull running-timer and recent state from Solidtime
    Pull(cmd::pull::Args),
    /// Connect, list, and manage calendar accounts.
    #[command(subcommand)]
    Calendar(cmd::calendar::CalendarCmd),
    /// Check for and apply updates to the standalone CLI. No-op for .app-bundled installs.
    Update {
        /// Print available version without applying.
        #[arg(long)]
        check: bool,
        /// Apply even if already on latest.
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("STINT_LOG")
                .unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    // Run startup recovery once, before any command. Skip for `Sync` to avoid
    // recursion when the GUI is also running concurrently. Skip for `Calendar`
    // because calendar commands open their own store and recovery is irrelevant.
    if !matches!(
        cli.command,
        Command::Sync(_) | Command::Pull(_) | Command::Calendar(_) | Command::Update { .. }
    ) {
        let store = cmd::open_store().await?;
        cmd::maybe_recover(&store).await?;
    }

    let json = cli.json;
    match cli.command {
        Command::Start(args) => cmd::start::run(args, json).await,
        Command::Stop => cmd::stop::run(json).await,
        Command::Restart(args) => cmd::restart::run(args, json).await,
        Command::Today => cmd::today::run(json).await,
        Command::List(args) => cmd::list::run(args, json).await,
        Command::Edit(args) => cmd::edit::run(args, json).await,
        Command::Delete(args) => cmd::delete::run(args, json).await,
        Command::Config(c) => cmd::config::run(c).await,
        Command::Projects(p) => cmd::projects::run(p, json).await,
        Command::Sync(args) => cmd::sync::run(args).await,
        Command::Pull(args) => cmd::pull::run(args).await,
        Command::Calendar(c) => {
            let store = cmd::open_store().await?;
            cmd::calendar::run(c, store).await
        }
        Command::Update { check, force } => {
            tokio::task::spawn_blocking(move || cmd::update::run(check, force)).await?
        }
    }
}
