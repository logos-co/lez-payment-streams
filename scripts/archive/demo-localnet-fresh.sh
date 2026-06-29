#!/usr/bin/env bash
# Blank-slate localnet fixture for demos (see docs/demo-localnet-recovery.md).
# Step 17b: full chain rebuild + snapshot; for default restore path use demo-localnet-prepare.sh.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export FULL_RESET=1
exec "$REPO_ROOT/scripts/archive/demo-localnet-prepare.sh"
