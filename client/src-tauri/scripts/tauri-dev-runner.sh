#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Cadence dev runner expected the path to the compiled Tauri binary." >&2
  exit 64
fi

readonly executable="$1"
shift

if [[ "$(uname -s)" == "Darwin" ]]; then
  if ! command -v codesign >/dev/null 2>&1; then
    echo "Cadence dev runner requires codesign so macOS privacy prompts can read Info.plist." >&2
    exit 69
  fi

  if ! output=$(
    codesign \
      --force \
      --sign - \
      --identifier "${CADENCE_DEV_CODESIGN_IDENTIFIER:-dev.sn0w.cadence}" \
      "$executable" \
      2>&1
  ); then
    echo "$output" >&2
    echo "Cadence dev runner could not sign the Tauri binary for macOS privacy prompts." >&2
    exit 65
  fi
fi

exec "$executable" "$@"
