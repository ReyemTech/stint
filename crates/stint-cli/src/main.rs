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
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start a new timer
    Start(cmd::start::Args),
    /// Stop the running timer
    Stop,
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
    /// Drain the sync queue once
    Sync,
    /// Pull running-timer and recent state from Solidtime
    Pull(cmd::pull::Args),
    /// Connect, list, and manage calendar accounts.
    #[command(subcommand)]
    Calendar(cmd::calendar::CalendarCmd),
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
    if !matches!(cli.command, Command::Sync | Command::Pull(_) | Command::Calendar(_)) {
        let store = cmd::open_store().await?;
        cmd::maybe_recover(&store).await?;
    }

    match cli.command {
        Command::Start(args) => cmd::start::run(args).await,
        Command::Stop => cmd::stop::run().await,
        Command::Today => cmd::today::run().await,
        Command::List(args) => cmd::list::run(args).await,
        Command::Edit(args) => cmd::edit::run(args).await,
        Command::Delete(args) => cmd::delete::run(args).await,
        Command::Config(c) => cmd::config::run(c).await,
        Command::Projects(p) => cmd::projects::run(p).await,
        Command::Sync => cmd::sync::run().await,
        Command::Pull(args) => cmd::pull::run(args).await,
        Command::Calendar(c) => {
            let store = cmd::open_store().await?;
            cmd::calendar::run(c, store).await
        }
    }
}
