pub mod cli;
pub mod commands;
pub mod native;
pub mod state;
pub mod urls;

#[cfg(target_os = "macos")]
pub mod macos;

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
        Command::Setup {
            browser,
            all,
            no_schedule,
            scoped_helper,
            allow_full_disk_access,
        } => platform_setup(
            browser,
            all,
            no_schedule,
            scoped_helper,
            allow_full_disk_access,
        ),
        Command::Sync { dry_run, limit } => platform_sync(dry_run, limit),
        Command::Status => platform_status(),
        Command::Uninstall => platform_uninstall(),
        Command::Host {
            browser,
            browser_arguments,
        } => native::serve(browser, &browser_arguments),
    }
}

fn platform_setup(
    browser: cli::Browser,
    all: bool,
    no_schedule: bool,
    scoped_helper: Option<std::path::PathBuf>,
    allow_full_disk_access: bool,
) -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::setup::run(
        browser,
        all,
        no_schedule,
        scoped_helper,
        allow_full_disk_access,
    );
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (
            browser,
            all,
            no_schedule,
            scoped_helper,
            allow_full_disk_access,
        );
        anyhow::bail!("Rindle history sync currently requires macOS")
    }
}

fn platform_sync(dry_run: bool, limit: usize) -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::sync::run(dry_run, limit);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (dry_run, limit);
        anyhow::bail!("Rindle history sync currently requires macOS")
    }
}

fn platform_status() -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::setup::status();
    #[cfg(not(target_os = "macos"))]
    anyhow::bail!("Rindle history sync currently requires macOS")
}

fn platform_uninstall() -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::setup::uninstall();
    #[cfg(not(target_os = "macos"))]
    anyhow::bail!("Rindle history sync currently requires macOS")
}
