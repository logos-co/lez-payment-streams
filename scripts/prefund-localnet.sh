#!/usr/bin/env bash
# Step 17b stage A — blank ledger, pinata, prefund baseline, snapshot (no stream).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/localnet-snapshot-common.sh
source "$REPO_ROOT/scripts/localnet-snapshot-common.sh"

export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}"
ROCKSDB="$(localnet_snapshot_rocksdb_dir "$REPO_ROOT")"
SNAPSHOT_NAME="${1:-funded}"

echo "=== prefund localnet (snapshot=$SNAPSHOT_NAME) ==="

"$REPO_ROOT/scripts/ensure-scaffold-lez-layout.sh"

if command -v lgs >/dev/null 2>&1; then
  lgs localnet stop 2>/dev/null || true
else
  echo "ERROR: lgs not on PATH" >&2
  exit 1
fi

if [[ -d "$ROCKSDB" ]]; then
  echo "Removing ledger at $ROCKSDB …"
  rm -rf "$ROCKSDB"
fi

if [[ -d .scaffold/state ]]; then
  rm -rf .scaffold/state/
fi

rm -f fixtures/localnet.json .lez_payment_streams-state .lez_payment_streams-state.tmp
rm -f .lez_payment_streams-fixture-provider

REINIT_WALLET="${REINIT_WALLET:-0}"
if [[ "$REINIT_WALLET" == "1" ]]; then
  "$REPO_ROOT/scripts/reinit-scaffold-wallet.sh"
fi

LEZ_PIN="$(localnet_snapshot_lez_pin "$REPO_ROOT")"
export PATH="$HOME/.cache/logos-scaffold/repos/lez/${LEZ_PIN}/target/release:$PATH"

PROGRAM_BIN="methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
STATE_FILE=".lez_payment_streams-state"
PROVIDER_FILE=".lez_payment_streams-fixture-provider"

TOPUP_ROUNDS="${SEED_WALLET_TOPUP_ROUNDS:-}"
DEPOSIT_AMOUNT="${SEED_DEPOSIT_AMOUNT:-2000}"
if [[ -z "$TOPUP_ROUNDS" ]]; then
  TOPUP_ROUNDS=$(( (DEPOSIT_AMOUNT + 149) / 150 + 4 ))
fi

lgs init 2>/dev/null || true
lgs setup
lgs localnet start

echo "Building and deploying guest…"
make build
make deploy
make setup
# shellcheck disable=SC1090
source "$STATE_FILE"
OWNER="$SIGNER_ID"

echo "Funding owner (pinata ${TOPUP_ROUNDS} rounds)…"
for ((round = 1; round <= TOPUP_ROUNDS; round++)); do
  echo "  topup round ${round}/${TOPUP_ROUNDS}…"
  lgs wallet topup --address "Public/$OWNER"
done

if [[ -f "$PROVIDER_FILE" ]]; then
  PROVIDER="$(cat "$PROVIDER_FILE")"
else
  if ! command -v wallet >/dev/null 2>&1; then
    echo "ERROR: wallet not on PATH (run lgs setup from this repo)" >&2
    exit 1
  fi
  PROVIDER="$(wallet account new public 2>&1 | sed -n 's/.*Public\/\([A-Za-z0-9]*\).*/\1/p')"
  if [[ -z "$PROVIDER" ]]; then
    echo "ERROR: failed to parse provider id from 'wallet account new public' output" >&2
    exit 1
  fi
  echo "$PROVIDER" > "$PROVIDER_FILE"
fi

echo "Prefunding vault 0 (no stream)…"
cargo run --quiet --manifest-path examples/Cargo.toml --bin seed_localnet_fixture -- prefund-onchain \
  --program-bin "$PROGRAM_BIN" \
  --owner "$OWNER" \
  --deposit-amount "$DEPOSIT_AMOUNT"

DEPOSIT_AMOUNT="$DEPOSIT_AMOUNT" FIXTURE_MANIFEST="$REPO_ROOT/fixtures/localnet.json" \
  "$REPO_ROOT/scripts/write-vault-manifest.sh"

lgs localnet stop
SNAPSHOT_RESTART=0 "$REPO_ROOT/scripts/snapshot-localnet.sh" "$SNAPSHOT_NAME"

echo "=== prefund done: .scaffold/snapshots/$SNAPSHOT_NAME ==="
