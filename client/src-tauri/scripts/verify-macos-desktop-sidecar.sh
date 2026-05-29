#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 /path/to/App.app" >&2
  exit 64
fi

app_bundle="$1"
sidecar="$app_bundle/Contents/Resources/resources/xero-desktop-sidecar"

team_identifier() {
  local target="$1"
  codesign --display --verbose=4 "$target" 2>&1 \
    | sed -n 's/^TeamIdentifier=//p' \
    | head -1
}

if [ ! -f "$sidecar" ]; then
  echo "::error::Missing bundled desktop sidecar at $sidecar."
  exit 66
fi

if [ ! -x "$sidecar" ]; then
  echo "::error file=$sidecar::Bundled desktop sidecar is not executable."
  exit 66
fi

if ! file "$sidecar" | grep -q "Mach-O"; then
  echo "::error file=$sidecar::Bundled desktop sidecar is not a Mach-O binary."
  exit 66
fi

codesign --verify --strict --verbose=2 "$sidecar"

app_team="$(team_identifier "$app_bundle")"
sidecar_team="$(team_identifier "$sidecar")"
if [ -z "$app_team" ] || [ -z "$sidecar_team" ]; then
  echo "::error::Could not read macOS signing team identifiers for app and desktop sidecar." >&2
  exit 66
fi

if [ "$app_team" != "$sidecar_team" ]; then
  echo "::error file=$sidecar::Desktop sidecar signing team $sidecar_team does not match app signing team $app_team." >&2
  exit 66
fi

echo "macOS desktop sidecar is bundled and signed."
