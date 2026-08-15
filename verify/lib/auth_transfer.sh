#!/usr/bin/env bash
# Shared authenticated_transfer ensure for payment-streams E2E.
# Requires ARTIFACT, LEE_WALLET_HOME_DIR (for wallet path), optional WALLET_CONFIG/STORAGE.

set -euo pipefail

[[ -n "${PS_AUTH_TRANSFER_SOURCED:-}" ]] && return 0
PS_AUTH_TRANSFER_SOURCED=1

_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=verify/lib/common.sh
source "$_LIB_DIR/common.sh"
# shellcheck source=verify/lib/chain_poll.sh
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
# Note: logos_execution_zone.close is unavailable/fails on current LEZ builds
# ("wallet is already open" after close). Exclusive handoff requires daemon stop.
ps_at_logoscore_close_wallet() {
  [[ "${PS_AT_LOGOSCORE_WALLET_HANDOFF:-0}" == "1" ]] || return 0
  command -v logoscore >/dev/null 2>&1 || return 0
  logoscore call logos_execution_zone save >/dev/null 2>&1 || true
  logoscore call logos_execution_zone close >/dev/null 2>&1 || true
}

ps_at_logoscore_open_wallet() {
  [[ "${PS_AT_LOGOSCORE_WALLET_HANDOFF:-0}" == "1" ]] || return 0
  command -v logoscore >/dev/null 2>&1 || return 0
  [[ -n "${WALLET_CONFIG:-}" && -n "${WALLET_STORAGE:-}" ]] || return 0
  local stats
  stats="${WALLET_STATISTICS:-$(ps_ensure_wallet_statistics "$WALLET_STORAGE")}"
  logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE" "$stats" >/dev/null 2>&1 || true
  ps_at_sync_wallet
}

# Full daemon stop so wallet CLI has exclusive storage access (D39.22).
ps_logoscore_rpc_down() {
  ! timeout 2 logoscore call logos_execution_zone save >/dev/null 2>&1
}

ps_logoscore_daemon_stop_for_wallet() {
  command -v logoscore >/dev/null 2>&1 || return 0
  logoscore call logos_execution_zone save >/dev/null 2>&1 || true
  timeout 20 logoscore stop >/dev/null 2>&1 || true
  if [[ -n "${DAEMON_PID:-}" ]]; then
    kill "$DAEMON_PID" 2>/dev/null || true
    wait "$DAEMON_PID" 2>/dev/null || true
    DAEMON_PID=""
  fi
  local i
  for i in $(seq 1 15); do
    if ps_logoscore_rpc_down; then
      sleep 1
      return 0
    fi
    timeout 5 logoscore stop >/dev/null 2>&1 || true
    sleep 1
  done
  echo "logoscore still reachable after exclusive-stop" >&2
  return 1
}

# Restart logoscore daemon after wallet CLI; requires MODULES (and optional DAEMON_LOG).
ps_logoscore_daemon_restart_after_wallet() {
  command -v logoscore >/dev/null 2>&1 || return 1
  [[ -n "${MODULES:-}" ]] || {
    echo "MODULES required to restart logoscore after wallet CLI" >&2
    return 1
  }
  ps_logoscore_rpc_down || ps_logoscore_daemon_stop_for_wallet || return 1
  local log="${DAEMON_LOG:-/dev/null}"
  # Inherit raised RPC budget on the daemon only (core_service→module); do not
  # export into the caller shell or logoscore stop hangs for minutes (D39.24).
  if [[ "${RISC0_DEV_MODE:-1}" == "0" ]]; then
    env LOGOSCORE_RPC_TIMEOUT_MS="${PS_LOGOSCORE_RPC_TIMEOUT_MS:-${LOGOSCORE_RPC_TIMEOUT_MS:-600000}}" \
      logoscore -D -m "$MODULES" -q >>"$log" 2>&1 &
  else
    logoscore -D -m "$MODULES" -q >>"$log" 2>&1 &
  fi
  # Parent module-e2e tracks DAEMON_PID when set.
  if [[ -n "${DAEMON_PID+x}" ]]; then
    DAEMON_PID=$!
  fi
  sleep 3
  logoscore load-module logos_execution_zone >/dev/null
  [[ -n "${WALLET_CONFIG:-}" && -n "${WALLET_STORAGE:-}" ]] || return 1
  local stats
  stats="${WALLET_STATISTICS:-$(ps_ensure_wallet_statistics "$WALLET_STORAGE")}"
  open_line="$(logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE" "$stats" 2>/dev/null | tail -1)" || true
  if grep -qi 'already open' <<<"${open_line:-}"; then
    echo "wallet already open after restart (stale in-memory handle; exclusive-stop did not release): ${open_line}" >&2
    return 1
  fi
  if ! python3 -c 'import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get("status")=="ok" or d.get("result")==0 else 1)' "$open_line" 2>/dev/null; then
    echo "wallet open failed after logoscore restart: ${open_line:-<empty>}" >&2
    return 1
  fi
  logoscore load-module payment_streams_module >/dev/null 2>&1 || true
  ps_at_sync_wallet
}

# Pinata via lgs wallet topup with logoscore down (one writer of storage.json).
# Prints "<balance> <attempts>" on stdout. Restarts the daemon before return.
ps_lgs_pinata_until() {
  local acct="$1" target="$2"
  local bal=0 attempts=0 max
  [[ -n "$acct" && -n "$target" ]] || return 1
  max=$((target / 150 + 3))
  (( max < 1 )) && max=1
  ps_logoscore_daemon_stop_for_wallet
  while (( bal < target )); do
    attempts=$((attempts + 1))
    if (( attempts > max )); then
      ps_logoscore_daemon_restart_after_wallet || true
      echo "$bal $attempts"
      return 1
    fi
    timeout 30 lgs wallet topup --address "Public/$acct" >/dev/null 2>&1 || true
    bal="$(ps_account_balance "$acct" 2>/dev/null | tr -d '[:space:]' || echo 0)"
    bal="${bal:-0}"
  done
  ps_logoscore_daemon_restart_after_wallet || {
    echo "$bal $attempts"
    return 1
  }
  echo "$bal $attempts"
}

# payment_streams_module caches the LEZ wallet handle; reload after handoff reopen.
ps_reload_payment_streams_wallet() {
  command -v logoscore >/dev/null 2>&1 || return 0
  logoscore unload-module payment_streams_module >/dev/null 2>&1 || true
  logoscore load-module payment_streams_module >/dev/null 2>&1 || true
  [[ -n "${WALLET_CONFIG:-}" && -n "${WALLET_STORAGE:-}" ]] || return 0
  local stats
  stats="${WALLET_STATISTICS:-$(ps_ensure_wallet_statistics "$WALLET_STORAGE")}"
  logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE" "$stats" >/dev/null 2>&1 || true
  ps_at_sync_wallet
}

# Cursor reset for note discovery after a clone or ImageID cut. Not a
# key-recovery step: NSKs must already be on disk from an exclusive wallet-CLI
# write. close() is a no-op on this LEZ pin, so this path only zeros
# last_synced_block and reopens.
ps_resync_wallet_from_genesis() {
  local storage="${WALLET_STORAGE:-}"
  [[ -n "$storage" && -f "$storage" ]] || return 0
  command -v logoscore >/dev/null 2>&1 || return 0
  logoscore call logos_execution_zone save >/dev/null 2>&1 || true
  python3 -c '
import json, sys
path = sys.argv[1]
with open(path, encoding="utf-8") as f:
    obj = json.load(f)
if not isinstance(obj, dict):
    raise SystemExit(0)
obj["last_synced_block"] = 0
with open(path, "w", encoding="utf-8") as f:
    json.dump(obj, f)
    f.write("\n")
' "$storage"
  logoscore unload-module payment_streams_module >/dev/null 2>&1 || true
  logoscore call logos_execution_zone close >/dev/null 2>&1 || true
  logoscore unload-module logos_execution_zone >/dev/null 2>&1 || true
  logoscore load-module logos_execution_zone >/dev/null 2>&1 || true
  [[ -n "${WALLET_CONFIG:-}" ]] || return 1
  local stats
  stats="${WALLET_STATISTICS:-$(ps_ensure_wallet_statistics "$storage")}"
  logoscore call logos_execution_zone open "$WALLET_CONFIG" "$storage" "$stats" >/dev/null 2>&1 || true
  logoscore load-module payment_streams_module >/dev/null 2>&1 || true
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
    ps_logoscore_daemon_stop_for_wallet
    ps_wallet_cli_timeout 90 auth-transfer init --account-id "Public/$acct" >/dev/null 2>&1 || true
    if [[ -n "${MODULES:-}" ]]; then
      ps_logoscore_daemon_restart_after_wallet || true
    fi
    ps_at_sync_wallet
    if ps_account_is_at_initialized "$acct"; then
      verify_json="$(ps_at_verify_extra)"
      ps_at_emit_phase "$phase" true "$(python3 -c 'import json,sys; e={"account_id":sys.argv[1],"already_initialized":False,"via":sys.argv[2]}; e.update(json.loads(sys.argv[3])); print(json.dumps(e))' "$acct" "$via" "$verify_json")"
      return 0
    fi
    err="wallet auth-transfer init did not yield AT-owned account"
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

# Real-prove Public→Private shield via wallet CLI (D39.22).
# Proves in-process (tens of seconds to several minutes); LogosAPI IPC is too short.
# Uses the same logoscore↔wallet handoff as AT-init (PS_AT_LOGOSCORE_WALLET_HANDOFF).
# Args: <from_b58> <to_b58> <amount> — raw ids without Public/Private prefix.
# On failure prints wallet stderr/stdout to stdout for the caller to capture.
ps_wallet_auth_transfer_send() {
  local from_b58="$1" to_b58="$2" amount="$3"
  local out_file rc=0 bin

  from_b58="${from_b58#Public/}"
  from_b58="${from_b58#Private/}"
  to_b58="${to_b58#Public/}"
  to_b58="${to_b58#Private/}"
  [[ -n "$from_b58" && -n "$to_b58" && -n "$amount" ]] || return 1
  [[ -n "${LEE_WALLET_HOME_DIR:-}" ]] || {
    echo "LEE_WALLET_HOME_DIR required for wallet auth-transfer send" >&2
    return 1
  }
  export NSSA_WALLET_HOME_DIR="${NSSA_WALLET_HOME_DIR:-$LEE_WALLET_HOME_DIR}"

  bin="$(ps_pinned_lez_wallet_bin)" || {
    echo "wallet binary not on PATH (pinned LEZ release)" >&2
    return 1
  }
  ps_assert_pinned_wallet_bin "$bin" || return $?

  out_file="$(mktemp "${TMPDIR:-/tmp}/ps-wallet-shield.XXXXXX")"
  # Exclusive daemon-stop so wallet CLI is the sole writer. close() is a no-op
  # on this LEZ pin; leaving the daemon up lets per-block autosave clobber the
  # CLI write (NSKs from the shield never persist).
  # PS_WALLET_AT_SKIP_HANDOFF=1: caller already holds the exclusive-stop window
  # (both shields in one CLI session so the second write cannot drop the first NSKs).
  if [[ "${PS_WALLET_AT_SKIP_HANDOFF:-0}" != "1" ]]; then
    ps_logoscore_daemon_stop_for_wallet || return 1
    ps_assert_logoscore_down_for_wallet_cli || return 1
  fi
  local rc=0
  local shield_timeout="${PS_WALLET_SHIELD_TIMEOUT:-1800}"
  timeout "$shield_timeout" env -u LD_LIBRARY_PATH \
    LEE_WALLET_HOME_DIR="$LEE_WALLET_HOME_DIR" \
    NSSA_WALLET_HOME_DIR="$NSSA_WALLET_HOME_DIR" \
    RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}" \
    "$bin" auth-transfer send \
    --from "Public/$from_b58" \
    --to "Private/$to_b58" \
    --amount "$amount" >"$out_file" 2>&1 || rc=$?
  if [[ "${PS_WALLET_AT_SKIP_HANDOFF:-0}" != "1" ]]; then
    if ! ps_logoscore_daemon_restart_after_wallet; then
      echo "failed to restart logoscore after wallet auth-transfer send" >&2
      cat "$out_file" || true
      rm -f "$out_file"
      return 1
    fi
  fi
  if [[ "$rc" -ne 0 ]]; then
    cat "$out_file" || true
    rm -f "$out_file"
    return "$rc"
  fi
  rm -f "$out_file"
  ps_at_sync_wallet
  return 0
}
