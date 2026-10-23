---
title: Documentation development
description: How the Rindle documentation website is built and deployed.
order: 14
category: Reference
summary: Astro, documentation pages, checks, and the Vercel deployment workflow.
---

Rindle's documentation is a static [Astro](https://astro.build/) site, using [ZueDocs](https://github.com/amxv/zuedocs) for reading, navigation, page search, and Markdown support.

This page is for people contributing to the documentation. You don't need to do any of this to use Rindle.

## Run it locally

From the repository root:

```sh
just docs-install
just docs-dev
```

The site lives under `docs/`. Content pages are Markdown files in `docs/src/content/docs/`, and the homepage is `docs/src/pages/index.astro`. Design tokens and the site stylesheet are in `docs/src/styles/`.

## Check changes

```sh
just docs-test
just docs-check
just docs-build
```

The checks validate the site and its deployment workflow. The documentation can be read normally in a browser or as Markdown, using the `.md` versions of its routes.

## Publishing documentation

The GitHub Actions docs workflow deploys the static site to Vercel after changes in `docs/` are pushed to `main`. The public URL is **[rindle.ashray.xyz](https://rindle.ashray.xyz)**.

Site deployment is separate from native GitHub Releases: improving a guide does not require a new CLI version.
