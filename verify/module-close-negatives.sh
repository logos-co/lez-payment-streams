#!/usr/bin/env bash
# Step 44 module-local negative cell: asserted close/create reject tokens.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export MODE=module
export CHAIN=local
export MODULE_E2E_CLOSE_NEGATIVES=1
export MODULE_E2E_SKIP_CLOSE=1
export MODULE_E2E_TOPUP=0
export MODULE_E2E_PAUSE_RESUME=0
export MODULE_E2E_WITHDRAW=0
export CLOSE_ROLE=owner

chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/module-e2e.sh
exec ./verify/e2e.sh local run
