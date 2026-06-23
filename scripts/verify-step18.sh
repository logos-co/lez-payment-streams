#!/usr/bin/env bash
# Part B placeholder — full testnet dual-host Store E2E.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
export CHAIN=testnet
export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet.json}"
if [[ ! -f "$FIXTURE_MANIFEST" ]]; then
  echo "verify-step18: missing $FIXTURE_MANIFEST (run bootstrap-testnet in Part B)" >&2
  exit 2
fi
exec "$REPO_ROOT/scripts/demo-e2e-local.sh"
