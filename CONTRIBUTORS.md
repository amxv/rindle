# Contributing to Rindle

Rindle is a macOS-only Safari/iCloud ↔ Chromium history sync service. Contributions must preserve URL privacy, deduplicate imports, and avoid opening actual browser-history pages.

## Prerequisites

- Rust from `rust-toolchain.toml` (currently 1.99.1)
- `just`
- Node.js 22+ for the Manifest V3 extension tests
- Bun for the documentation site
- macOS Command Line Tools (including Swift and code signing) for release packaging

## Development and tests

```bash
just check-fast
just check
just build
./target/debug/rindle --help
```

The full checks cover formatting, Clippy warnings, Rust tests, and extension tests. Application behavior belongs in `src/` and `extension/`; update tests and user-facing docs alongside it. The native host tests must use isolated SQLite databases and test homes rather than live browser history.

## Documentation

```bash
just docs-install
just docs-test
just docs-check
just docs-dev
```

The docs live in `docs/` with Astro and ZueDocs. GitHub Actions deploys changed docs to Vercel from `main`; unrelated Rust commits do not request a deployment. Keep `docs/vercel.json` configured with `git.deploymentEnabled: false` and no `ignoreCommand` to avoid duplicated, quota-consuming deployments.

## Distribution

Rindle uses macOS-only GitHub Releases (Apple Silicon and Intel), without npm publishing, Linux, Windows or a menu bar app. `scripts/package-release.sh` builds the CLI and sandboxed helper archives; `.github/workflows/release.yml` runs smoke tests and publishes the checked archives. Follow `.agents/skills/release/SKILL.md` for the release process, and verify real Safari ↔ Dia syncing before publishing.
