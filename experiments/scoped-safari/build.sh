#!/bin/sh
set -eu
# Standalone macOS permission experiment. NOT installed by `rindle setup`.
cd "$(dirname "$0")"
output=${RINDLE_SCOPED_PROBE_DIR:-"/tmp/rindle-scoped-safari-experiment"}
app="$output/RindleScopedProbe.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp Info.plist "$app/Contents/Info.plist"
chmod 644 "$app/Contents/Info.plist"
swiftc -O -framework AppKit -framework Foundation -lsqlite3 main.swift ScopedSafariService.swift -o "$app/Contents/MacOS/RindleScopedProbe"
chmod 755 "$app/Contents/MacOS/RindleScopedProbe"
# For bookmark persistence across versions use a stable, properly provisioned
# signing identity. Ad-hoc is only for this *isolated* local experiment.
codesign --force --sign "${CODESIGN_IDENTITY:--}" --entitlements ScopedSafariProbe.entitlements --timestamp=none "$app"
printf 'Scoped, sandboxed probe ready: %s\n' "$app"
printf 'To request folder-specific access: open %s\n' "$app"
printf 'To test persisted access: "%s/Contents/MacOS/RindleScopedProbe" --verify\n' "$app"
