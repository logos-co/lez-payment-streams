#!/usr/bin/env bash
# Step 18 Part B — vault/stream bootstrap on public testnet.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/testnet-common.sh
source "$REPO_ROOT/scripts/testnet-common.sh"

require_testnet_rpc
ensure_testnet_rc3_wallet
OWNER="$(ensure_testnet_owner_funded)"
sync_testnet_owner_to_510_wallet

export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
WALLET_BIN="$(lez_rc3_wallet_bin)"
PROVIDER="${TESTNET_PROVIDER_ID:-}"
if [[ -z "$PROVIDER" ]]; then
  PROVIDER="$("$WALLET_BIN" account new public 2>&1 | sed -n 's/.*Public\/\([A-Za-z0-9]*\).*/\1/p')"
  if [[ -z "$PROVIDER" ]]; then
    echo "ERROR: failed to create provider account" >&2
    exit 1
  fi
  echo "Created provider Public/$PROVIDER"
fi

if [[ ! -f "$PROGRAM_BIN" ]]; then
  make build
fi

PROGRAM_ID_HEX="${TESTNET_PROGRAM_ID_HEX:-$(make -s program-id)}"
SUBMIT_BIN="$(lez_testnet_submit_bin)"
export LEZ_TESTNET_SUBMIT="$SUBMIT_BIN"
export PATH="$(dirname "$SUBMIT_BIN"):$PATH"

MANIFEST="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet.json}"

echo "=== bootstrap-testnet owner=$OWNER provider=$PROVIDER ==="
cargo run --quiet --manifest-path "$REPO_ROOT/examples/Cargo.toml" --bin bootstrap_testnet_fixture -- \
  --program-bin "$PROGRAM_BIN" \
  --owner "$OWNER" \
  --provider "$PROVIDER" \
  --program-id-hex "$PROGRAM_ID_HEX" \
  --rc3-wallet-config "$TESTNET_WALLET_DIR/wallet_config.json" \
  --rc3-wallet-storage "$TESTNET_WALLET_DIR/storage.json" \
  --submit-helper "$SUBMIT_BIN" \
  --sequencer-url "$TESTNET_SEQUENCER" \
  --write-manifest "$MANIFEST"

echo "=== bootstrap-testnet done: $MANIFEST ==="
