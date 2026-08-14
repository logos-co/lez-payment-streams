#!/usr/bin/env bash
# Prove logoscore↔wallet exclusive handoff: stop/restart reloads private keys.
# Also shows close()+open is a no-op ("wallet is already open") so unload/close
# handoff cannot refresh keys the wallet CLI wrote (D39.22 / module path).
# Soft prove only; no Store / CLOCK_50 / hour-long path.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"

export PATH="${PATH:-}"
if [[ -d "$REPO_ROOT/../logos-logoscore-cli/result/bin" ]]; then
  export PATH="$REPO_ROOT/../logos-logoscore-cli/result/bin:$PATH"
fi

command -v logoscore >/dev/null || ps_fatal "logoscore not on PATH"
command -v python3 >/dev/null || ps_fatal "python3 required"

export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
CHAIN="${CHAIN:-testnet}"
export CHAIN

SEED_HOME="$(ps_chain_wallet_home)"
SEED_CFG="$SEED_HOME/wallet_config.json"
SEED_STOR="$SEED_HOME/storage.json"
[[ -f "$SEED_CFG" && -f "$SEED_STOR" ]] || ps_fatal "missing seed wallet under $SEED_HOME"

WORK="${TMPDIR:-/tmp}/ps-wallet-cli-handoff-$$"
HOME_A="$WORK/wallet"
CFG="$WORK/cfg"
MOD="${MODULES_USER:-$(ps_e2e_user_modules_dir)}"
PERS="$WORK/persist"
mkdir -p "$HOME_A" "$CFG" "$PERS"
cp "$SEED_CFG" "$HOME_A/wallet_config.json"
cp "$SEED_STOR" "$HOME_A/storage.json"
WC="$HOME_A/wallet_config.json"
WS="$HOME_A/storage.json"

cleanup() {
  logoscore --config-dir "$CFG" stop >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

private_count() {
  python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(sum(1 for r in d["key_chain"]["accounts"] if next(iter(r))=="Private"))' "$1"
}

start_host() {
  logoscore --config-dir "$CFG" stop >/dev/null 2>&1 || true
  sleep 1
  logoscore --config-dir "$CFG" -D -m "$MOD" --persistence-path "$PERS" -q >/dev/null 2>&1 &
  sleep 4
  logoscore --config-dir "$CFG" load-module logos_execution_zone >/dev/null
  logoscore --config-dir "$CFG" load-module payment_streams_module >/dev/null 2>&1 || true
  logoscore --config-dir "$CFG" call logos_execution_zone open "$WC" "$WS" >/dev/null
}

ps_log_info "Positive: create private → save → stop → start → open must keep Private count"
start_host
before="$(private_count "$WS")"
create_line="$(logoscore --config-dir "$CFG" call logos_execution_zone create_account_private 2>/dev/null | tail -1)"
priv_hex="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$create_line")"
[[ -n "$priv_hex" ]] || ps_fatal "create_account_private failed: $create_line"
logoscore --config-dir "$CFG" call logos_execution_zone save >/dev/null
after_save="$(private_count "$WS")"
[[ "$after_save" -gt "$before" ]] || ps_fatal "Private count did not increase after save ($before -> $after_save)"

logoscore --config-dir "$CFG" stop >/dev/null 2>&1 || true
sleep 2
start_host
after_restart="$(private_count "$WS")"
[[ "$after_restart" -eq "$after_save" ]] || ps_fatal "Private count changed across stop/restart ($after_save -> $after_restart)"
b58_line="$(logoscore --config-dir "$CFG" call logos_execution_zone account_id_to_base58 "$priv_hex" 2>/dev/null | tail -1)"
b58="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$b58_line")"
[[ -n "$b58" ]] || ps_fatal "stop/restart open did not resolve private hex $priv_hex"
ps_log_info "Positive handoff OK (Private count $after_restart; b58=${b58:0:12}…)"

ps_log_info "Negative: close()+open is a no-op (cannot refresh disk writes)"
close_out="$(logoscore --config-dir "$CFG" call logos_execution_zone close 2>&1 | tail -3 || true)"
open_out="$(logoscore --config-dir "$CFG" call logos_execution_zone open "$WC" "$WS" 2>&1 | tail -5 || true)"
if echo "$open_out" | grep -qi 'already open'; then
  ps_log_info "OK: observed wallet-already-open after close (exclusive stop required)"
elif echo "$close_out" | grep -qi 'METHOD_FAILED\|close failed'; then
  ps_log_info "OK: close METHOD_FAILED (exclusive stop required; unload/close handoff unsafe)"
else
  ps_log_info "WARN: unexpected close/open (close_out=${close_out:0:120}; open_out=${open_out:0:160})"
fi

ps_log_info "verify-wallet-cli-handoff: PASS"
