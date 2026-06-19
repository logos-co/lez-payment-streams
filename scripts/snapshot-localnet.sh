#!/usr/bin/env bash
# Step 17b — capture pre-stream funded localnet baseline (sequencer must be stopped).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

SNAPSHOT_NAME="${1:-funded}"
SNAP_DIR="$REPO_ROOT/.scaffold/snapshots/$SNAPSHOT_NAME"
RESTART="${SNAPSHOT_RESTART:-1}"

# shellcheck source=scripts/localnet-snapshot-common.sh
source "$REPO_ROOT/scripts/localnet-snapshot-common.sh"

LEZ_CACHE="$(localnet_snapshot_lez_cache "$REPO_ROOT")"
ROCKSDB="$(localnet_snapshot_rocksdb_dir "$REPO_ROOT")"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: '$1' not on PATH" >&2
    exit 1
  }
}

require_cmd lgs

echo "=== snapshot localnet ($SNAPSHOT_NAME) ==="

if lgs localnet status 2>/dev/null | grep -qi running; then
  echo "Stopping localnet before copying RocksDB…"
  lgs localnet stop
fi

if [[ -f "$ROCKSDB/LOCK" ]]; then
  echo "ERROR: RocksDB LOCK still present at $ROCKSDB — stop the sequencer and retry" >&2
  exit 1
fi

if [[ ! -d "$ROCKSDB" ]]; then
  echo "ERROR: missing ledger at $ROCKSDB (run prefund / seed first)" >&2
  exit 1
fi

rm -rf "$SNAP_DIR"
mkdir -p "$SNAP_DIR"

echo "Copying rocksdb from $ROCKSDB …"
cp -a "$ROCKSDB" "$SNAP_DIR/rocksdb"

if [[ -d "$REPO_ROOT/.scaffold/wallet" ]]; then
  cp -a "$REPO_ROOT/.scaffold/wallet" "$SNAP_DIR/wallet"
fi
if [[ -d "$REPO_ROOT/.scaffold/state" ]]; then
  cp -a "$REPO_ROOT/.scaffold/state" "$SNAP_DIR/state"
fi

for f in .lez_payment_streams-state .lez_payment_streams-fixture-provider; do
  if [[ -f "$REPO_ROOT/$f" ]]; then
    cp -a "$REPO_ROOT/$f" "$SNAP_DIR/"
  fi
done

# Baseline is pre-stream; per-run manifest is written by create-stream-onchain after restore.

localnet_snapshot_write_metadata "$REPO_ROOT" "$SNAP_DIR"
echo "Wrote $SNAP_DIR/snapshot.json"

if [[ "$RESTART" == "1" ]]; then
  "$REPO_ROOT/scripts/ensure-scaffold-lez-layout.sh"
  lgs localnet start
fi

echo "=== snapshot done: $SNAP_DIR ==="
