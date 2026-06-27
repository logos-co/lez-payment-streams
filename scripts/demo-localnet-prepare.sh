#!/usr/bin/env bash
# Step 17b / 24c — restore vault baseline only; per-run stream is created in E2E orchestrator.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export FIXTURE_MANIFEST="$REPO_ROOT/fixtures/localnet.json"

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
    if localnet_snapshot_stale_for_restore "$SNAP_DIR"; then
      echo "Snapshot older than SNAPSHOT_MAX_AGE_S (${SNAPSHOT_MAX_AGE_S:-1800}) — rebuilding funded baseline (clock drift)…"
      "$REPO_ROOT/scripts/prefund-localnet.sh" "$SNAPSHOT_NAME"
    fi
    "$REPO_ROOT/scripts/restore-localnet.sh" "$SNAPSHOT_NAME"
  else
    echo "No valid snapshot at $SNAP_DIR — running prefund (one-time stage A)…"
    "$REPO_ROOT/scripts/prefund-localnet.sh" "$SNAPSHOT_NAME"
    "$REPO_ROOT/scripts/restore-localnet.sh" "$SNAPSHOT_NAME"
  fi
fi

if [[ "${SKIP_STREAM_CREATE:-1}" != "1" ]]; then
  "$REPO_ROOT/scripts/create-localnet-stream-fixture.sh"
else
  echo "SKIP_STREAM_CREATE=1 — vault baseline only (stream created in E2E orchestrator)"
  DEPOSIT_AMOUNT="${SEED_DEPOSIT_AMOUNT:-2000}" FIXTURE_MANIFEST="$FIXTURE_MANIFEST" \
    "$REPO_ROOT/scripts/write-vault-manifest.sh"
fi

if [[ "$SKIP_VERIFY" == "1" ]]; then
  echo "SKIP_VERIFY=1 — skipping verify-step10a-dod.sh"
else
  echo "--- verify step 10a ---"
  "$REPO_ROOT/scripts/verify-step10a-dod.sh"
fi

echo "=== prepare done ==="
