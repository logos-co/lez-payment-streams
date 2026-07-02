#!/usr/bin/env bash
# Shared authenticated_transfer ensure for payment-streams E2E.
# Requires ARTIFACT, LEE_WALLET_HOME_DIR (for wallet path), optional WALLET_CONFIG/STORAGE.

set -euo pipefail

[[ -n "${PS_AUTH_TRANSFER_SOURCED:-}" ]] && return 0
PS_AUTH_TRANSFER_SOURCED=1

_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$_LIB_DIR/common.sh"
# shellcheck source=scripts/lib/chain_poll.sh
source "$_LIB_DIR/chain_poll.sh"

ps_at_emit_phase() {
  local phase="$1" ok="$2" extra="${3:-{}}"
  [[ -n "${ARTIFACT:-}" ]] || ps_fatal "ARTIFACT not set for ps_at_emit_phase"
  mkdir -p "$(dirname "$ARTIFACT")"
  echo "{\"phase\":\"$phase\",\"ok\":$ok,\"extra\":$extra}" >> "$ARTIFACT"
}

ps_at_extract_field() {
  python3 -c '
import json,sys
try:
    outer=json.loads(sys.argv[1])
    inner=outer.get("result","{}")
    if isinstance(inner,str):
        inner=json.loads(inner) if inner.strip().startswith("{") else {}
    print(inner.get(sys.argv[2],""))
except Exception:
    pass
' "$1" "$2" 2>/dev/null
}

ps_at_sync_wallet() {
  local height
  height="$(curl -sf -X POST "$(ps_seq_url)" -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' |
    python3 -c 'import json,sys; d=json.load(sys.stdin); r=d.get("result"); print(r if isinstance(r,int) else (r or ""))' 2>/dev/null || true)"
  if [[ -n "$height" ]] && command -v logoscore >/dev/null 2>&1; then
    logoscore call logos_execution_zone sync_to_block "$height" >/dev/null 2>&1 || true
  fi
  sleep 3
}

ps_at_verify_extra() {
  local v="${PS_AT_VERIFY_MODE:-image_id}"
  python3 -c 'import json,sys; print(json.dumps({"verify": sys.argv[1]}))' "$v"
}

# When logoscore holds the same storage.json, release before standalone wallet CLI.
ps_at_logoscore_close_wallet() {
  [[ "${PS_AT_LOGOSCORE_WALLET_HANDOFF:-0}" == "1" ]] || return 0
  command -v logoscore >/dev/null 2>&1 || return 0
  logoscore call logos_execution_zone close >/dev/null 2>&1 || true
}

ps_at_logoscore_open_wallet() {
  [[ "${PS_AT_LOGOSCORE_WALLET_HANDOFF:-0}" == "1" ]] || return 0
  command -v logoscore >/dev/null 2>&1 || return 0
  [[ -n "${WALLET_CONFIG:-}" && -n "${WALLET_STORAGE:-}" ]] || return 0
  logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE" >/dev/null 2>&1 || true
  ps_at_sync_wallet
}

ps_at_logoscore_reachable() {
  command -v logoscore >/dev/null 2>&1 || return 1
  timeout 5 logoscore call logos_execution_zone save >/dev/null 2>&1
}

ps_at_logoscore_call() {
  timeout "${PS_AT_LOGOSCORE_TIMEOUT:-30}" logoscore "$@" 2>/dev/null
}

# ps_auth_transfer_init_one <base58> <phase_name auth_init_owner|auth_init_provider>
ps_auth_transfer_init_one() {
  local acct="$1" phase="$2"
  local verify_json via="on_chain" already="true" tx_hash="" err="" hint=""

  if ps_account_is_at_initialized "$acct"; then
    verify_json="$(ps_at_verify_extra)"
    ps_at_emit_phase "$phase" true "$(python3 -c 'import json,sys; e={"account_id":sys.argv[1],"already_initialized":True,"via":sys.argv[2]}; e.update(json.loads(sys.argv[3])); print(json.dumps(e))' "$acct" "$via" "$verify_json")"
    return 0
  fi
  already="false"

  ps_prepend_lez_wallet_path
  local wallet_bin
  wallet_bin="$(command -v wallet 2>/dev/null || true)"
  if [[ -n "$wallet_bin" && -n "${LEE_WALLET_HOME_DIR:-}" ]]; then
    via="wallet"
    ps_at_logoscore_close_wallet
    timeout 90 "$wallet_bin" auth-transfer init --account-id "Public/$acct" >/dev/null 2>&1 || true
    ps_at_logoscore_open_wallet
    ps_at_sync_wallet
    if ps_account_is_at_initialized "$acct"; then
      verify_json="$(ps_at_verify_extra)"
      ps_at_emit_phase "$phase" true "$(python3 -c 'import json,sys; e={"account_id":sys.argv[1],"already_initialized":False,"via":sys.argv[2]}; e.update(json.loads(sys.argv[3])); print(json.dumps(e))' "$acct" "$via" "$verify_json")"
      return 0
    fi
    err="wallet auth-transfer init did not yield AT-owned account"
    ps_at_logoscore_open_wallet
  fi

  if ps_at_logoscore_reachable; then
    via="logoscore_register"
    local hex line
    hex="$(ps_at_logoscore_call call logos_execution_zone account_id_from_base58 "$acct" | tail -1)"
    hex="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$hex" 2>/dev/null || true)"
    if [[ -n "$hex" ]]; then
      line="$(ps_at_logoscore_call call logos_execution_zone register_public_account "$hex" | tail -1)"
      tx_hash="$(ps_at_extract_field "$line" tx_hash)"
      if [[ -n "$tx_hash" ]]; then
        await_inclusion "$tx_hash"
        ps_at_sync_wallet
        if ps_account_is_at_initialized "$acct"; then
          verify_json="$(ps_at_verify_extra)"
          ps_at_emit_phase "$phase" true "$(python3 -c 'import json,sys; e={"account_id":sys.argv[1],"already_initialized":False,"via":sys.argv[2],"tx_hash":sys.argv[3]}; e.update(json.loads(sys.argv[4])); print(json.dumps(e))' "$acct" "$via" "$tx_hash" "$verify_json")"
          return 0
        fi
      fi
    fi
    err="${err:-logoscore register_public_account failed}"
  else
    err="${err:-no wallet or logoscore path for AT init}"
  fi

  verify_json="$(ps_at_verify_extra)"
  if [[ "$phase" == "auth_init_provider" ]]; then
    hint="rotate_provider_account_id"
  fi
  ps_at_emit_phase "$phase" false "$(python3 -c 'import json,sys; e={"account_id":sys.argv[1],"already_initialized":False,"via":sys.argv[2],"error":sys.argv[3]}; 
h=sys.argv[4]
if h: e["hint"]=h
e.update(json.loads(sys.argv[5])); print(json.dumps(e))' "$acct" "$via" "$err" "$hint" "$verify_json")"
  return 1
}

# ps_auth_transfer_ensure <owner_b58> <provider_b58> — exit 0 if both ok.
ps_auth_transfer_ensure() {
  local owner="$1" provider="$2"
  [[ -n "${ARTIFACT:-}" ]] || ps_fatal "ARTIFACT required for ps_auth_transfer_ensure"
  ps_auth_transfer_init_one "$owner" auth_init_owner || return 1
  ps_auth_transfer_init_one "$provider" auth_init_provider || return 1
  return 0
}
