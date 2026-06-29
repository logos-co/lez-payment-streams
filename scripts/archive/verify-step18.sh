#!/usr/bin/env bash
# Step 18 Part B — full testnet dual-host Store E2E.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/testnet-common.sh
source "$REPO_ROOT/scripts/archive/testnet-common.sh"

require_testnet_rpc
echo "testnet block height: $(testnet_rpc_last_block)"

ensure_testnet_wallet

SUBMIT_BIN="$(lez_testnet_submit_bin)"
export LEZ_TESTNET_SUBMIT="$SUBMIT_BIN"
export PATH="$(dirname "$SUBMIT_BIN"):$PATH"

export CHAIN=testnet
export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet.json}"
export E2E_PHASE="${E2E_PHASE:-core}"
export PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF="${PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF:-0}"
export TESTNET_SKIP_PREFLIGHT_TOPUP="${TESTNET_SKIP_PREFLIGHT_TOPUP:-1}"
if [[ "${TESTNET_SKIP_PREFLIGHT_TOPUP}" != "1" ]]; then
  echo "--- testnet preflight top-up (unaccrued for Store verify) ---"
  chmod +x "$REPO_ROOT/scripts/archive/testnet-preflight-topup.sh"
  "$REPO_ROOT/scripts/archive/testnet-preflight-topup.sh"
fi
if [[ ! -f "$FIXTURE_MANIFEST" ]]; then
  echo "verify-step18: missing $FIXTURE_MANIFEST (run make bootstrap-testnet)" >&2
  exit 2
fi

export WALLET_CONFIG="${WALLET_CONFIG:-$TESTNET_WALLET_DIR/wallet_config.json}"
export WALLET_STORAGE="${WALLET_STORAGE:-$TESTNET_WALLET_DIR/storage.json}"
export LEZ_TESTNET_WALLET_CONFIG="${LEZ_TESTNET_WALLET_CONFIG:-$WALLET_CONFIG}"
export LEZ_TESTNET_WALLET_STORAGE="${LEZ_TESTNET_WALLET_STORAGE:-$WALLET_STORAGE}"
export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
export LEE_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"

if [[ ! -f "$WALLET_STORAGE" ]]; then
  echo "ERROR: testnet wallet storage missing at $WALLET_STORAGE (run bootstrap-testnet)" >&2
  exit 1
fi

echo "--- read smoke (rc5 module → testnet) ---"
WALLET_CONFIG="$WALLET_CONFIG" WALLET_STORAGE="$WALLET_STORAGE" FIXTURE_MANIFEST="$FIXTURE_MANIFEST" \
  "$REPO_ROOT/scripts/archive/verify-step18-testnet-read-smoke.sh"

exec "$REPO_ROOT/scripts/archive/demo-e2e-local.sh"
