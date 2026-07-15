#!/usr/bin/env bash
# Clears USER_JOURNEY wallet and module install state for a clean rerun.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODULES="${MODULES:-$REPO_ROOT/.scaffold/e2e/user/modules}"
WALLET_HOME="${WALLET_HOME:-$REPO_ROOT/.scaffold/e2e/testnet-wallet}"

logoscore stop 2>/dev/null || true

if [[ -d "$MODULES" ]]; then
  rm -rf "${MODULES:?}/"*
fi
mkdir -p "$MODULES"

if [[ -d "$WALLET_HOME" ]]; then
  rm -rf "$WALLET_HOME"
fi

echo "Cleared: $MODULES (contents), $WALLET_HOME"
echo "Next: ./scripts/user-journey-shell.sh, then USER_JOURNEY from Step 1."
