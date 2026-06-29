#!/usr/bin/env bash
# Remove Step 12 / demo logoscore instance dirs (see docs/archive/operator/localnet-recovery.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

removed=0
shopt -s nullglob
for dir in "$REPO_ROOT"/.scaffold/step12-persist*; do
  if [[ -d "$dir" ]]; then
    rm -rf "$dir"
    echo "removed $dir"
    removed=$((removed + 1))
  fi
done
shopt -u nullglob

if [[ "$removed" -eq 0 ]]; then
  echo "no .scaffold/step12-persist* dirs to remove"
fi
