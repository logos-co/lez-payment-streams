#!/usr/bin/env bash
# Step 17b — create on-chain stream at per-run stream_id and write fixtures/localnet.json.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

LEZ_PIN="$(grep -A2 '\[repos.lez\]' scaffold.toml | grep '^pin' | sed 's/.*"\([^"]*\)".*/\1/')"
export PATH="$HOME/.cache/logos-scaffold/repos/lez/${LEZ_PIN}/target/release:$PATH"

PROGRAM_BIN="${PAYMENT_STREAMS_GUEST_BIN:-methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
STATE_FILE=".lez_payment_streams-state"
PROVIDER_FILE=".lez_payment_streams-fixture-provider"
MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
WALLET_HOME="${LEE_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}"
export LEE_WALLET_HOME_DIR="$WALLET_HOME"

DEPOSIT_AMOUNT="${SEED_DEPOSIT_AMOUNT:-1000}"
STREAM_RATE="${SEED_STREAM_RATE:-1}"
STREAM_ALLOCATION="${SEED_STREAM_ALLOCATION:-200}"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: '$1' not on PATH" >&2
    exit 1
  }
}

require_cmd lgs
require_cmd wallet

if ! lgs localnet status 2>/dev/null | grep -qi running; then
  echo "Starting localnet…"
  "$REPO_ROOT/scripts/ensure-scaffold-lez-layout.sh"
  lgs localnet start
fi

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Building guest…"
  make build
fi

if [[ ! -f "$STATE_FILE" ]]; then
  echo "ERROR: missing $STATE_FILE (restore snapshot or run prefund)" >&2
  exit 1
fi
# shellcheck disable=SC1090
source "$STATE_FILE"
if [[ -z "${SIGNER_ID:-}" ]]; then
  echo "ERROR: SIGNER_ID missing in $STATE_FILE" >&2
  exit 1
fi

if [[ ! -f "$PROVIDER_FILE" ]]; then
  echo "ERROR: missing $PROVIDER_FILE" >&2
  exit 1
fi
PROVIDER="$(cat "$PROVIDER_FILE")"

if [[ -z "${STREAM_ID:-}" ]]; then
  STREAM_ID="$(cargo run --quiet --manifest-path examples/Cargo.toml --bin seed_localnet_fixture -- read-vault-next-stream-id \
    --program-bin "$PROGRAM_BIN" \
    --owner "$SIGNER_ID")"
fi

CREATE_EXTRA=()
# Step 24c / E2E: never silently skip create when a stream PDA already exists (stale snapshot).
if [[ "${CREATE_FORCE:-0}" == "1" || "${E2E_PER_RUN_STREAM:-0}" == "1" ]]; then
  CREATE_EXTRA+=(--force)
fi

echo "Creating stream ${STREAM_ID} (rate=$STREAM_RATE allocation=$STREAM_ALLOCATION)…"
logoscore stop 2>/dev/null || true
sleep 1
# Step 24c: let any pending owner tx from a prior logoscore smoke fold before the seed
# fetches the committed nonce, otherwise the create tx is dropped as a duplicate nonce.
SEQUENCER_URL="${SEQUENCER_URL:-}" FIXTURE_MANIFEST="$MANIFEST" \
  "$REPO_ROOT/scripts/wait-chain-settle.sh" "$SIGNER_ID" || true
SEQUENCER_URL="${SEQUENCER_URL:-}" \
  "$REPO_ROOT/scripts/wait-clock-synced.sh"
cargo run --quiet --manifest-path examples/Cargo.toml --bin seed_localnet_fixture -- create-stream-onchain \
  --program-bin "$PROGRAM_BIN" \
  --owner "$SIGNER_ID" \
  --provider "$PROVIDER" \
  --stream-id "$STREAM_ID" \
  --stream-rate "$STREAM_RATE" \
  --allocation "$STREAM_ALLOCATION" \
  "${CREATE_EXTRA[@]}" \
  --write-manifest "$MANIFEST"

echo "Done. Manifest: $MANIFEST"
