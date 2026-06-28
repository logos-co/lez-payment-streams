#!/usr/bin/env bash
# Step 24c — wait for on-chain Clock10 to catch up to wall time after snapshot restore.
#
# Local sequencer only folds blocks when the mempool has user txs; idle polling never
# advances Clock10. We nudge with pinata top-ups until skew is within tolerance.
#
# Usage: wait-clock-synced.sh
#   SEQUENCER_URL     optional (wallet config is authoritative)
#   MAX_CLOCK_SKEW_S  max allowed wall-minus-clock skew (default 5)
#   CLOCK_SYNC_TIMEOUT_S  give up after this many seconds (default 120)
#   CLOCK_SYNC_POLL_S     poll interval (default 2)
#   CLOCK_SYNC_NUDGE_ROUNDS  pinata nudges before hard fail (default 20)
#   SKIP_CLOCK_SYNC=1   no-op
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if [[ "${SKIP_CLOCK_SYNC:-0}" == "1" ]]; then
  exit 0
fi

LEZ_PIN="$(grep -A2 '\[repos.lez\]' scaffold.toml | grep '^pin' | sed 's/.*"\([^"]*\)".*/\1/')"
export PATH="$HOME/.cache/logos-scaffold/repos/lez/${LEZ_PIN}/target/release:$PATH"
export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}"

MAX_SKEW="${MAX_CLOCK_SKEW_S:-5}"
TIMEOUT_S="${CLOCK_SYNC_TIMEOUT_S:-120}"
POLL_S="${CLOCK_SYNC_POLL_S:-2}"
NUDGE_ROUNDS="${CLOCK_SYNC_NUDGE_ROUNDS:-20}"
SEQ_URL="${SEQUENCER_URL:-http://127.0.0.1:3040}"

run_wait() {
  local timeout_s="$1"
  cargo run --quiet --manifest-path "$REPO_ROOT/examples/Cargo.toml" --bin seed_localnet_fixture -- \
    wait-clock-synced \
    --max-skew-s "$MAX_SKEW" \
    --timeout-s "$timeout_s" \
    --poll-s "$POLL_S" \
    --sequencer-url "$SEQ_URL"
}

OWNER=""
if [[ -f "$REPO_ROOT/.lez_payment_streams-state" ]]; then
  # shellcheck disable=SC1090
  source "$REPO_ROOT/.lez_payment_streams-state"
  OWNER="${SIGNER_ID:-}"
fi

if run_wait 3; then
  exit 0
fi

if [[ -z "$OWNER" ]] || ! command -v lgs >/dev/null 2>&1; then
  echo "wait-clock-synced: no owner or lgs; retrying long poll only" >&2
  run_wait "$TIMEOUT_S"
  exit $?
fi

echo "wait-clock-synced: nudging block production (pinata) until Clock10 catches up…" >&2
for ((round = 1; round <= NUDGE_ROUNDS; round++)); do
  lgs wallet topup --address "Public/$OWNER" >/dev/null 2>&1 || true
  if run_wait 8; then
    exit 0
  fi
done

echo "wait-clock-synced: final wait (${TIMEOUT_S}s)…" >&2
run_wait "$TIMEOUT_S"
