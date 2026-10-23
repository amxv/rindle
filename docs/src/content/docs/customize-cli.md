---
title: Customize the CLI
description: Add the first product command to Rindle without disturbing release plumbing.
order: 2
category: Development
summary: Where application behavior belongs and how to keep the CLI testable.
---

## Rust structure

```text
src/main.rs       process exit/error boundary
src/lib.rs        application dispatch
src/cli.rs        clap parser model
src/commands/     command implementations
tests/            binary-level integration tests
```

Keep `main.rs` small. Command parsing and behavior should remain reachable from library code so tests can exercise the application without reproducing process setup.

## Add a command

Rindle has no application-specific commands yet. Add a `clap` variant in `src/cli.rs`, create a focused module under `src/commands/`, then dispatch it from `src/lib.rs`. Keep the existing update commands intact.

Prefer command-specific help text to a large custom help renderer. `clap` derives version information from the Cargo package, so no separate version file is required.

## Validate while editing

```bash
just check-fast
cargo test --locked <test-name>
```

Before pushing:

```bash
just check
```

Project lints forbid unsafe code and deny `dbg!`, `todo!`, and `unimplemented!`.
