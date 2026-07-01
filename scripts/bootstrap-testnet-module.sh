#!/usr/bin/env bash
# One-time module-only testnet fixture bootstrap.
# Creates a dedicated vault (default vault_id=1) separate from the Store flow's
# vault 0, so the module E2E has a clean baseline.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/archive/testnet-common.sh
source "$REPO_ROOT/scripts/archive/testnet-common.sh"

require_testnet_rpc
ensure_testnet_wallet

export TESTNET_SKIP_PINATA="${TESTNET_SKIP_PINATA:-1}"
export TESTNET_AUTH_TRANSFER_ELF_PATH="${TESTNET_AUTH_TRANSFER_ELF_PATH:-$(testnet_auth_transfer_elf_path)}"

MANIFEST="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet-module.json}"
PROGRAM_ID_HEX="${TESTNET_PROGRAM_ID_HEX:-16b95d3701d256eecd41d5a55e4f570331994d787abf0cba44eec209e24f8a44}"
VAULT_ID="${VAULT_ID:-1}"

# Reuse owner/provider from existing testnet.json if available.
OWNER=""
PROVIDER=""
if [[ -f "$REPO_ROOT/fixtures/testnet.json" && "${TESTNET_REUSE_FIXTURE:-1}" == "1" ]]; then
  OWNER="$(python3 -c "import json; print(json.load(open('$REPO_ROOT/fixtures/testnet.json')).get('owner_account_id','').strip())")"
  PROVIDER="$(python3 -c "import json; print(json.load(open('$REPO_ROOT/fixtures/testnet.json')).get('provider_account_id','').strip())")"
  if [[ -n "$OWNER" ]]; then
    echo "Reusing owner/provider from existing testnet fixture"
    export TESTNET_SKIP_PINATA="${TESTNET_SKIP_PINATA:-1}"
  fi
fi

if [[ -z "$OWNER" ]]; then
  OWNER="$(ensure_testnet_owner_funded)"
fi

export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
WALLET_BIN="$(lez_wallet_bin)"

if [[ -z "$PROVIDER" ]]; then
  PROVIDER="${TESTNET_PROVIDER_ID:-}"
fi
if [[ -z "$PROVIDER" ]]; then
  PROVIDER="$("$WALLET_BIN" account new public 2>&1 | sed -n 's/.*Public\/\([A-Za-z0-9]*\).*/\1/p')"
  if [[ -z "$PROVIDER" ]]; then
    echo "ERROR: failed to create provider account" >&2
    exit 1
  fi
  echo "Created provider Public/$PROVIDER"
else
  echo "Reusing provider Public/$PROVIDER"
fi

if [[ ! -f "$PROGRAM_BIN" ]]; then
  make build
fi

SUBMIT_BIN="$(lez_testnet_submit_bin)"
export LEZ_TESTNET_SUBMIT="$SUBMIT_BIN"
export PATH="$(dirname "$SUBMIT_BIN"):$PATH"

echo "=== bootstrap-testnet-module owner=$OWNER provider=$PROVIDER vault=$VAULT_ID ==="

STREAM_RATE="${SEED_STREAM_RATE:-50}"
DEPOSIT_AMOUNT="${TESTNET_DEPOSIT_AMOUNT:-400}"
STREAM_ALLOCATION="${TESTNET_STREAM_ALLOCATION:-350}"

cargo run --quiet --manifest-path "$REPO_ROOT/examples/Cargo.toml" --bin bootstrap_testnet_fixture -- \
  --program-bin "$PROGRAM_BIN" \
  --owner "$OWNER" \
  --provider "$PROVIDER" \
  --program-id-hex "$PROGRAM_ID_HEX" \
  --rc3-wallet-config "$WALLET_CONFIG" \
  --rc3-wallet-storage "$WALLET_STORAGE" \
  --submit-helper "$SUBMIT_BIN" \
  --sequencer-url "$TESTNET_SEQUENCER" \
  --stream-rate "$STREAM_RATE" \
  --deposit-amount "$DEPOSIT_AMOUNT" \
  --stream-allocation "$STREAM_ALLOCATION" \
  --vault-id "$VAULT_ID" \
  --write-manifest "$MANIFEST"

echo "=== bootstrap-testnet-module done: $MANIFEST ==="
