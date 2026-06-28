#!/usr/bin/env bash
# Write vault-only fixture manifest (schema v2, no stream fields).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PROGRAM_BIN="${PAYMENT_STREAMS_GUEST_BIN:-methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
WALLET_HOME="${LEE_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}"
export LEE_WALLET_HOME_DIR="$WALLET_HOME"

DEPOSIT_AMOUNT="${SEED_DEPOSIT_AMOUNT:-1000}"
STREAM_RATE="${SEED_STREAM_RATE:-1}"
STREAM_ALLOCATION="${SEED_STREAM_ALLOCATION:-200}"
SEQUENCER_URL="${SEQUENCER_URL:-http://127.0.0.1:3040}"

STATE_FILE=".lez_payment_streams-state"
PROVIDER_FILE=".lez_payment_streams-fixture-provider"

if [[ ! -f "$STATE_FILE" ]]; then
  echo "ERROR: missing $STATE_FILE" >&2
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

cargo run --quiet --manifest-path examples/Cargo.toml --bin seed_localnet_fixture -- write-vault-manifest \
  --program-bin "$PROGRAM_BIN" \
  --owner "$SIGNER_ID" \
  --provider "$PROVIDER" \
  --deposit-amount "$DEPOSIT_AMOUNT" \
  --stream-rate "$STREAM_RATE" \
  --allocation "$STREAM_ALLOCATION" \
  --sequencer-url "$SEQUENCER_URL" \
  --output "$MANIFEST"

echo "Wrote vault baseline manifest: $MANIFEST"
