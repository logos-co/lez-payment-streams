#!/usr/bin/env bash
# D39.26 includability smoke: pin wallet auth-transfer send amount=1 → getTransaction
# non-null under RISC0_DEV_MODE=0. Uses a throwaway clone of the testnet seed.
# Fail-fast if TX hash appears but tip advances without inclusion.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"

export PATH="${PATH:-}"
if [[ -d "$REPO_ROOT/../logos-logoscore-cli/result/bin" ]]; then
  export PATH="$REPO_ROOT/../logos-logoscore-cli/result/bin:$PATH"
fi
ps_prepend_lez_wallet_path

command -v logoscore >/dev/null || ps_fatal "logoscore not on PATH"
command -v wallet >/dev/null || ps_fatal "wallet not on PATH (pinned LEZ release)"
command -v python3 >/dev/null || ps_fatal "python3 required"
command -v curl >/dev/null || ps_fatal "curl required"

# Before ps_chain_wallet_home / module dirs (defaults to local without this).
export CHAIN="${CHAIN:-testnet}"
export RISC0_DEV_MODE=0

SEED_HOME="$(ps_chain_wallet_home)"
SEED_CFG="$SEED_HOME/wallet_config.json"
SEED_STOR="$SEED_HOME/storage.json"
[[ -f "$SEED_CFG" && -f "$SEED_STOR" ]] || ps_fatal "missing seed under $SEED_HOME"

SEQ_URL="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("sequencer_addr","https://testnet.lez.logos.co/"))' "$SEED_CFG")"
FUNDER="${PS_PPE_SMOKE_FUNDER:-$(python3 -c 'import json; print(json.load(open("'"$REPO_ROOT"'/fixtures/testnet-module.json"))["owner_account_id"])')}"
TIP_BLOCKS_AFTER_HASH="${PS_PPE_ORPHAN_TIP_BLOCKS:-8}"
POST_HASH_DEADLINE_S="${PS_PPE_ORPHAN_DEADLINE_S:-240}"
PROVE_DEADLINE_S="${PS_PPE_PROVE_DEADLINE_S:-600}"

WORK="${TMPDIR:-/tmp}/ps-ppe-smoke-$$"
CFG="$WORK/cfg"
MOD="${MODULES_USER:-$(ps_e2e_user_modules_dir)}"
PERS="$WORK/persist"
WHOME="$WORK/wallet"
mkdir -p "$CFG" "$PERS" "$WHOME"
cp "$SEED_CFG" "$WHOME/wallet_config.json"
cp "$SEED_STOR" "$WHOME/storage.json"
WC="$WHOME/wallet_config.json"
WS="$WHOME/storage.json"

cleanup() {
  logoscore --config-dir "$CFG" stop >/dev/null 2>&1 || true
  kill "${WALLET_PID:-}" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

tip() {
  curl -sf -X POST "$SEQ_URL" -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' \
    | python3 -c 'import json,sys; print(json.load(sys.stdin).get("result",""))'
}

tx_found() {
  local h="$1"
  curl -sf -X POST "$SEQ_URL" -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getTransaction\",\"params\":[\"$h\"]}" \
    | python3 -c 'import json,sys; print("1" if json.load(sys.stdin).get("result") is not None else "0")'
}

ps_log_info "PPE include smoke: funder=${FUNDER:0:12}… amount=1 RISC0_DEV_MODE=0"
logoscore --config-dir "$CFG" -D -m "$MOD" --persistence-path "$PERS" -q >/dev/null 2>&1 &
sleep 4
logoscore --config-dir "$CFG" load-module logos_execution_zone >/dev/null
logoscore --config-dir "$CFG" call logos_execution_zone open "$WC" "$WS" >/dev/null
create_line="$(logoscore --config-dir "$CFG" call logos_execution_zone create_account_private 2>/dev/null | tail -1)"
priv_hex="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$create_line")"
[[ -n "$priv_hex" ]] || ps_fatal "create_account_private failed: $create_line"
to_b58="$(logoscore --config-dir "$CFG" call logos_execution_zone account_id_to_base58 "$priv_hex" 2>/dev/null | tail -1 \
  | python3 -c 'import json,sys; o=json.loads(sys.stdin.read()); r=o.get("result",""); print(r if isinstance(r,str) else "")')"
[[ -n "$to_b58" ]] || ps_fatal "account_id_to_base58 failed"
logoscore --config-dir "$CFG" call logos_execution_zone save >/dev/null
logoscore --config-dir "$CFG" stop >/dev/null 2>&1 || true
sleep 2

ps_export_wallet_home "$WHOME"
ps_log_info "Shielding 1 → Private/${to_b58:0:12}… (prove may take a few minutes)"
OUT="$WORK/wallet.out"
: >"$OUT"
wallet auth-transfer send \
  --from "Public/$FUNDER" \
  --to "Private/$to_b58" \
  --amount 1 >"$OUT" 2>&1 &
WALLET_PID=$!

hash=""
tip_at_hash=""
deadline=$((SECONDS + PROVE_DEADLINE_S))
while kill -0 "$WALLET_PID" 2>/dev/null; do
  if [[ -z "$hash" ]]; then
    hash="$(rg -o 'Transaction hash is ([0-9a-fA-F]{64})' -r '$1' "$OUT" 2>/dev/null | tail -1 || true)"
    if [[ -n "$hash" ]]; then
      tip_at_hash="$(tip)"
      ps_log_info "TX hash=$hash tip_at_hash=$tip_at_hash — watching inclusion (fail-fast orphan)"
      deadline=$((SECONDS + POST_HASH_DEADLINE_S))
    fi
  else
    if [[ "$(tx_found "$hash")" == "1" ]]; then
      kill "$WALLET_PID" 2>/dev/null || true
      wait "$WALLET_PID" 2>/dev/null || true
      ps_log_info "PPE include smoke PASS hash=$hash recipient=${to_b58:0:12}…"
      exit 0
    fi
    now_tip="$(tip)"
    if [[ -n "$tip_at_hash" && -n "$now_tip" ]]; then
      advanced=$((now_tip - tip_at_hash))
      if (( advanced >= TIP_BLOCKS_AFTER_HASH )); then
        kill "$WALLET_PID" 2>/dev/null || true
        wait "$WALLET_PID" 2>/dev/null || true
        ps_fatal "orphan PPE: hash=$hash still missing after tip +$advanced (from $tip_at_hash to $now_tip)"
      fi
    fi
  fi
  if (( SECONDS >= deadline )); then
    kill "$WALLET_PID" 2>/dev/null || true
    wait "$WALLET_PID" 2>/dev/null || true
    if [[ -n "$hash" ]]; then
      ps_fatal "orphan/slow PPE: hash=$hash not included within ${POST_HASH_DEADLINE_S}s after hash"
    fi
    ps_fatal "PPE smoke: no TX hash within ${PROVE_DEADLINE_S}s (wallet still proving or hung)"
  fi
  sleep 5
done

wait "$WALLET_PID" || true
hash="${hash:-$(rg -o 'Transaction hash is ([0-9a-fA-F]{64})' -r '$1' "$OUT" 2>/dev/null | tail -1 || true)}"
[[ -n "$hash" ]] || ps_fatal "wallet exited without TX hash; log: $(tail -c 500 "$OUT")"
for _ in $(seq 1 30); do
  if [[ "$(tx_found "$hash")" == "1" ]]; then
    ps_log_info "PPE include smoke PASS hash=$hash recipient=${to_b58:0:12}…"
    exit 0
  fi
  sleep 2
done
ps_fatal "orphan PPE after wallet exit: hash=$hash getTransaction still null"
