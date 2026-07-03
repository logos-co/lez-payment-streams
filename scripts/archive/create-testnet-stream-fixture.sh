#!/usr/bin/env bash
# Step 24c — per-run CreateStream on public testnet via lez-testnet-submit (no logoscore chainAction).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/testnet-common.sh
source "$REPO_ROOT/scripts/archive/testnet-common.sh"

require_testnet_rpc

MANIFEST="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet.json}"
PROGRAM_BIN="${PAYMENT_STREAMS_GUEST_BIN:-methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
PROGRAM_ID_HEX="${TESTNET_PROGRAM_ID_HEX:-16b95d3701d256eecd41d5a55e4f570331994d787abf0cba44eec209e24f8a44}"

if [[ ! -f "$MANIFEST" ]]; then
  echo "ERROR: missing $MANIFEST (run make bootstrap-testnet)" >&2
  exit 1
fi

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Building guest…"
  make build
fi

ensure_testnet_wallet
SUBMIT_BIN="$(lez_testnet_submit_bin)"
export LEZ_TESTNET_SUBMIT="$SUBMIT_BIN"
export PATH="$(dirname "$SUBMIT_BIN"):$PATH"
export WALLET_CONFIG="${WALLET_CONFIG:-$TESTNET_WALLET_DIR/wallet_config.json}"
export WALLET_STORAGE="${WALLET_STORAGE:-$TESTNET_WALLET_DIR/storage.json}"

OWNER="$(python3 -c "import json; print(json.load(open('$MANIFEST')).get('owner_account_id','').strip())")"
PROVIDER="$(python3 -c "import json; print(json.load(open('$MANIFEST')).get('provider_account_id','').strip())")"
VAULT_ID="$(python3 -c "import json; print(int(json.load(open('$MANIFEST')).get('vault_id',0)))")"
SEQ_URL="$(python3 -c "import json; print(json.load(open('$MANIFEST')).get('sequencer_url','').strip())")"

STREAM_RATE="${SEED_STREAM_RATE:-$(python3 -c "import json; m=json.load(open('$MANIFEST')); print(int(m.get('stream_rate',50)))")}"
ALLOCATION="${SEED_ALLOCATION:-${SEED_STREAM_ALLOCATION:-$(python3 -c "import json; m=json.load(open('$MANIFEST')); print(int(m.get('allocation', m.get('stream_allocation', 350))))")}}"

if [[ -z "${STREAM_ID:-}" ]]; then
  echo "ERROR: STREAM_ID required for testnet per-run create" >&2
  exit 1
fi

CREATE_EXTRA=()
if [[ "${CREATE_FORCE:-0}" == "1" || "${E2E_PER_RUN_STREAM:-0}" == "1" ]]; then
  CREATE_EXTRA+=(--force)
fi

echo "Creating testnet stream ${STREAM_ID} (rate=$STREAM_RATE allocation=$ALLOCATION)…"
SEQUENCER_URL="${SEQUENCER_URL:-$SEQ_URL}" FIXTURE_MANIFEST="$MANIFEST" \
  "$REPO_ROOT/scripts/archive/wait-chain-settle.sh" "$OWNER" || true

cargo run --quiet --manifest-path examples/Cargo.toml --bin bootstrap_testnet_fixture -- \
  --program-bin "$PROGRAM_BIN" \
  --owner "$OWNER" \
  --provider "$PROVIDER" \
  --program-id-hex "$PROGRAM_ID_HEX" \
  --rc3-wallet-config "$WALLET_CONFIG" \
  --rc3-wallet-storage "$WALLET_STORAGE" \
  --submit-helper "$SUBMIT_BIN" \
  --sequencer-url "$SEQ_URL" \
  --vault-id "$VAULT_ID" \
  --stream-id "$STREAM_ID" \
  --stream-rate "$STREAM_RATE" \
  --allocation "$ALLOCATION" \
  --write-manifest "$MANIFEST" \
  --create-stream-only \
  "${CREATE_EXTRA[@]}"

echo "Done. Manifest: $MANIFEST"
