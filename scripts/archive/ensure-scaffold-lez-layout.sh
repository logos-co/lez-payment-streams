#!/usr/bin/env bash
# LEZ 510+ checkouts nest sequencer under lez/; older lgs builds expect sequencer/ at pin root.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_PIN="$(grep -A2 '\[repos.lez\]' "$REPO_ROOT/scaffold.toml" | grep '^pin' | sed 's/.*"\([^"]*\)".*/\1/')"
LEZ_CACHE="${HOME}/.cache/logos-scaffold/repos/lez/${LEZ_PIN}"

if [[ ! -d "$LEZ_CACHE" ]]; then
  echo "WARN: LEZ cache missing at $LEZ_CACHE (run lgs setup)" >&2
  exit 0
fi

if [[ ! -e "$LEZ_CACHE/sequencer" && -d "$LEZ_CACHE/lez/sequencer" ]]; then
  ln -sfn lez/sequencer "$LEZ_CACHE/sequencer"
  echo "ensure-scaffold-lez-layout: linked $LEZ_CACHE/sequencer -> lez/sequencer"
fi

if [[ ! -f "$LEZ_CACHE/sequencer/service/configs/debug/sequencer_config.json" ]]; then
  echo "WARN: sequencer_config.json not found under $LEZ_CACHE/sequencer" >&2
  exit 1
fi
