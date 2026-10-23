---
title: Command reference
description: Rindle's current commands and repository development tasks.
order: 4
category: Reference
summary: Built-in update commands, checks, and build helpers.
---

## CLI

```bash
rindle --help
rindle --version
rindle upgrade --check
rindle upgrade
rindle auto-update on
rindle auto-update status
rindle auto-update off
```

`upgrade` and `auto-update` are part of the direct-distribution scaffold. They only become useful once a tagged GitHub Release is available. Automatic updates are off by default and must be enabled explicitly; `on` registers a daily user-owned system task.

Rindle does not yet have any application-specific subcommands.

## Development

```bash
just check-fast
just check
just build
just build-release
just run --help
```

Under the hood, these use `cargo fmt`, `cargo check`, Clippy, and `cargo test` with a committed lockfile.

## Docs and distribution

```bash
just docs-install
just docs-dev
just docs-check
just docs-test
dist plan
dist generate
```

Use `just release-tag <version>` only after preparing a release, validating the CLI, and updating the changelog. See [Releases and installers](/docs/releases).
