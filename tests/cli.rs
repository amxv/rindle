use assert_cmd::Command;
use predicates::prelude::*;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rindle"))
}

#[test]
fn shows_help() {
    cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("upgrade"))
        .stdout(predicate::str::contains("auto-update"))
        .stdout(predicate::str::contains("hello").not());
}

#[test]
fn shows_version() {
    cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn describes_upgrade() {
    cli()
        .args(["upgrade", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"));
}

#[test]
fn describes_auto_update() {
    cli()
        .args(["auto-update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"));
}
