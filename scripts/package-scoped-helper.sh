#!/bin/sh
# Build the macOS universal sandboxed Safari helper and a SHA-256-verified
# archive. Release mode REQUIRES a Developer ID certificate and notarization.
set -eu

mode=${1:-}
case "$mode" in
  --development|--release) ;;
  *) echo 'Usage: scripts/package-scoped-helper.sh --development|--release' >&2; exit 2 ;;
esac

if [ "$mode" = --release ]; then
  : "${RINDLE_DEVELOPER_ID_IDENTITY:?Release requires a Developer ID Application signing identity}"
  : "${RINDLE_NOTARY_KEY_PATH:?Release requires an App Store Connect API .p8 file}"
  : "${RINDLE_NOTARY_KEY_ID:?Release requires an Apple notarization key ID}"
  : "${RINDLE_NOTARY_ISSUER_ID:?Release requires an Apple notarization issuer ID}"
  test -r "$RINDLE_NOTARY_KEY_PATH"
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
umask 077
stage=$(mktemp -d "${TMPDIR:-/tmp}/rindle-helper-release.XXXXXXXX")
trap 'rm -rf "$stage"' EXIT HUP INT TERM
app="$stage/ScopedSafariHelper.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" target/distrib
cp experiments/scoped-safari/Info.plist "$app/Contents/Info.plist"
chmod 644 "$app/Contents/Info.plist"
version=$(awk -F '"' '/^version = "/ {print $2; exit}' Cargo.toml)
case "$version" in
  ''|*[!0-9A-Za-z._-]*) echo 'Could not determine a valid Cargo package version' >&2; exit 1 ;;
esac
plutil -replace CFBundleShortVersionString -string "$version" "$app/Contents/Info.plist"
plutil -replace CFBundleVersion -string "$version" "$app/Contents/Info.plist"

sdk=$(xcrun --sdk macosx --show-sdk-path)
src=experiments/scoped-safari
for arch in arm64 x86_64; do
  swiftc -O -sdk "$sdk" -target "$arch-apple-macosx14.0" \
    -framework AppKit -framework Foundation -lsqlite3 \
    "$src/main.swift" "$src/ScopedSafariService.swift" \
    -o "$stage/RindleScopedProbe-$arch"
done
lipo -create "$stage/RindleScopedProbe-arm64" "$stage/RindleScopedProbe-x86_64" \
  -output "$app/Contents/MacOS/RindleScopedProbe"
chmod 755 "$app/Contents/MacOS/RindleScopedProbe"

if [ "$mode" = --release ]; then
  codesign --force --options runtime --timestamp \
    --entitlements "$src/ScopedSafariProbe.entitlements" \
    --sign "$RINDLE_DEVELOPER_ID_IDENTITY" "$app"
  # A valid local development certificate is NOT acceptable for release.
  codesign -dv --verbose=3 "$app" 2>"$stage/signature.txt"
  grep -q '^Authority=Developer ID Application:' "$stage/signature.txt" || {
    echo 'Release helper was not signed by a Developer ID Application identity' >&2
    exit 1
  }
  grep -Eq '^TeamIdentifier=[A-Z0-9]{10}$' "$stage/signature.txt" || {
    echo 'Missing Apple Developer team identity' >&2
    exit 1
  }
  ditto -c -k --keepParent "$app" "$stage/notary-upload.zip"
  xcrun notarytool submit "$stage/notary-upload.zip" \
    --key "$RINDLE_NOTARY_KEY_PATH" --key-id "$RINDLE_NOTARY_KEY_ID" \
    --issuer "$RINDLE_NOTARY_ISSUER_ID" --wait --output-format json \
    >"$stage/notary-status.json"
  /usr/bin/python3 - "$stage/notary-status.json" <<'PY'
import json,sys
result=json.load(open(sys.argv[1]))
if result.get('status') != 'Accepted':
    raise SystemExit('Apple notarization was not accepted; refusing release')
print('Apple notarization status: Accepted')
PY
  xcrun stapler staple "$app"
  xcrun stapler validate "$app"
  spctl --assess --type execute --verbose "$app"
else
  codesign --force --timestamp=none \
    --entitlements "$src/ScopedSafariProbe.entitlements" \
    --sign "${RINDLE_LOCAL_SIGNING_IDENTITY:--}" "$app"
  printf '%s\n' 'Development helper only: not notarized and must not be publicly distributed.'
fi

codesign --verify --strict --deep "$app"
arches=$(lipo -archs "$app/Contents/MacOS/RindleScopedProbe")
case " $arches " in *' arm64 '*) ;; *) echo 'Missing arm64 slice' >&2; exit 1 ;; esac
case " $arches " in *' x86_64 '*) ;; *) echo 'Missing x86_64 slice' >&2; exit 1 ;; esac

archive=target/distrib/rindle-scoped-helper-universal.zip
ditto -c -k --sequesterRsrc --keepParent "$app" "$archive"
(
  cd target/distrib
  shasum -a 256 rindle-scoped-helper-universal.zip > rindle-scoped-helper-universal.zip.sha256
  shasum -a 256 -c rindle-scoped-helper-universal.zip.sha256
)
printf 'Rindle scoped Safari helper packaged (%s, %s): %s\n' "$mode" "$arches" "$archive"
