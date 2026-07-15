#!/usr/bin/env bash
# module-e2e-local.sh — Flow A (module only) single-host happy path.
#
# Exercises payment_streams_module chainAction end-to-end through logoscore:
# vault init, deposit, stream create, pause/resume/top-up, accrual, payee claim.
# No delivery_module, no Store, no eligibility. This is the module-only cell of
# the 2x2 verification matrix (Flow A x localnet). Revived and modernized from
# scripts/archive/step11b-logoscore-e2e.sh onto the unified script stack.
#
# Usage: ./scripts/module-e2e-local.sh
# Driven by: MODE=module CHAIN=local ./scripts/e2e.sh local run
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"

# Flow A is localnet only for now; A-testnet is future work (the payee claim that
# ends this happy path is not reliable on public testnet — see
# docs/archive/operator/testnet-claim-known-issue.md).
if ps_is_testnet; then
  ps_fatal "module-e2e-local.sh is localnet only (A-testnet is future work)"
fi

MODULES="${MODULES:-${MODULES_USER:-$(ps_e2e_user_modules_dir)}}"
WALLET_CONFIG="${WALLET_CONFIG:-$(ps_scaffold_localnet_wallet_dir)/wallet_config.json}"
WALLET_E2E_DIR="${WALLET_E2E_DIR:-$(ps_e2e_user_wallet_local_dir)}"
WALLET_E2E_PASSWORD="${WALLET_E2E_PASSWORD:-scaffold-local-dev}"

VAULT_ID="${VAULT_ID:-0}"
STREAM_ID="${STREAM_ID:-0}"
DEPOSIT="${DEPOSIT:-100}"
RATE="${RATE:-10}"
ALLOCATION="${ALLOCATION:-80}"
TOPUP_INCREASE="${TOPUP_INCREASE:-1}"

ARTIFACT="${ARTIFACT:-$(ps_e2e_artifacts_dir)/module-e2e-$(date +%Y%m%dT%H%M%S).log}"
mkdir -p "$(dirname "$ARTIFACT")"
: > "$ARTIFACT"

# Daemon log: redirect logoscore's per-block "Stored persistent accounts at ..."
# and sync lines out of the narrative (mirrors run_local_e2e.py start_daemon).
DAEMON_LOG="${DAEMON_LOG:-$(dirname "$ARTIFACT")/module-e2e-daemon.log}"
: > "$DAEMON_LOG"

FAILURES=0
DAEMON_PID=""

emit_phase() {
  # emit_phase <phase> <ok:true|false> [extra-json-object]
  local phase="$1" ok="$2" extra="${3:-}"
  [[ -z "$extra" ]] && extra="{}"
  echo "{\"phase\":\"$phase\",\"ok\":$ok,\"extra\":$extra}" | tee -a "$ARTIFACT"
}

cleanup() {
  logoscore stop 2>/dev/null || true
  [[ -n "$DAEMON_PID" ]] && wait "$DAEMON_PID" 2>/dev/null || true
}
trap cleanup EXIT

ps_require_command logoscore
ps_require_command lgs

# ---------------------------------------------------------------------------
# Localnet must be up; the module submits real transactions to the sequencer.
# ---------------------------------------------------------------------------
if [[ "$("$REPO_ROOT/scripts/lifecycle.sh" localnet status)" != "running" ]]; then
  "$REPO_ROOT/scripts/lifecycle.sh" localnet start
fi

[[ -d "$MODULES" ]] || ps_fatal "Modules dir not found: $MODULES (run: MODE=module ./scripts/e2e.sh local prepare)"

# ---------------------------------------------------------------------------
# Wallet: a fresh, isolated wallet with its own owner + provider accounts. This
# keeps Flow A independent from the Store flow's funded snapshot owner/vault and
# guarantees a fresh vault 0 / stream 0 each run (so reads are deterministic).
# ---------------------------------------------------------------------------
mkdir -p "$WALLET_E2E_DIR"
rm -f "$WALLET_E2E_DIR/storage.json"
cp "$WALLET_CONFIG" "$WALLET_E2E_DIR/wallet_config.json"
WALLET_CONFIG="$WALLET_E2E_DIR/wallet_config.json"
WALLET_STORAGE="$WALLET_E2E_DIR/storage.json"

sync_wallet() {
  local height
  height="$(curl -sf -X POST "$(ps_seq_url)" -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' |
    python3 -c 'import json,sys; d=json.load(sys.stdin); r=d.get("result"); print(r if isinstance(r,int) else (r or ""))' 2>/dev/null || true)"
  if [[ -n "$height" ]]; then
    logoscore call logos_execution_zone sync_to_block "$height" >/dev/null 2>&1 || true
  fi
  sleep 3
}

# Parse `logoscore call ... chainAction` output: outer JSON has a "result"
# field that is itself a JSON string {"status":..., <key>:...}.
inner_status_ok() {
  # inner_status_ok <outer-json> [required-key]
  python3 -c '
import json,sys
try:
    outer=json.loads(sys.argv[1])
    inner=outer.get("result","{}")
    if isinstance(inner,str):
        inner=json.loads(inner) if inner.strip().startswith("{") else {}
except Exception:
    sys.exit(1)
ok = inner.get("status") == "ok"
if len(sys.argv) > 2 and sys.argv[2]:
    ok = ok and (sys.argv[2] in inner)
sys.exit(0 if ok else 1)
' "$1" "${2:-}" 2>/dev/null
}

# Extract the tx hash from a chainAction response. Field name varies by
# chain/wallet build (tx_hash, txHash, or nested under wallet). extract_tx_hash <json-line>
extract_tx_hash() {
  python3 -c '
import json,sys
try:
    outer=json.loads(sys.argv[1])
    inner=outer.get("result","{}")
    if isinstance(inner,str):
        inner=json.loads(inner) if inner.strip().startswith("{") else {}
    h = inner.get("tx_hash") or inner.get("txHash")
    if isinstance(h,str) and h.strip():
        print(h.strip()); sys.exit(0)
    w = inner.get("wallet")
    if isinstance(w,dict):
        wh = w.get("tx_hash") or w.get("txHash")
        if isinstance(wh,str) and wh.strip():
            print(wh.strip()); sys.exit(0)
except Exception:
    pass
' "$1" 2>/dev/null
}

# call_ps <phase> <required:0|1> <op> <params-json> [status-key]
call_ps() {
  local phase="$1" required="$2" op="$3" params="$4" key="${5:-}"
  local attempt line="" tx_hash=""
  for attempt in 1 2 3 4 5 6; do
    line="$(logoscore call payment_streams_module chainAction "$op" "$params" 2>/dev/null | tail -1)"
    if inner_status_ok "$line" "$key"; then
      tx_hash="$(extract_tx_hash "$line")"
      emit_phase "$phase" true "{\"op\":\"$op\",\"attempt\":$attempt$( [[ -n "$tx_hash" ]] && echo ",\"tx_hash\":\"$tx_hash\"" )}"
      if [[ -n "$tx_hash" ]]; then
        ps_log_info "    tx published on chain: $tx_hash"
      fi
      sync_wallet
      return 0
    fi
    sleep 8
  done
  emit_phase "$phase" false "{\"op\":\"$op\",\"raw\":$(python3 -c 'import json,sys;print(json.dumps(sys.argv[1]))' "${line:-}")}"
  if [[ "$required" == "1" ]]; then
    FAILURES=$((FAILURES + 1))
  fi
  sync_wallet
  return 0
}

# ---------------------------------------------------------------------------
# Start daemon and load modules (wallet first, then payment streams; D6).
# ---------------------------------------------------------------------------
logoscore stop 2>/dev/null || true
sleep 2
logoscore -D -m "$MODULES" -q >>"$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!
sleep 3
logoscore load-module logos_execution_zone >/dev/null
logoscore load-module payment_streams_module >/dev/null

if [[ ! -f "$WALLET_STORAGE" ]]; then
  OPEN_LINE="$(logoscore call logos_execution_zone create_new "$WALLET_CONFIG" "$WALLET_STORAGE" "$WALLET_E2E_PASSWORD" 2>/dev/null | tail -1)"
else
  OPEN_LINE="$(logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE" 2>/dev/null | tail -1)"
  if ! python3 -c 'import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get("result")==0 else 1)' "$OPEN_LINE" 2>/dev/null; then
    rm -f "$WALLET_STORAGE"
    OPEN_LINE="$(logoscore call logos_execution_zone create_new "$WALLET_CONFIG" "$WALLET_STORAGE" "$WALLET_E2E_PASSWORD" 2>/dev/null | tail -1)"
  fi
fi
# create_new returns status="ok" with result=<mnemonic string>; open returns
# result=0. Accept either as a successful wallet open.
if python3 -c 'import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get("status")=="ok" or d.get("result")==0 else 1)' "$OPEN_LINE" 2>/dev/null; then
  emit_phase wallet_open true
else
  emit_phase wallet_open false
  ps_fatal "wallet open failed: $OPEN_LINE"
fi
logoscore call logos_execution_zone save >/dev/null 2>&1 || true

to_base58() {
  local hex_id="$1" line b58
  line="$(logoscore call logos_execution_zone account_id_to_base58 "$hex_id" 2>/dev/null | tail -1)"
  b58="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$line" 2>/dev/null || true)"
  [[ -n "$b58" ]] && echo "$b58" || echo "$hex_id"
}

parse_new_account() {
  python3 -c '
import json,sys
outer=json.loads(sys.argv[1])
inner=outer.get("result","")
if isinstance(inner,str) and inner.startswith("{"): inner=json.loads(inner)
if isinstance(inner,dict):
  for k in ("account_id","accountId","base58","account_id_base58"):
    if k in inner: print(str(inner[k]).replace("Public/","").strip()); sys.exit(0)
s=str(inner).replace("Public/","").strip()
if s: print(s)
' "$1" 2>/dev/null || true
}

# Owner and provider (payee) are both fresh public accounts in this isolated
# wallet, so vault 0 / stream 0 are guaranteed fresh this run.
OWNER="$(parse_new_account "$(logoscore call logos_execution_zone create_account_public 2>/dev/null | tail -1)")"
[[ -z "$OWNER" ]] && ps_fatal "could not create owner account"
[[ ${#OWNER} -eq 64 ]] && OWNER="$(to_base58 "$OWNER")"

PROVIDER="$(parse_new_account "$(logoscore call logos_execution_zone create_account_public 2>/dev/null | tail -1)")"
[[ -z "$PROVIDER" ]] && ps_fatal "could not create provider account"
[[ ${#PROVIDER} -eq 64 ]] && PROVIDER="$(to_base58 "$PROVIDER")"
logoscore call logos_execution_zone save >/dev/null 2>&1 || true

# Fund owner and provider for gas via the pinned scaffold wallet CLI.
SCAFFOLD_WALLET="$(ps_lez_cache)/target/release/wallet"
if [[ -x "$SCAFFOLD_WALLET" ]]; then
  export PATH="$(dirname "$SCAFFOLD_WALLET"):$PATH"
  export LEE_WALLET_HOME_DIR="$WALLET_E2E_DIR"
  timeout 30 lgs wallet topup --address "Public/$OWNER" >/dev/null 2>&1 || true
  timeout 30 lgs wallet topup --address "Public/$PROVIDER" >/dev/null 2>&1 || true
fi

ps_log_info "owner=$OWNER provider=$PROVIDER vault=$VAULT_ID stream=$STREAM_ID"
sync_wallet

j() { python3 -c 'import json,sys; print(json.dumps(json.loads(sys.argv[1])))' "$1"; }

call_ps vault_init   1 initializeVault "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID}")"
call_ps deposit      1 deposit         "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"amount_lo\":$DEPOSIT,\"amount_hi\":0}")"
call_ps create_stream 1 createStream   "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"provider\":\"$PROVIDER\",\"rate\":$RATE,\"allocation_lo\":$ALLOCATION,\"allocation_hi\":0}")"
call_ps pause_stream 1 pauseStream     "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")"
call_ps resume_stream 1 resumeStream   "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")"
call_ps topup_stream 1 topUpStream     "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"increase_lo\":$TOPUP_INCREASE,\"increase_hi\":0}")"
# Status reads are observability, not part of the happy-path gate. The module
# does not wait for inclusion, so a derived stream/vault PDA can read back as
# "account data missing" after a successful submit; treated as SKIP, matching
# the original step11b verify (docs/archive/steps/module-chain-writes-runbook.md).
call_ps vault_status 0 getVaultStatus  "$(j "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID}")" vault_id
call_ps stream_status 0 getStreamStatus "$(j "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")" stream_id
# Let the stream accrue, then the payee claims residual (teardown).
sleep 5
call_ps claim        1 claim           "$(j "{\"provider\":\"$PROVIDER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")"

if [[ "$FAILURES" -eq 0 ]]; then
  emit_phase module_e2e_complete true "{\"artifact\":$(python3 -c 'import json,sys;print(json.dumps(sys.argv[1]))' "$ARTIFACT")}"
  ps_log_info "Flow A (module) local happy path GREEN. Artifact: $ARTIFACT Daemon log: $DAEMON_LOG"
  exit 0
else
  emit_phase module_e2e_complete false "{\"failures\":$FAILURES}"
  ps_log_error "Flow A (module) local happy path had $FAILURES failure(s). Artifact: $ARTIFACT Daemon log: $DAEMON_LOG"
  exit 1
fi
