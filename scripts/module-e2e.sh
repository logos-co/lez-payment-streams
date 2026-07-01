#!/usr/bin/env bash
# module-e2e.sh — Flow A (module only) single-host happy path.
#
# Exercises payment_streams_module chainAction end-to-end through logoscore:
# vault init, deposit, stream create, pause/resume/top-up, accrual, payee claim.
# No delivery_module, no Store, no eligibility. This is the module-only cell of
# the 2x2 verification matrix (Flow A x localnet or testnet).
#
# Usage:
#   CHAIN=local   ./scripts/module-e2e.sh   (default)
#   CHAIN=testnet ./scripts/module-e2e.sh
# Driven by: MODE=module CHAIN=<chain> ./scripts/e2e.sh <chain> run
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"

MODULES="${MODULES:-${MODULES_USER:-$REPO_ROOT/.scaffold/e2e/user/modules}}"

VAULT_ID="${VAULT_ID:-0}"
STREAM_ID="${STREAM_ID:-0}"
DEPOSIT="${DEPOSIT:-100}"
RATE="${RATE:-10}"
ALLOCATION="${ALLOCATION:-80}"
TOPUP_INCREASE="${TOPUP_INCREASE:-1}"

ARTIFACT="${ARTIFACT:-$REPO_ROOT/.scaffold/e2e/artifacts/module-e2e-$(date +%Y%m%dT%H%M%S).log}"
mkdir -p "$(dirname "$ARTIFACT")"
: > "$ARTIFACT"

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
# Chain-dependent setup: wallet, accounts, funding
# ---------------------------------------------------------------------------
if ps_is_testnet; then
  # Testnet: use the canonical testnet wallet home and fixture manifest
  WALLET_HOME="$(ps_chain_wallet_home)"
  WALLET_CONFIG="${WALLET_CONFIG:-$(ps_default_wallet_config)}"
  WALLET_STORAGE="${WALLET_STORAGE:-$(ps_default_wallet_storage)}"

  FIXTURE="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet-module.json}"
  if [[ ! -f "$FIXTURE" && -z "${FIXTURE_MANIFEST:-}" ]]; then
    # Fallback to the shared testnet fixture if no module-specific fixture exists
    FIXTURE="$REPO_ROOT/fixtures/testnet.json"
  fi
  if [[ ! -f "$FIXTURE" ]]; then
    ps_fatal "Testnet fixture not found: $FIXTURE (run: make bootstrap-testnet-module)"
  fi

  OWNER="$(ps_json_get "$FIXTURE" owner_account_id)"
  PROVIDER="$(ps_json_get "$FIXTURE" provider_account_id)"
  PROGRAM_ID_HEX="$(ps_json_get "$FIXTURE" program_id_hex)"

  [[ -n "$OWNER" ]] || ps_fatal "fixture missing owner_account_id"
  [[ -n "$PROVIDER" ]] || ps_fatal "fixture missing provider_account_id"
  [[ -n "$PROGRAM_ID_HEX" ]] || ps_fatal "fixture missing program_id_hex"

  ps_log_info "testnet fixture: owner=$OWNER provider=$PROVIDER"
else
  # Localnet: fresh isolated wallet with its own owner + provider accounts.
  # Keeps Flow A independent from the Store flow's funded snapshot.
  WALLET_E2E_DIR="${WALLET_E2E_DIR:-$REPO_ROOT/.scaffold/module-e2e-wallet}"
  WALLET_E2E_PASSWORD="${WALLET_E2E_PASSWORD:-scaffold-local-dev}"

  mkdir -p "$WALLET_E2E_DIR"
  rm -f "$WALLET_E2E_DIR/storage.json"
  cp "${WALLET_CONFIG:-$REPO_ROOT/.scaffold/wallet/wallet_config.json}" "$WALLET_E2E_DIR/wallet_config.json"
  WALLET_CONFIG="$WALLET_E2E_DIR/wallet_config.json"
  WALLET_STORAGE="$WALLET_E2E_DIR/storage.json"
  WALLET_HOME="$WALLET_E2E_DIR"

  OWNER=""
  PROVIDER=""
  PROGRAM_ID_HEX=""
fi

# ---------------------------------------------------------------------------
# Localnet must be up; testnet reaches out to the public sequencer.
# ---------------------------------------------------------------------------
if ps_is_local; then
  if [[ "$("$REPO_ROOT/scripts/lifecycle.sh" localnet status)" != "running" ]]; then
    "$REPO_ROOT/scripts/lifecycle.sh" localnet start
  fi
fi

[[ -d "$MODULES" ]] || ps_fatal "Modules dir not found: $MODULES (run: MODE=module ./scripts/e2e.sh local prepare)"

# ---------------------------------------------------------------------------
# Start daemon and load modules (wallet first, then payment streams; D6).
# ---------------------------------------------------------------------------
logoscore stop 2>/dev/null || true
sleep 2
logoscore -D -m "$MODULES" -q &
DAEMON_PID=$!
sleep 3
logoscore load-module logos_execution_zone >/dev/null
logoscore load-module payment_streams_module >/dev/null

# ---------------------------------------------------------------------------
# Wallet open / create
# ---------------------------------------------------------------------------
OPEN_LINE=""
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

# ---------------------------------------------------------------------------
# Account creation (localnet only) or fixture read (testnet)
# ---------------------------------------------------------------------------
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

if ps_is_local; then
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
    export LEE_WALLET_HOME_DIR="$WALLET_HOME"
    timeout 30 lgs wallet topup --address "Public/$OWNER" >/dev/null 2>&1 || true
    timeout 30 lgs wallet topup --address "Public/$PROVIDER" >/dev/null 2>&1 || true
  fi
fi

ps_log_info "owner=$OWNER provider=$PROVIDER vault=$VAULT_ID stream=$STREAM_ID chain=${CHAIN:-local}"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
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

# call_ps <phase> <required:0|1> <op> <params-json> [status-key]
call_ps() {
  local phase="$1" required="$2" op="$3" params="$4" key="${5:-}"
  local attempt line=""
  for attempt in 1 2 3 4 5 6; do
    line="$(logoscore call payment_streams_module chainAction "$op" "$params" 2>/dev/null | tail -1)"
    if inner_status_ok "$line" "$key"; then
      emit_phase "$phase" true "{\"op\":\"$op\",\"attempt\":$attempt}"
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

sync_wallet

j() { python3 -c 'import json,sys; print(json.dumps(json.loads(sys.argv[1])))' "$1"; }

call_ps vault_init   1 initializeVault "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID}")"
call_ps deposit      1 deposit         "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"amount_lo\":$DEPOSIT,\"amount_hi\":0}")"
call_ps create_stream 1 createStream   "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"provider\":\"$PROVIDER\",\"rate\":$RATE,\"allocation_lo\":$ALLOCATION,\"allocation_hi\":0}")"
call_ps pause_stream 1 pauseStream     "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")"
call_ps resume_stream 1 resumeStream   "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")"
call_ps topup_stream 1 topUpStream     "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"increase_lo\":$TOPUP_INCREASE,\"increase_hi\":0}")"
# Status reads are observability, not part of the happy-path gate.
call_ps vault_status 0 getVaultStatus  "$(j "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID}")" vault_id
call_ps stream_status 0 getStreamStatus "$(j "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")" stream_id
# Let the stream accrue, then the payee claims residual (teardown).
sleep 5
call_ps claim        1 claim           "$(j "{\"provider\":\"$PROVIDER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")"

if [[ "$FAILURES" -eq 0 ]]; then
  emit_phase module_e2e_complete true "{\"artifact\":$(python3 -c 'import json,sys;print(json.dumps(sys.argv[1]))' "$ARTIFACT")}"
  ps_log_info "Flow A (module) ${CHAIN:-local} happy path GREEN. Artifact: $ARTIFACT"
  exit 0
else
  emit_phase module_e2e_complete false "{\"failures\":$FAILURES}"
  ps_log_error "Flow A (module) ${CHAIN:-local} happy path had $FAILURES failure(s). Artifact: $ARTIFACT"
  exit 1
fi
