#!/usr/bin/env bash
# Step 18 Part B — one-time guest deploy to public testnet (legacy send_tx via lez-testnet-submit).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/testnet-common.sh
source "$REPO_ROOT/scripts/testnet-common.sh"

require_testnet_rpc
ensure_testnet_rc3_wallet

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

SUBMIT_BIN="$(lez_testnet_submit_bin)"
WALLET_CFG="$TESTNET_WALLET_DIR/wallet_config.json"

echo "=== deploy-testnet (expected program-id $EXPECTED_ID, ELF $(stat -c%s "$PROGRAM_BIN") bytes) ==="
set +e
DEPLOY_OUT="$("$SUBMIT_BIN" deploy-program \
  --wallet-config "$WALLET_CFG" \
  --program-bin "$PROGRAM_BIN" 2>&1)"
DEPLOY_RC=$?
set -e
echo "$DEPLOY_OUT"

if [[ "$DEPLOY_RC" -ne 0 ]]; then
  if echo "$DEPLOY_OUT" | rg -qi 'Transaction too large'; then
    echo "ERROR: deploy exceeds public testnet max transaction size." >&2
    echo "Guest ELF is $(stat -c%s "$PROGRAM_BIN") bytes; coordinate LEZ testnet tx size policy or shrink the guest." >&2
    exit 1
  fi
  if echo "$DEPLOY_OUT" | rg -qi 'already|exist|duplicate|deployed|ProgramAlreadyExists'; then
    echo "Treat deploy as idempotent success (program already on chain)."
  else
    echo "ERROR: deploy-program failed (exit $DEPLOY_RC)" >&2
    exit "$DEPLOY_RC"
  fi
fi

ACTUAL_ID="$EXPECTED_ID"
echo "program_id_hex=$ACTUAL_ID"
echo "Update fixtures/testnet.json.example program_id_hex after verifying on chain."
echo "=== deploy-testnet done ==="
