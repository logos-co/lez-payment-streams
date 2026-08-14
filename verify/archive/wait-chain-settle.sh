#!/usr/bin/env bash
# Step 24c — wait for the local sequencer to settle the owner account before a seed submit.
#
# Background: seed_localnet_fixture fetches the signer nonce fresh from the sequencer
# (getAccount.nonce, the committed value). A logoscore smoke that submitted an owner
# chainAction can leave that tx pending (accepted, not yet folded) when logoscore stops.
# The seed then fetches the pre-fold nonce, submits a duplicate, and the sequencer drops
# it ("Transaction not found in preconfigured amount of blocks"). Waiting a couple blocks
# lets any pending owner tx fold so the committed nonce is authoritative before the seed runs.
#
# Usage: wait-chain-settle.sh [OWNER_ACCOUNT_ID]
#   SEQUENCER_URL    sequencer JSON-RPC endpoint (default http://127.0.0.1:3040)
#   FIXTURE_MANIFEST manifest to read owner/sequencer from when args/env are unset
#   SETTLE_BLOCKS    blocks to advance before returning (default 2)
#   SETTLE_TIMEOUT_S best-effort cap; returns 0 even on timeout (default 90)
#   SETTLE_POLL_S    poll interval seconds (default 2)
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
SEQ_URL="${SEQUENCER_URL:-}"
OWNER="${1:-${OWNER:-}}"
SETTLE_BLOCKS="${SETTLE_BLOCKS:-2}"
SETTLE_TIMEOUT_S="${SETTLE_TIMEOUT_S:-90}"
SETTLE_POLL_S="${SETTLE_POLL_S:-2}"

if [[ "${SKIP_CHAIN_SETTLE:-0}" == "1" ]]; then
  exit 0
fi

if [[ -z "$SEQ_URL" && -f "$MANIFEST" ]]; then
  SEQ_URL="$(python3 -c "import json; print(json.load(open('$MANIFEST')).get('sequencer_url',''))" 2>/dev/null || true)"
fi
SEQ_URL="${SEQ_URL:-http://127.0.0.1:3040}"

if [[ -z "$OWNER" ]]; then
  if [[ -f "$REPO_ROOT/.lez_payment_streams-state" ]]; then
    # shellcheck disable=SC1090
    source "$REPO_ROOT/.lez_payment_streams-state"
    OWNER="${SIGNER_ID:-}"
  fi
fi
if [[ -z "$OWNER" && -f "$MANIFEST" ]]; then
  OWNER="$(python3 -c "import json; print(json.load(open('$MANIFEST')).get('owner_account_id',''))" 2>/dev/null || true)"
fi

_rpc() {
  curl -sf -X POST "$SEQ_URL" -H 'Content-Type: application/json' -d "$1" 2>/dev/null || true
}

_last_block() {
  _rpc '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' \
    | python3 -c 'import json,sys
try:
  d=json.load(sys.stdin); r=d.get("result")
  print(r if isinstance(r,int) else "")
except Exception:
  print("")' 2>/dev/null || true
}

_owner_nonce() {
  [[ -z "$OWNER" ]] && { echo ""; return; }
  _rpc "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$OWNER\"]}" \
    | python3 -c 'import json,sys
try:
  d=json.load(sys.stdin); r=d.get("result") or {}
  print(r.get("nonce",""))
except Exception:
  print("")' 2>/dev/null || true
}

start_block="$(_last_block)"
if [[ -z "$start_block" ]]; then
  echo "settle: sequencer unreachable at $SEQ_URL; skipping wait" >&2
  exit 0
fi
start_nonce="$(_owner_nonce)"
echo "settle: start block=$start_block owner_nonce=${start_nonce:-?} (need +${SETTLE_BLOCKS} blocks)" >&2

deadline=$(( $(date +%s) + SETTLE_TIMEOUT_S ))
last_nonce="$start_nonce"
stable_reads=0
while true; do
  now=$(date +%s)
  cur_block="$(_last_block)"
  cur_nonce="$(_owner_nonce)"
  if [[ -n "$cur_nonce" && "$cur_nonce" != "$last_nonce" ]]; then
    last_nonce="$cur_nonce"
    stable_reads=0
  else
    stable_reads=$((stable_reads + 1))
  fi
  if [[ -n "$cur_block" && -n "$start_block" ]] \
    && (( cur_block - start_block >= SETTLE_BLOCKS )) \
    && (( stable_reads >= 2 )); then
    echo "settle: settled block=$cur_block owner_nonce=${cur_nonce:-?}" >&2
    exit 0
  fi
  if (( now >= deadline )); then
    echo "settle: timeout after ${SETTLE_TIMEOUT_S}s (block=$cur_block owner_nonce=${cur_nonce:-?}); proceeding" >&2
    exit 0
  fi
  sleep "$SETTLE_POLL_S"
done
