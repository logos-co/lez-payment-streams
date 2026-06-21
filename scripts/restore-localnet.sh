#!/usr/bin/env bash
# Step 17b — restore funded baseline from snapshot (then run create-stream for per-run fixture).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

SNAPSHOT_NAME="${1:-funded}"
SNAP_DIR="$REPO_ROOT/.scaffold/snapshots/$SNAPSHOT_NAME"

# shellcheck source=scripts/localnet-snapshot-common.sh
source "$REPO_ROOT/scripts/localnet-snapshot-common.sh"

export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}"
ROCKSDB="$(localnet_snapshot_rocksdb_dir "$REPO_ROOT")"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: '$1' not on PATH" >&2
    exit 1
  }
}

require_cmd lgs
require_cmd wallet

echo "=== restore localnet ($SNAPSHOT_NAME) ==="

if [[ ! -d "$SNAP_DIR" ]]; then
  echo "ERROR: no snapshot at $SNAP_DIR" >&2
  exit 1
fi

localnet_snapshot_validate_metadata "$REPO_ROOT" "$SNAP_DIR"

if lgs localnet status 2>/dev/null | grep -qi running; then
  echo "Stopping localnet…"
  lgs localnet stop
fi

port_free=0
for _ in 1 2 3 4 5; do
  if ! curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' >/dev/null 2>&1; then
    port_free=1
    break
  fi
  echo "Waiting for port 3040 to free…"
  sleep 1
done
if [[ "$port_free" != "1" ]]; then
  echo "ERROR: port 3040 still reachable after localnet stop; a foreign sequencer may be running" >&2
  exit 1
fi

if [[ -d "$ROCKSDB" ]]; then
  rm -rf "$ROCKSDB"
fi
mkdir -p "$(dirname "$ROCKSDB")"
cp -a "$SNAP_DIR/rocksdb" "$ROCKSDB"

if [[ -d "$SNAP_DIR/wallet" ]]; then
  rm -rf "$REPO_ROOT/.scaffold/wallet"
  mkdir -p "$REPO_ROOT/.scaffold"
  cp -a "$SNAP_DIR/wallet" "$REPO_ROOT/.scaffold/wallet"
fi
if [[ -d "$SNAP_DIR/state" ]]; then
  rm -rf "$REPO_ROOT/.scaffold/state"
  mkdir -p "$REPO_ROOT/.scaffold"
  cp -a "$SNAP_DIR/state" "$REPO_ROOT/.scaffold/state"
fi

for f in .lez_payment_streams-state .lez_payment_streams-fixture-provider; do
  if [[ -f "$SNAP_DIR/$f" ]]; then
    cp -a "$SNAP_DIR/$f" "$REPO_ROOT/$f"
  fi
done

rm -f "$REPO_ROOT/fixtures/localnet.json"

"$REPO_ROOT/scripts/ensure-scaffold-lez-layout.sh"
lgs localnet start

if ! lgs wallet -- check-health >/dev/null 2>&1; then
  echo "WARN: wallet check-health failed after restore (see step10a troubleshooting)" >&2
fi

echo "=== restore done (create stream before Step 10a verify) ==="
