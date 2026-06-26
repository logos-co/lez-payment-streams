#!/usr/bin/env bash
# Step 18 Part B — full testnet dual-host Store E2E.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/testnet-common.sh
source "$REPO_ROOT/scripts/testnet-common.sh"

require_testnet_rpc
echo "testnet block height: $(testnet_rpc_last_block)"

SUBMIT_BIN="$(lez_testnet_submit_bin)"
export LEZ_TESTNET_SUBMIT="$SUBMIT_BIN"
export PATH="$(dirname "$SUBMIT_BIN"):$PATH"

export CHAIN=testnet
export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet.json}"
if [[ ! -f "$FIXTURE_MANIFEST" ]]; then
  echo "verify-step18: missing $FIXTURE_MANIFEST (run make bootstrap-testnet)" >&2
  exit 2
fi

export LEZ_TESTNET_WALLET_CONFIG="${LEZ_TESTNET_WALLET_CONFIG:-$TESTNET_WALLET_DIR/wallet_config.json}"
export LEZ_TESTNET_WALLET_STORAGE="${LEZ_TESTNET_WALLET_STORAGE:-$TESTNET_WALLET_DIR/storage.json}"

WALLET_CFG_510="$(patch_510_wallet_config_for_testnet)"
export WALLET_CONFIG="$WALLET_CFG_510"
export WALLET_STORAGE="${WALLET_STORAGE:-$REPO_ROOT/.scaffold/wallet/storage.json}"

if [[ ! -f "$WALLET_STORAGE" ]]; then
  echo "ERROR: 510 wallet storage missing at $WALLET_STORAGE" >&2
  exit 1
fi
if [[ ! -f "$LEZ_TESTNET_WALLET_STORAGE" ]]; then
  echo "ERROR: rc3 testnet wallet missing (run bootstrap-testnet)" >&2
  exit 1
fi

echo "--- read smoke (510 module → testnet) ---"
WALLET_CONFIG="$WALLET_CFG_510" FIXTURE_MANIFEST="$FIXTURE_MANIFEST" \
  "$REPO_ROOT/scripts/verify-step18-testnet-read-smoke.sh"

exec "$REPO_ROOT/scripts/demo-e2e-local.sh"
