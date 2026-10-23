use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    Dia,
    Chrome,
    Edge,
}

impl Browser {
    pub const ALL: [Self; 3] = [Self::Dia, Self::Chrome, Self::Edge];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dia => "dia",
            Self::Chrome => "chrome",
            Self::Edge => "edge",
        }
    }
}

#[derive(Debug, Parser)]
#[command(version, about = env!("CARGO_PKG_DESCRIPTION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Configure the extension host and background sync for a browser.
    Setup {
        /// Primary browser to connect (defaults to Dia).
        #[arg(long, value_enum, default_value_t = Browser::Dia, conflicts_with = "all")]
        browser: Browser,
        /// Configure Dia, Google Chrome, and Microsoft Edge together.
        #[arg(long)]
        all: bool,
        /// Register the browser without enabling the 3-hour macOS schedule.
        #[arg(long)]
        no_schedule: bool,
        /// Use a signed sandboxed Safari helper with a saved folder bookmark instead of Full Disk Access.
        #[arg(long, conflicts_with = "allow_full_disk_access")]
        scoped_helper: Option<PathBuf>,
        /// Explicitly opt in to the old direct SQLite backend requiring macOS Full Disk Access.
        #[arg(long)]
        allow_full_disk_access: bool,
    },
    /// Import new history in both directions (Safari opens hidden if needed).
    Sync {
        /// Show proposed changes without writing browser databases or launching Safari.
        #[arg(long)]
        dry_run: bool,
        /// Maximum URLs to transfer per direction this run (1-1000).
        #[arg(long, default_value_t = 1000)]
        limit: usize,
    },
    /// Inspect integration health, registered browsers, and pending transfers.
    Status,
    /// Remove background sync and native host registrations (never deletes history).
    Uninstall,
    /// Native Messaging host for the Chromium extension (browser-managed process).
    #[command(hide = true)]
    Host {
        /// Dedicated browser registrations verify the claimed browser;
        /// shared Chromium host locations rely on the browser in the request.
        #[arg(long, value_enum)]
        browser: Option<Browser>,
        #[arg(hide = true, trailing_var_arg = true, allow_hyphen_values = true)]
        browser_arguments: Vec<String>,
    },
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
