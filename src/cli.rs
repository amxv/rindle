use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about = env!("CARGO_PKG_DESCRIPTION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Enable, disable, or inspect daily automatic upgrades.
    AutoUpdate {
        #[command(subcommand)]
        action: AutoUpdateAction,
    },
    /// Install the latest release or check for updates.
    Upgrade {
        #[arg(long)]
        check: bool,
        /// Reserved for the daily system scheduler.
        #[arg(long, hide = true, conflicts_with = "check")]
        automatic: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum AutoUpdateAction {
    /// Enable daily update checks and installation.
    On,
    /// Disable daily automatic updates.
    Off,
    /// Show the saved setting and last check time.
    Status,
}
