#!/usr/bin/env bash
# Maintainer regression: two Store integration runs on one local ledger (no restore
# between legs). Not part of the external integrator verification story.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

chmod +x scripts/e2e.sh scripts/lifecycle.sh scripts/fixture.sh scripts/e2e/*.py \
  scripts/e2e/continuation-owner-topup.sh 2>/dev/null || true

MODE=store CHAIN=local E2E_REUSE_BASELINE_VAULT=1 ./scripts/e2e.sh local run

echo "Waiting for sequencer before second demo leg (no restore)…"
sleep 45

./scripts/e2e/continuation-owner-topup.sh

SKIP_BUILD=1 SKIP_SEED=1 RESTORE_LOCALNET=0 E2E_REUSE_BASELINE_VAULT=1 MODE=store CHAIN=local ./scripts/e2e.sh local run
