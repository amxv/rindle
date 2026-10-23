---
title: Docs site maintenance
description: Maintain the Astro/ZueDocs site without mixing JavaScript tooling into the Rust root.
order: 5
category: Reference
summary: The docs workspace boundary, checks, and Vercel behavior.
---

## Workspace boundary

Everything required by the web app lives in `docs/`:

```text
docs/package.json
docs/bun.lock
docs/astro.config.mjs
docs/vercel.json
docs/scripts/
docs/src/
```

The repository root intentionally has no Node package or Astro configuration.

## Development

```bash
just docs-install
just docs-dev
```

## Validation

```bash
just docs-test
just docs-check
```

Astro check is sufficient locally; GitHub Actions builds the production output before deploying to Vercel.

## Vercel

Set the Vercel project Root Directory to `docs` and connect its GitHub repo,
but keep `git.deploymentEnabled: false` in the committed `vercel.json`. The
`.github/workflows/docs.yml` workflow responds to `docs/**` changes on
`main`, validates Astro, and deploys with the Vercel CLI. Rust-only and
Cargo changes never request a Vercel deployment, so they cannot use its
daily deployment quota through this pipeline.

The one-time setup provisions a project-scoped CI token as the `VERCEL_TOKEN`
GitHub Actions secret and saves the two non-sensitive project IDs as Actions
variables. The GitHub workflow builds and deploys Astro's static
output with `--prebuilt`, avoiding broad settings calls that may fail with
restricted tokens. Agents use only `just docs-check` for local Astro
validation. `just docs-test` validates both the GitHub path
filter and the retained local Git diff helper `docs/scripts/should-build.mjs`.
It is deliberately **not** wired to `ignoreCommand`, because a canceled
Vercel Ignored Build Step still counts toward deployment quotas.
