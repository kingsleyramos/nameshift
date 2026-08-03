#!/usr/bin/env bash
# Generate the §17 benchmark tree: 10k files across 100 directories.
# Usage: scripts/gen-fixtures.sh [target-dir]

set -euo pipefail

target="${1:-target/fixtures}"
dirs=100
files_per_dir=100

rm -rf "$target"
mkdir -p "$target"

for d in $(seq -w 1 "$dirs"); do
  dir="$target/dir-$d"
  mkdir -p "$dir"
  for f in $(seq -w 1 "$files_per_dir"); do
    printf 'fixture %s/%s\n' "$d" "$f" > "$dir/IMG_${d}${f}.jpg"
  done
done

echo "generated $((dirs * files_per_dir)) files under $target"
