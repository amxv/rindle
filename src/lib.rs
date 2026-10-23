pub mod cli;
pub mod commands;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

pub fn run() -> Result<()> {
    run_with(Cli::parse())
}

pub fn run_with(cli: Cli) -> Result<()> {
    match cli.command {
        Command::AutoUpdate { action } => commands::auto_update::run(action),
        Command::Upgrade { check, automatic } => commands::upgrade::run(check, automatic),
    }
}
