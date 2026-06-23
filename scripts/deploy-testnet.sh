#!/usr/bin/env bash
# Step 18 Part B — one-time guest deploy to public testnet (rc3 wallet CLI).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/testnet-common.sh
source "$REPO_ROOT/scripts/testnet-common.sh"

require_testnet_rpc
ensure_testnet_rc3_wallet
ensure_testnet_owner_funded >/dev/null

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Building guest…"
  make build
fi

EXPECTED_ID="$(
  cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" --bin lez_payment_streams_cli -- \
    -i "$REPO_ROOT/lez-payment-streams-idl.json" program-id "$PROGRAM_BIN" 2>/dev/null \
    | sed -n 's/.*ImageID (hex bytes): //p' | tr -d ' \r'
)"
if [[ -z "$EXPECTED_ID" ]]; then
  echo "ERROR: could not read program ImageID hex from guest ELF" >&2
  exit 1
fi
export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
WALLET_BIN="$(lez_rc3_wallet_bin)"

echo "=== deploy-testnet (expected program-id $EXPECTED_ID) ==="
set +e
DEPLOY_OUT="$("$WALLET_BIN" deploy-program "$PROGRAM_BIN" 2>&1)"
DEPLOY_RC=$?
set -e
echo "$DEPLOY_OUT"

if [[ "$DEPLOY_RC" -ne 0 ]]; then
  if echo "$DEPLOY_OUT" | rg -qi 'Transaction too large'; then
    echo "ERROR: deploy-program exceeds public testnet max transaction size (~511800 bytes)." >&2
    echo "Guest ELF is ~$(stat -c%s "$PROGRAM_BIN") bytes; coordinate LEZ testnet max_block_size or shrink the guest." >&2
    exit 1
  fi
  if echo "$DEPLOY_OUT" | rg -qi 'already|exist|duplicate|deployed'; then
    echo "Treat deploy as idempotent success (program may already be on chain)."
  else
    echo "ERROR: deploy-program failed (exit $DEPLOY_RC)" >&2
    exit "$DEPLOY_RC"
  fi
fi

ACTUAL_ID="$EXPECTED_ID"

echo "program_id_hex=$ACTUAL_ID"
echo "=== deploy-testnet done ==="
