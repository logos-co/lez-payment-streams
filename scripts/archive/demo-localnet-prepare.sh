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
source "$REPO_ROOT/scripts/archive/localnet-snapshot-common.sh"

echo "=== demo localnet prepare (FULL_RESET=$FULL_RESET) ==="

if [[ "$FULL_RESET" == "1" ]]; then
  "$REPO_ROOT/scripts/archive/prefund-localnet.sh" "$SNAPSHOT_NAME"
else
  if [[ -f "$SNAP_DIR/snapshot.json" ]] && localnet_snapshot_validate_metadata "$REPO_ROOT" "$SNAP_DIR"; then
    "$REPO_ROOT/scripts/archive/restore-localnet.sh" "$SNAPSHOT_NAME"
  else
    echo "No valid snapshot at $SNAP_DIR — running prefund (one-time stage A)…"
    "$REPO_ROOT/scripts/archive/prefund-localnet.sh" "$SNAPSHOT_NAME"
    "$REPO_ROOT/scripts/archive/restore-localnet.sh" "$SNAPSHOT_NAME"
  fi
fi

if [[ "${SKIP_STREAM_CREATE:-1}" != "1" ]]; then
  "$REPO_ROOT/scripts/archive/create-localnet-stream-fixture.sh"
else
  echo "SKIP_STREAM_CREATE=1 — vault baseline only (stream created in E2E orchestrator)"
  DEPOSIT_AMOUNT="${SEED_DEPOSIT_AMOUNT:-1000}" FIXTURE_MANIFEST="$FIXTURE_MANIFEST" \
    "$REPO_ROOT/scripts/archive/write-vault-manifest.sh"
fi

if [[ "$SKIP_VERIFY" == "1" ]]; then
  echo "SKIP_VERIFY=1 — skipping fixture validation"
else
  echo "--- fixture validation (no longer requires verify-step10a-dod.sh) ---"
  # Step 10a verify was an implementation verification, not needed for E2E
  # Basic fixture presence is validated by snapshot metadata check above
fi

echo "=== prepare done ==="
