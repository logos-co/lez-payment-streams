#!/usr/bin/env bash
# Blank-slate localnet fixture for demos (see docs/archive/operator/localnet-recovery.md).
# Legacy wrapper: sets FULL_RESET=1 and execs demo-localnet-prepare.sh.
# Prefer: make full-reset-localnet
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export FULL_RESET=1
exec "$REPO_ROOT/scripts/archive/demo-localnet-prepare.sh"
