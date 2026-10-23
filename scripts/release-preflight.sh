#!/bin/sh
# Fail closed before creating a GitHub tag. This prints only SECRET NAMES,
# never Apple signing materials or user information.
set -eu
cd "$(dirname "$0")/.."

command -v gh >/dev/null 2>&1 || { echo 'GitHub CLI required for release preflight' >&2; exit 1; }
command -v dist >/dev/null 2>&1 || { echo 'Pinned cargo-dist is required for release preflight' >&2; exit 1; }
if [ -n "$(git status --porcelain)" ]; then
  echo 'Commit all release changes before tagging' >&2
  exit 1
fi
git fetch --quiet origin main
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
  echo 'Push the release candidate to origin/main before tagging' >&2
  exit 1
fi
dist generate --check

secrets=$(gh secret list --repo amxv/rindle --json name --jq '.[].name')
missing=0
for key in \
  RINDLE_APPLE_DEVELOPER_ID_P12_B64 \
  RINDLE_APPLE_DEVELOPER_ID_P12_PASSWORD \
  RINDLE_APPLE_NOTARY_KEY_P8_B64 \
  RINDLE_APPLE_NOTARY_KEY_ID \
  RINDLE_APPLE_NOTARY_ISSUER_ID
do
  if ! printf '%s\n' "$secrets" | grep -qx "$key"; then
    echo "Missing GitHub Actions secret: $key" >&2
    missing=1
  fi
done
if [ "$missing" -ne 0 ]; then
  echo 'No release tag created. Apple Developer ID/notarization credentials must be configured.' >&2
  exit 1
fi

# The custom universal-helper build MUST be in the release host's `needs`
# list, and the installer must verify its notarized signature and checksum.
grep -q 'custom-build-scoped-helper' .github/workflows/release.yml
grep -q 'artifacts-scoped-helper' .github/workflows/build-scoped-helper.yml
grep -q 'Developer ID Application' docs/public/install.sh
grep -q 'shasum -a 256 -c' docs/public/install.sh
echo 'Release preflight passed: clean main, required GitHub secrets, generated gated release workflow.'
