#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 /path/to/App.app" >&2
  exit 64
fi

app_bundle="$1"
if [ ! -d "$app_bundle/Contents" ]; then
  echo "macOS app bundle not found: $app_bundle" >&2
  exit 66
fi

if ! command -v otool >/dev/null 2>&1; then
  echo "otool is required to verify macOS dylib linkage." >&2
  exit 69
fi

status=0
while IFS= read -r binary; do
  if ! file "$binary" | grep -Eq "Mach-O"; then
    continue
  fi

  forbidden_links="$(
    otool -L "$binary" |
      awk '$1 ~ "^/(opt/homebrew|opt/local|usr/local|sw)/" { print "  " $1 }'
  )"

  if [ -n "$forbidden_links" ]; then
    echo "::error file=$binary::Non-portable macOS dylib linkage found."
    printf '%s\n' "$forbidden_links"
    status=1
  fi
done < <(find "$app_bundle/Contents" -type f -perm -111 -print)

if [ "$status" -ne 0 ]; then
  echo "Refusing to ship an app that depends on local package-manager dylibs." >&2
  exit "$status"
fi

echo "macOS app dylib linkage is portable."
