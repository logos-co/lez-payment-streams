#!/usr/bin/env bash
# Sync testnet seed wallet to tip and burn private derivation slots so the next
# Store privacy E2E create_account_private pair is not a recycled id (DaV7/FqxTy).
# Soft logoscore path only for create/save; no PPE prove.
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

# Before ps_chain_wallet_home / module dirs (defaults to local without this).
export CHAIN="${CHAIN:-testnet}"

SEED_HOME="$(ps_chain_wallet_home)"
SEED_CFG="$SEED_HOME/wallet_config.json"
SEED_STOR="$SEED_HOME/storage.json"
[[ -f "$SEED_CFG" && -f "$SEED_STOR" ]] || ps_fatal "missing seed wallet under $SEED_HOME"

BURN_COUNT="${PS_PRIVACY_SEED_BURN_COUNT:-4}"
RECYCLED_REGEX='^(DaV7bT45|FqxTyJhY|8vSpcfHE)'

WORK="${TMPDIR:-/tmp}/ps-privacy-seed-$$"
CFG="$WORK/cfg"
MOD="${MODULES_USER:-$(ps_e2e_user_modules_dir)}"
PERS="$WORK/persist"
mkdir -p "$SEED_HOME" "$CFG" "$PERS"

cleanup() {
  logoscore --config-dir "$CFG" stop >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

cp "$SEED_CFG" "$WORK/wallet_config.json"
cp "$SEED_STOR" "$WORK/storage.json"
WC="$WORK/wallet_config.json"
WS="$WORK/storage.json"

ps_export_wallet_home "$WORK"

ps_log_info "Syncing seed copy to tip (wallet account sync-private)"
# sync-private advances last_synced_block and discovers notes for known keys.
wallet account sync-private >/dev/null
python3 - <<'PY' "$WS"
import json, sys
from pathlib import Path
p = Path(sys.argv[1])
d = json.loads(p.read_text())
print(f"last_synced_block={d.get('last_synced_block')}")
PY

private_ids() {
  python3 - <<'PY' "$WS"
import json, sys
d = json.load(open(sys.argv[1]))
for row in d["key_chain"]["accounts"]:
    if "Private" in row:
        print(row["Private"]["account_id"])
PY
}

start_host() {
  logoscore --config-dir "$CFG" stop >/dev/null 2>&1 || true
  sleep 1
  logoscore --config-dir "$CFG" -D -m "$MOD" --persistence-path "$PERS" -q >/dev/null 2>&1 &
  sleep 4
  logoscore --config-dir "$CFG" load-module logos_execution_zone >/dev/null
  logoscore --config-dir "$CFG" call logos_execution_zone open "$WC" "$WS" >/dev/null
}

ps_log_info "Burning $BURN_COUNT private derivation slots on seed"
start_host
before="$(private_ids | wc -l | tr -d ' ')"
burned=()
for ((i = 1; i <= BURN_COUNT; i++)); do
  create_line="$(logoscore --config-dir "$CFG" call logos_execution_zone create_account_private 2>/dev/null | tail -1)"
  priv_hex="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$create_line")"
  [[ -n "$priv_hex" ]] || ps_fatal "create_account_private failed: $create_line"
  b58_line="$(logoscore --config-dir "$CFG" call logos_execution_zone account_id_to_base58 "$priv_hex" 2>/dev/null | tail -1)"
  b58="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$b58_line")"
  burned+=("$b58")
  ps_log_info "  burned[$i]=${b58:0:12}…"
done
logoscore --config-dir "$CFG" call logos_execution_zone save >/dev/null
logoscore --config-dir "$CFG" stop >/dev/null 2>&1 || true
sleep 1

# Persist bumped+synced storage back to seed.
cp "$WS" "$SEED_STOR"
cp "$WC" "$SEED_CFG"

after="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(sum(1 for r in d["key_chain"]["accounts"] if "Private" in r))' "$SEED_STOR")"
synced="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("last_synced_block"))' "$SEED_STOR")"
[[ "$after" -ge $((before + BURN_COUNT)) ]] || ps_fatal "seed private count did not grow ($before -> $after)"

# Next create after clone must not be a known recycled id (probe once on a throwaway clone).
PROBE="$WORK/probe"
mkdir -p "$PROBE/cfg" "$PROBE/persist" "$PROBE/wallet"
cp "$SEED_CFG" "$PROBE/wallet/wallet_config.json"
cp "$SEED_STOR" "$PROBE/wallet/storage.json"
logoscore --config-dir "$PROBE/cfg" -D -m "$MOD" --persistence-path "$PROBE/persist" -q >/dev/null 2>&1 &
sleep 4
logoscore --config-dir "$PROBE/cfg" load-module logos_execution_zone >/dev/null
logoscore --config-dir "$PROBE/cfg" call logos_execution_zone open \
  "$PROBE/wallet/wallet_config.json" "$PROBE/wallet/storage.json" >/dev/null
next_line="$(logoscore --config-dir "$PROBE/cfg" call logos_execution_zone create_account_private 2>/dev/null | tail -1)"
next_hex="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$next_line")"
next_b58="$(logoscore --config-dir "$PROBE/cfg" call logos_execution_zone account_id_to_base58 "$next_hex" 2>/dev/null | tail -1 \
  | python3 -c 'import json,sys; o=json.loads(sys.stdin.read()); r=o.get("result",""); print(r if isinstance(r,str) else "")')"
logoscore --config-dir "$PROBE/cfg" stop >/dev/null 2>&1 || true
if [[ "$next_b58" =~ $RECYCLED_REGEX ]]; then
  ps_fatal "next private id still recycled ($next_b58); raise PS_PRIVACY_SEED_BURN_COUNT and re-run"
fi

ps_log_info "Seed ready: privates=$after last_synced_block=$synced next_create=${next_b58:0:12}… (not recycled)"
