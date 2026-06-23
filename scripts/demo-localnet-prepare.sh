#!/usr/bin/env bash
# Step 17b — prepare localnet for demos: restore baseline + fresh stream, or rebuild snapshot.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

FULL_RESET="${FULL_RESET:-0}"
SKIP_VERIFY="${SKIP_VERIFY:-0}"
SNAPSHOT_NAME="${SNAPSHOT_NAME:-funded}"
SNAP_DIR="$REPO_ROOT/.scaffold/snapshots/$SNAPSHOT_NAME"

# shellcheck source=scripts/localnet-snapshot-common.sh
source "$REPO_ROOT/scripts/localnet-snapshot-common.sh"

echo "=== demo localnet prepare (FULL_RESET=$FULL_RESET) ==="

if [[ "$FULL_RESET" == "1" ]]; then
  "$REPO_ROOT/scripts/prefund-localnet.sh" "$SNAPSHOT_NAME"
else
  if [[ -f "$SNAP_DIR/snapshot.json" ]] && localnet_snapshot_validate_metadata "$REPO_ROOT" "$SNAP_DIR"; then
    "$REPO_ROOT/scripts/restore-localnet.sh" "$SNAPSHOT_NAME"
  else
    echo "No valid snapshot at $SNAP_DIR — running prefund (one-time stage A)…"
    "$REPO_ROOT/scripts/prefund-localnet.sh" "$SNAPSHOT_NAME"
    "$REPO_ROOT/scripts/restore-localnet.sh" "$SNAPSHOT_NAME"
  fi
fi

if [[ "${SKIP_STREAM_CREATE:-0}" != "1" ]]; then
  "$REPO_ROOT/scripts/create-localnet-stream-fixture.sh"
else
  echo "SKIP_STREAM_CREATE=1 — vault baseline only (stream created in E2E orchestrator)"
fi

if [[ "$SKIP_VERIFY" == "1" ]]; then
  echo "SKIP_VERIFY=1 — skipping verify-step10a-dod.sh"
else
  echo "--- verify step 10a ---"
  "$REPO_ROOT/scripts/verify-step10a-dod.sh"
fi

echo "=== prepare done ==="
