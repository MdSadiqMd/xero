#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 /path/to/App.app" >&2
  exit 64
fi

app_bundle="$1"
sidecar="$app_bundle/Contents/Resources/resources/xero-desktop-sidecar"

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

echo "macOS desktop sidecar is bundled and signed."
