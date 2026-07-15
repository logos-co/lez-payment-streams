#!/usr/bin/env bash
# Opens a shell with logoscore and lgpm for docs/journeys/USER_JOURNEY.md.
# Requires: Nix (flakes), lgs on PATH, host Docker/Rust/RISC Zero for guest build and lgs setup.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if ! command -v lgs >/dev/null 2>&1; then
  echo "lgs not on PATH; install Logos Scaffold (see docs/journeys/USER_JOURNEY.md#prerequisites)." >&2
  exit 1
fi
exec nix shell --accept-flake-config \
  github:logos-co/logos-logoscore-cli/0.2.0 \
  --command bash -c "cd \"${REPO_ROOT}\" && exec bash"
