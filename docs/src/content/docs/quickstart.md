---
title: Quickstart
description: Clone Rindle and run the Rust CLI and documentation locally.
order: 1
category: Start
summary: Build the CLI from source and get started with development.
---

Rindle is an early-stage Rust CLI scaffold. It has release and update infrastructure but no application-specific commands yet. No GitHub Release has been published, so the direct installers are not usable until the first release.

## Clone and check

Install Rust with [rustup](https://rustup.rs/) and [just](https://github.com/casey/just), then:

```bash
git clone https://github.com/amxv/rindle.git
cd rindle
just check
cargo run -- --help
```

The exact Rust version is pinned in `rust-toolchain.toml`.

## Work on the CLI

The command parser lives in `src/cli.rs`, with dispatch in `src/lib.rs` and implementations in `src/commands/`. Add product commands there and corresponding integration tests under `tests/`.

## Run the docs locally

Install [Bun](https://bun.sh/) and run:

```bash
just docs-install
just docs-dev
```

The site runs from the isolated `docs/` directory, rather than adding JavaScript dependencies to the Rust package.
