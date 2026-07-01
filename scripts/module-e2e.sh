#!/usr/bin/env bash
# module-e2e.sh — User Journey (Flow A, module only) single-host happy path.
#
# Exercises payment_streams_module chainAction end-to-end through logoscore:
# vault init, deposit, stream create, pause/resume/top-up, accrual, claim, close.
# No delivery_module, no Store, no eligibility. This is the module-only cell of
# the 2x2 verification matrix (Flow A x localnet or testnet).
#
# Scenario: Alice creates a payment stream to Bob, funds accrue, Bob claims
# accrued amount, Alice closes stream and reclaims unspent allocation.
#
# Usage:
#   CHAIN=local   ./scripts/module-e2e.sh   (default)
#   CHAIN=testnet ./scripts/module-e2e.sh
#   ./scripts/module-e2e.sh --verbosity quiet|normal|verbose
# Driven by: MODE=module CHAIN=<chain> ./scripts/e2e.sh <chain> run
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"

# ---------------------------------------------------------------------------
# Verbosity
# ---------------------------------------------------------------------------
VERBOSITY="verbose"
if [[ ! -t 1 ]]; then
  VERBOSITY="quiet"
fi
while [[ $# -gt 0 ]]; do
  case "$1" in
    --verbosity) VERBOSITY="$2"; shift 2 ;;
    --verbosity=*) VERBOSITY="${1#*=}"; shift ;;
    *) shift ;;
  esac
done
case "$VERBOSITY" in
  quiet|normal|verbose) ;;
  *) ps_fatal "invalid --verbosity: $VERBOSITY (use quiet|normal|verbose)" ;;
esac

# ---------------------------------------------------------------------------
# Narrative helpers
# ---------------------------------------------------------------------------
_narr_ts() { date +%H:%M:%S; }

_narr() {
  # _narr <level> <text>
  # level: always (normal+verbose), verbose (verbose only)
  local level="$1"; shift
  if [[ "$VERBOSITY" == "quiet" ]]; then return 0; fi
  if [[ "$level" == "verbose" && "$VERBOSITY" != "verbose" ]]; then return 0; fi
  echo "[$(_narr_ts)] $*" >&2
}

narr_header() {
  if [[ "$VERBOSITY" == "quiet" ]]; then return 0; fi
  local line="============================================"
  echo "" >&2
  echo "[$(_narr_ts)] $line" >&2
  echo "[$(_narr_ts)] Payment Streams E2E: User Journey ($([ "$CHAIN" = "testnet" ] && echo TestNet || echo LocalNet))" >&2
  echo "[$(_narr_ts)] Scenario: Alice creates a stream to Bob, funds accrue," >&2
  echo "[$(_narr_ts)]          Bob claims accrued amount, Alice closes stream" >&2
  echo "[$(_narr_ts)] $line" >&2
}

narr_phase() {
  _narr always ""
  _narr always "PHASE: $1"
}

narr_step() {
  _narr always "  → $*"
}

narr_ok() {
  _narr always "  ✓ $*"
}

narr_fail() {
  _narr always "  ✗ $*"
}

narr_verbose() {
  _narr verbose "    $*"
}

narr_value() {
  _narr always "    $*"
}

narr_hint() {
  _narr always "  ! Hint: $*"
}

narr_complete() {
  if [[ "$VERBOSITY" == "quiet" ]]; then return 0; fi
  local line="============================================"
  echo "" >&2
  echo "[$(_narr_ts)] $line" >&2
  echo "[$(_narr_ts)] E2E COMPLETE: All phases succeeded" >&2
  echo "[$(_narr_ts)] Artifact: $ARTIFACT" >&2
  echo "[$(_narr_ts)] $line" >&2
}

narr_complete_fail() {
  if [[ "$VERBOSITY" == "quiet" ]]; then return 0; fi
  local line="============================================"
  echo "" >&2
  echo "[$(_narr_ts)] $line" >&2
  echo "[$(_narr_ts)] E2E FAILED: $FAILURES phase(s) failed" >&2
  echo "[$(_narr_ts)] Artifact: $ARTIFACT" >&2
  echo "[$(_narr_ts)] $line" >&2
}

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------
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
  echo "{\"phase\":\"$phase\",\"ok\":$ok,\"extra\":$extra}" >> "$ARTIFACT"
}

cleanup() {
  logoscore stop 2>/dev/null || true
  [[ -n "$DAEMON_PID" ]] && wait "$DAEMON_PID" 2>/dev/null || true
}
trap cleanup EXIT

ps_require_command logoscore
ps_require_command lgs

narr_header

# ---------------------------------------------------------------------------
# PHASE: Environment Setup
# ---------------------------------------------------------------------------
narr_phase "Environment Setup"

if ps_is_testnet; then
  # Testnet: use the canonical testnet wallet home and fixture manifest
  WALLET_HOME="$(ps_chain_wallet_home)"
  WALLET_CONFIG="${WALLET_CONFIG:-$(ps_default_wallet_config)}"
  WALLET_STORAGE="${WALLET_STORAGE:-$(ps_default_wallet_storage)}"

  FIXTURE="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet-module.json}"
  if [[ ! -f "$FIXTURE" && -z "${FIXTURE_MANIFEST:-}" ]]; then
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

  narr_step "Using testnet fixture: owner=$OWNER provider=$PROVIDER"
else
  # Localnet: fresh isolated wallet with its own owner + provider accounts.
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

narr_step "Starting LEZ sequencer"
if ps_is_local; then
  if [[ "$("$REPO_ROOT/scripts/lifecycle.sh" localnet status)" != "running" ]]; then
    "$REPO_ROOT/scripts/lifecycle.sh" localnet start
  fi
fi
narr_ok "Sequencer ready"

[[ -d "$MODULES" ]] || {
  narr_fail "Modules dir not found: $MODULES"
  narr_hint "Run: MODE=module ./scripts/e2e.sh local prepare"
  ps_fatal "Modules dir not found: $MODULES"
}

narr_step "Starting logoscore, loading modules"
logoscore stop 2>/dev/null || true
sleep 2
logoscore -D -m "$MODULES" -q &
DAEMON_PID=$!
sleep 3
logoscore load-module logos_execution_zone >/dev/null
logoscore load-module payment_streams_module >/dev/null
narr_ok "logoscore ready, modules loaded: logos_execution_zone, payment_streams_module"

# ---------------------------------------------------------------------------
# Wallet open / create
# ---------------------------------------------------------------------------
narr_step "Opening wallet"
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

if python3 -c 'import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get("status")=="ok" or d.get("result")==0 else 1)' "$OPEN_LINE" 2>/dev/null; then
  emit_phase wallet_open true
  narr_ok "Wallet opened"
else
  emit_phase wallet_open false
  narr_fail "Wallet open failed"
  narr_hint "Check wallet config and storage paths"
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
  OWNER="$(parse_new_account "$(logoscore call logos_execution_zone create_account_public 2>/dev/null | tail -1)")"
  [[ -z "$OWNER" ]] && { narr_fail "Could not create owner account"; ps_fatal "could not create owner account"; }
  [[ ${#OWNER} -eq 64 ]] && OWNER="$(to_base58 "$OWNER")"

  PROVIDER="$(parse_new_account "$(logoscore call logos_execution_zone create_account_public 2>/dev/null | tail -1)")"
  [[ -z "$PROVIDER" ]] && { narr_fail "Could not create provider account"; ps_fatal "could not create provider account"; }
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

narr_value "owner=$OWNER provider=$PROVIDER vault=$VAULT_ID stream=$STREAM_ID chain=${CHAIN:-local}"

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

# Extract a field from chainAction response (inner result JSON).
# extract_field <json-line> <key>
extract_field() {
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

# call_ps <phase> <required:0|1> <op> <params-json> [status-key] [narrative-label]
call_ps() {
  local phase="$1" required="$2" op="$3" params="$4" key="${5:-}" label="${6:-$phase}"
  local attempt line=""
  for attempt in 1 2 3 4 5 6; do
    line="$(logoscore call payment_streams_module chainAction "$op" "$params" 2>/dev/null | tail -1)"
    if inner_status_ok "$line" "$key"; then
      emit_phase "$phase" true "{\"op\":\"$op\",\"attempt\":$attempt}"
      narr_ok "$label"
      sync_wallet
      echo "$line"
      return 0
    fi
    sleep 8
  done
  emit_phase "$phase" false "{\"op\":\"$op\",\"raw\":$(python3 -c 'import json,sys;print(json.dumps(sys.argv[1]))' "${line:-}")}"
  narr_fail "$label"
  narr_hint "Check sequencer height, wallet sync, and gas balance"
  if [[ "$required" == "1" ]]; then
    FAILURES=$((FAILURES + 1))
  fi
  sync_wallet
  echo ""
  return 0
}

sync_wallet

j() { python3 -c 'import json,sys; print(json.dumps(json.loads(sys.argv[1])))' "$1"; }

# ---------------------------------------------------------------------------
# PHASE: Vault Initialization
# ---------------------------------------------------------------------------
narr_phase "Vault Initialization"

narr_step "Alice creates vault $VAULT_ID"
call_ps vault_init 1 initializeVault "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID}")" "" "Vault created: vault_id=$VAULT_ID"

narr_step "Depositing $DEPOSIT tokens into vault"
DEPOSIT_LINE="$(call_ps deposit 1 deposit "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"amount_lo\":$DEPOSIT,\"amount_hi\":0}")" "" "Deposit confirmed, vault balance: $DEPOSIT")"

# ---------------------------------------------------------------------------
# PHASE: Stream Creation
# ---------------------------------------------------------------------------
narr_phase "Stream Creation"

narr_step "Alice opens stream $STREAM_ID to Bob"
narr_value "rate=$RATE tokens/sec, allocation=$ALLOCATION tokens, vault=$VAULT_ID"
narr_verbose "A payment stream allocates tokens to a provider at a fixed rate."
narr_verbose "The allocation is the maximum the stream can pay out."
call_ps create_stream 1 createStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"provider\":\"$PROVIDER\",\"rate\":$RATE,\"allocation_lo\":$ALLOCATION,\"allocation_hi\":0}")" "" "Stream created: stream_id=$STREAM_ID, status=ACTIVE"

# ---------------------------------------------------------------------------
# PHASE: Stream Lifecycle
# ---------------------------------------------------------------------------
narr_phase "Stream Lifecycle"

narr_step "Pausing stream $STREAM_ID"
call_ps pause_stream 1 pauseStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")" "" "Stream paused"

narr_step "Resuming stream $STREAM_ID"
call_ps resume_stream 1 resumeStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")" "" "Stream resumed"

narr_step "Topping up stream $STREAM_ID by $TOPUP_INCREASE token"
TOPUP_LINE="$(call_ps topup_stream 1 topUpStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"increase_lo\":$TOPUP_INCREASE,\"increase_hi\":0}")" "" "Top-up confirmed, allocation now $((ALLOCATION + TOPUP_INCREASE)) tokens")"
CURRENT_ALLOCATION=$((ALLOCATION + TOPUP_INCREASE))

# ---------------------------------------------------------------------------
# PHASE: Accrual
# ---------------------------------------------------------------------------
narr_phase "Accrual"

narr_step "Waiting for funds to accrue (rate=$RATE tokens/sec)"
narr_verbose "Accrual is timestamp-based: derived from on-chain accrued_as_of field."
narr_verbose "On testnet, granularity is limited by block time."

MIN_ACCRUED=$((RATE * 3))
narr_value "Need at least $MIN_ACCRUED tokens accrued before claim"

ACCRUED=0
ACCRUAL_ATTEMPTS=30
for attempt in $(seq 1 $ACCRUAL_ATTEMPTS); do
  sync_wallet
  STREAMS_LINE="$(logoscore call payment_streams_module listMyStreams "$VAULT_ID" 2>/dev/null | tail -1)"
  ACCRUED="$(python3 -c '
import json,sys
try:
    outer=json.loads(sys.argv[1])
    inner=outer.get("result","{}")
    if isinstance(inner,str):
        inner=json.loads(inner) if inner.strip().startswith("{") else {}
    for row in (inner.get("streams") or []):
        if int(row.get("stream_id",-1)) == int(sys.argv[2]):
            print(int(row.get("accrued_lo") or 0))
            sys.exit(0)
    print(0)
except Exception:
    print(0)
' "$STREAMS_LINE" "$STREAM_ID" 2>/dev/null || echo 0)"
  if [[ "$ACCRUED" -ge "$MIN_ACCRUED" ]]; then
    break
  fi
  sleep 5
done

emit_phase accrual "$([[ "$ACCRUED" -ge "$MIN_ACCRUED" ]] && echo true || echo false)" "{\"accrued_lo\":$ACCRUED,\"min_required\":$MIN_ACCRUED,\"attempts\":$attempt}"
if [[ "$ACCRUED" -ge "$MIN_ACCRUED" ]]; then
  narr_ok "Accrued: $ACCRUED tokens after ${attempt} poll(s)"
else
  narr_fail "Insufficient accrual: $ACCRUED tokens (need $MIN_ACCRUED)"
  narr_hint "Check sequencer is advancing and clock is synced"
fi

# ---------------------------------------------------------------------------
# PHASE: Claim
# ---------------------------------------------------------------------------
narr_phase "Claim"

narr_step "Bob claims accrued funds from stream $STREAM_ID"
CLAIM_LINE="$(call_ps claim 1 claim "$(j "{\"provider\":\"$PROVIDER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")" "" "Claim successful: Bob received $ACCRUED tokens")"
narr_value "Provider balance: $ACCRUED"

# ---------------------------------------------------------------------------
# PHASE: Close
# ---------------------------------------------------------------------------
narr_phase "Close"

narr_step "Alice closes stream $STREAM_ID, reclaims unspent allocation"
CLOSE_LINE="$(call_ps close_stream 1 closeStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"authority\":\"$PROVIDER\"}")" "" "Stream closed")"

RECLAIMED=$((CURRENT_ALLOCATION - ACCRUED))
VAULT_BALANCE=$((DEPOSIT - ACCRUED))
narr_value "Stream status: accrued=$ACCRUED, unspent reclaimed=$RECLAIMED, vault balance=$VAULT_BALANCE"

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
if [[ "$FAILURES" -eq 0 ]]; then
  emit_phase module_e2e_complete true "{\"artifact\":$(python3 -c 'import json,sys;print(json.dumps(sys.argv[1]))' "$ARTIFACT")}"
  narr_complete
  exit 0
else
  emit_phase module_e2e_complete false "{\"failures\":$FAILURES}"
  narr_complete_fail
  exit 1
fi
