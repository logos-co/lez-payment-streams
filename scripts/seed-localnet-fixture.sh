#!/usr/bin/env bash
# Step 10a — idempotent local chain fixture (see docs/step10a-local-chain-fixture.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

LEZ_PIN="$(grep -A2 '\[repos.lez\]' scaffold.toml | grep '^pin' | sed 's/.*"\([^"]*\)".*/\1/')"
SCAFFOLD_WALLET="$HOME/.cache/logos-scaffold/repos/lez/${LEZ_PIN}/target/release/wallet"
if [[ -x "$SCAFFOLD_WALLET" ]]; then
  export PATH="$(dirname "$SCAFFOLD_WALLET"):$PATH"
fi

PROGRAM_BIN="methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
STATE_FILE=".lez_payment_streams-state"
WALLET_HOME="${LEE_WALLET_HOME_DIR:-${NSSA_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}}"
export LEE_WALLET_HOME_DIR="$WALLET_HOME"
MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
FORCE="${SEED_FORCE:-0}"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: '$1' not on PATH (run lgs setup from this repo)" >&2
    exit 1
  }
}

require_cmd lgs
require_cmd wallet

if ! lgs localnet status 2>/dev/null | grep -qi running; then
  echo "Starting localnet (lgs init/setup as needed)…"
  lgs init 2>/dev/null || true
  lgs setup
  lgs localnet start
fi

lgs localnet status 2>/dev/null || true
if ! lgs wallet -- check-health 2>/dev/null; then
  echo "WARN: wallet check-health failed (foreign localnet or program-id drift is common); continuing seed…" >&2
fi

echo "Building guest (IDL not required for on-chain seed)…"
make build

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "ERROR: missing $PROGRAM_BIN after make build" >&2
  exit 1
fi

echo "Deploying program…"
make deploy

if [[ ! -f "$STATE_FILE" ]]; then
  echo "Creating owner account (make setup)…"
  make setup
fi
# shellcheck disable=SC1090
source "$STATE_FILE"
if [[ -z "${SIGNER_ID:-}" ]]; then
  echo "ERROR: SIGNER_ID missing in $STATE_FILE" >&2
  exit 1
fi

OWNER="$SIGNER_ID"
echo "Funding owner Public/$OWNER…"
lgs wallet topup --address "Public/$OWNER"

PROVIDER_FILE=".lez_payment_streams-fixture-provider"
if [[ -f "$PROVIDER_FILE" ]]; then
  PROVIDER="$(cat "$PROVIDER_FILE")"
else
  PROVIDER="$(wallet account new public 2>&1 | sed -n 's/.*Public\/\([A-Za-z0-9]*\).*/\1/p')"
  echo "$PROVIDER" > "$PROVIDER_FILE"
fi
echo "Stream provider account: $PROVIDER"

SKIP_EXTRA=()
if [[ "$FORCE" == "1" ]]; then
  SKIP_EXTRA=(--force)
fi

echo "Seeding on-chain demo vault (vault_id=0, stream_id=0)…"
cargo run --quiet --manifest-path examples/Cargo.toml --bin seed_localnet_fixture -- seed-onchain \
  --program-bin "$PROGRAM_BIN" \
  --owner "$OWNER" \
  --provider "$PROVIDER" \
  "${SKIP_EXTRA[@]}" \
  --write-manifest "$MANIFEST"

echo "Done. Manifest: $MANIFEST"
echo "Fresh blank slate: ./scripts/demo-localnet-fresh.sh"
echo "Reset chain only: lgs localnet stop && rm -rf .scaffold/state/ && rm -f $STATE_FILE $PROVIDER_FILE $MANIFEST"
