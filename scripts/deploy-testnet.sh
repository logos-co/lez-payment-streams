#!/usr/bin/env bash
# Step 18 Part B — one-time guest deploy to public testnet (rc5 wallet deploy-program).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/archive/testnet-common.sh
source "$REPO_ROOT/scripts/archive/testnet-common.sh"

require_testnet_rpc
ensure_testnet_wallet

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Building guest…"
  make build
fi

EXPECTED_ID="${TESTNET_PROGRAM_ID_HEX:-$(make -s program-id | sed -n 's/.*ImageID (hex bytes): //p' | tr -d ' ')}"
WALLET_BIN="$(lez_wallet_bin)"
export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
export LEE_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"

echo "=== deploy-testnet (expected program-id ${EXPECTED_ID:-unknown}, ELF $(stat -c%s "$PROGRAM_BIN") bytes) ==="
set +e
"$WALLET_BIN" deploy-program "$PROGRAM_BIN"
DEPLOY_RC=$?
set -e

if [[ "$DEPLOY_RC" -ne 0 ]]; then
  echo "deploy-program exit $DEPLOY_RC (program may already be on chain; verify with make program-id)"
else
  echo "deploy-program exit 0"
fi

ACTUAL_ID="$(make -s program-id | sed -n 's/.*ImageID (hex bytes): //p' | tr -d ' ')"
if [[ "$ACTUAL_ID" != "$EXPECTED_ID" ]]; then
  echo "WARN: make program-id ($ACTUAL_ID) != expected ($EXPECTED_ID)" >&2
fi
echo "program_id_hex=$ACTUAL_ID"
echo "=== deploy-testnet done ==="
