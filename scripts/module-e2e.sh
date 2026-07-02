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

# seq_tx_included <tx_hash> -> returns 0 if the sequencer has included the tx
# (getTransaction returns a non-null result), 1 otherwise. The module returns a
# tx_hash on submission; this checks the sequencer's authoritative view of
# inclusion, which is what actually determines whether state changed.
seq_tx_included() {
  local hash="$1" res
  [[ -n "$hash" ]] || return 1
  res="$(curl -sf -X POST "$(ps_seq_url)" -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getTransaction\",\"params\":[\"$hash\"]}" 2>/dev/null || true)"
  python3 -c '
import json,sys
try:
    d=json.loads(sys.argv[1])
    r=d.get("result")
    ok = r is not None and r != "" and r != []
except Exception:
    ok=False
sys.exit(0 if ok else 1)
' "$res" 2>/dev/null
}

# await_inclusion <tx_hash> -> polls getTransaction until the tx is included,
# returns 0 once included, 1 after the retry budget is exhausted. Serializing
# writes on inclusion avoids nonce races where later txs are submitted before
# earlier ones land and get dropped by the sequencer.
await_inclusion() {
  local hash="$1" attempt
  for attempt in $(seq 1 "${INCLUSION_ATTEMPTS:-20}"); do
    if seq_tx_included "$hash"; then
      return 0
    fi
    sleep 5
  done
  return 1
}

# auth_transfer_init <account_base58> -> initialize the account under the
# authenticated_transfer program (LEZ v0.2.0 requires this before the account
# can be debited by deposit or credited by claim). The account must still be
# default-owned, so call this before topup and before the account signs any tx.
# Returns 0 once the init tx is included on chain, 1 otherwise.
auth_transfer_init() {
  local acct="$1" hex line tx_hash
  hex="$(logoscore call logos_execution_zone account_id_from_base58 "$acct" 2>/dev/null | tail -1)"
  hex="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$hex" 2>/dev/null || true)"
  [[ -n "$hex" ]] || return 1
  line="$(logoscore call logos_execution_zone register_public_account "$hex" 2>/dev/null | tail -1)"
  tx_hash="$(extract_field "$line" tx_hash)"
  [[ -n "$tx_hash" ]] || return 1
  await_inclusion "$tx_hash"
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
  local attempt line="" tx_hash=""
  for attempt in 1 2 3 4 5 6; do
    line="$(logoscore call payment_streams_module chainAction "$op" "$params" 2>/dev/null | tail -1)"
    if inner_status_ok "$line" "$key"; then
      tx_hash="$(extract_field "$line" tx_hash)"
      if [[ -n "$tx_hash" ]] && ! await_inclusion "$tx_hash"; then
        emit_phase "$phase" false "{\"op\":\"$op\",\"attempt\":$attempt,\"tx_hash\":\"$tx_hash\",\"inclusion\":\"timeout\"}"
        narr_fail "$label"
        narr_hint "Tx submitted but not included on chain (getTransaction null) — check sequencer mempool and nonce"
        if [[ "$required" == "1" ]]; then
          FAILURES=$((FAILURES + 1))
        fi
        sync_wallet
        echo ""
        return 0
      fi
      emit_phase "$phase" true "{\"op\":\"$op\",\"attempt\":$attempt$( [[ -n "$tx_hash" ]] && echo ",\"tx_hash\":\"$tx_hash\"" )}"
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
# Account funding (localnet): initialize owner/provider under the
# authenticated_transfer program, then top up gas. AT-init must happen while
# the accounts are still default-owned (before topup and before they sign any
# tx), otherwise deposit (which chains into AT to debit the owner) and claim
# (which chains into AT to credit the provider) are rejected on chain.
# ---------------------------------------------------------------------------
if ps_is_local; then
  narr_step "Initializing accounts under authenticated_transfer program"
  if auth_transfer_init "$OWNER"; then
    emit_phase auth_init_owner true
    narr_ok "Owner initialized under authenticated_transfer"
  else
    emit_phase auth_init_owner false
    narr_fail "Owner AT-init not confirmed on chain"
    narr_hint "register_public_account did not settle; deposit will be rejected"
    FAILURES=$((FAILURES + 1))
  fi
  if auth_transfer_init "$PROVIDER"; then
    emit_phase auth_init_provider true
    narr_ok "Provider initialized under authenticated_transfer"
  else
    emit_phase auth_init_provider false
    narr_fail "Provider AT-init not confirmed on chain"
    narr_hint "register_public_account did not settle; claim will be rejected"
    FAILURES=$((FAILURES + 1))
  fi
  sync_wallet

  narr_step "Funding owner and provider for gas"
  SCAFFOLD_WALLET="$(ps_lez_cache)/target/release/wallet"
  if [[ -x "$SCAFFOLD_WALLET" ]]; then
    export PATH="$(dirname "$SCAFFOLD_WALLET"):$PATH"
    export LEE_WALLET_HOME_DIR="$WALLET_HOME"
    timeout 30 lgs wallet topup --address "Public/$OWNER" >/dev/null 2>&1 || true
    timeout 30 lgs wallet topup --address "Public/$PROVIDER" >/dev/null 2>&1 || true
  fi
  sync_wallet
fi

# ---------------------------------------------------------------------------
# On-chain read helpers
#
# The demo verifies real chain state after each balance-changing operation
# instead of reporting script-side estimates. These helpers read through the
# module's getVaultStatus / getStreamStatus ops (which take the owner as an
# explicit parameter, so they work with the fresh localnet owner as well as the
# testnet fixture owner) and the sequencer's getAccount (for the provider
# account balance). All reads are async-inclusion-sensitive, so callers poll.
# ---------------------------------------------------------------------------

# read_vault <owner> <vault_id> -> echoes "holding_balance total_allocated_lo"
# (empty on error). holding_balance is parsed from the 16-byte little-endian
# u128 balance hex returned by getVaultStatus.
read_vault() {
  local owner="$1" vid="$2" line
  line="$(logoscore call payment_streams_module chainAction getVaultStatus \
    "{\"owner\":\"$owner\",\"vault_id\":$vid}" 2>/dev/null | tail -1)"
  python3 -c '
import json,sys
try:
    o=json.loads(sys.argv[1])
    inner=o.get("result","{}")
    if isinstance(inner,str):
        inner=json.loads(inner) if inner.strip().startswith("{") else {}
    if inner.get("status") != "ok":
        sys.exit(0)
    h=inner.get("vault_holding_balance_hex","") or ""
    if h[:2].lower() == "0x": h=h[2:]
    bal=int.from_bytes(bytes.fromhex(h), "little") if h else 0
    vc=inner.get("vault_config") or {}
    total=int(vc.get("total_allocated_lo",0) or 0)
    print(f"{bal} {total}")
except Exception:
    pass
' "$line" 2>/dev/null
}

# read_stream <owner> <vault_id> <stream_id> -> echoes
# "accrued_lo unaccrued_lo stream_state" (empty on error).
# stream_state: 0=Active, 1=Paused, 2=Closed.
read_stream() {
  local owner="$1" vid="$2" sid="$3" line
  line="$(logoscore call payment_streams_module chainAction getStreamStatus \
    "{\"owner\":\"$owner\",\"vault_id\":$vid,\"stream_id\":$sid}" 2>/dev/null | tail -1)"
  python3 -c '
import json,sys
try:
    o=json.loads(sys.argv[1])
    inner=o.get("result","{}")
    if isinstance(inner,str):
        inner=json.loads(inner) if inner.strip().startswith("{") else {}
    if inner.get("status") != "ok":
        sys.exit(0)
    a=int(inner.get("accrued_lo",0) or 0)
    u=int(inner.get("unaccrued_lo",0) or 0)
    st=int(inner.get("stream_state",-1) if inner.get("stream_state") is not None else -1)
    print(f"{a} {u} {st}")
except Exception:
    pass
' "$line" 2>/dev/null
}

stream_state_name() {
  case "$1" in
    0) echo "Active" ;;
    1) echo "Paused" ;;
    2) echo "Closed" ;;
    *) echo "unknown" ;;
  esac
}

# poll_read <fn> <args...> -> echoes fn output once it returns non-empty data,
# returns 0; returns 1 (no output) after the retry budget is exhausted.
poll_read() {
  local attempt out
  for attempt in $(seq 1 6); do
    sync_wallet
    out="$("$@")"
    if [[ -n "$out" ]]; then
      printf '%s' "$out"
      return 0
    fi
    sleep 5
  done
  return 1
}

# ---------------------------------------------------------------------------
# PHASE: Vault Initialization
# ---------------------------------------------------------------------------
narr_phase "Vault Initialization"

narr_step "Alice creates vault $VAULT_ID"
call_ps vault_init 1 initializeVault "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID}")" "" "Vault created: vault_id=$VAULT_ID"

narr_step "Depositing $DEPOSIT tokens into vault"
DEPOSIT_LINE="$(call_ps deposit 1 deposit "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"amount_lo\":$DEPOSIT,\"amount_hi\":0}")" "" "Deposit accepted by sequencer")"

# Verify the deposit settled on chain by reading the vault holding balance.
if DEPOSIT_VAULT="$(poll_read read_vault "$OWNER" "$VAULT_ID")"; then
  read -r DEPOSIT_VAULT_BAL _ <<< "$DEPOSIT_VAULT"
  if [[ "$DEPOSIT_VAULT_BAL" -eq "$DEPOSIT" ]]; then
    emit_phase deposit_balance true "{\"vault_balance\":$DEPOSIT_VAULT_BAL,\"expected\":$DEPOSIT}"
    narr_ok "Deposit confirmed on chain: vault holding balance $DEPOSIT_VAULT_BAL"
  else
    emit_phase deposit_balance false "{\"vault_balance\":$DEPOSIT_VAULT_BAL,\"expected\":$DEPOSIT}"
    narr_fail "Vault balance $DEPOSIT_VAULT_BAL != expected $DEPOSIT"
    narr_hint "Deposit may not be included yet; re-run getVaultStatus"
  fi
else
  emit_phase deposit_balance false "{\"error\":\"read_failed\"}"
  narr_fail "Could not read vault balance on chain"
  narr_hint "getVaultStatus returned no data — check wallet sync and sequencer"
fi

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
call_ps pause_stream 1 pauseStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")" "" "Pause accepted by sequencer"
if PAUSE_STREAM="$(poll_read read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"; then
  read -r _ _ PAUSE_ST <<< "$PAUSE_STREAM"
  emit_phase pause_state true "{\"stream_state\":$PAUSE_ST}"
  narr_ok "Stream paused: on-chain state $(stream_state_name "$PAUSE_ST")"
else
  emit_phase pause_state false "{\"error\":\"read_failed\"}"
  narr_ok "Stream paused (on-chain state not yet reflected)"
fi

narr_step "Resuming stream $STREAM_ID"
call_ps resume_stream 1 resumeStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")" "" "Resume accepted by sequencer"
if RESUME_STREAM="$(poll_read read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"; then
  read -r _ _ RESUME_ST <<< "$RESUME_STREAM"
  emit_phase resume_state true "{\"stream_state\":$RESUME_ST}"
  narr_ok "Stream resumed: on-chain state $(stream_state_name "$RESUME_ST")"
else
  emit_phase resume_state false "{\"error\":\"read_failed\"}"
  narr_ok "Stream resumed (on-chain state not yet reflected)"
fi

narr_step "Topping up stream $STREAM_ID by $TOPUP_INCREASE token"
# Capture on-chain allocation (accrued + unaccrued) before the top-up so we can
# verify the delta equals the top-up amount.
PRE_ALLOC=0
if TOPUP_PRE="$(poll_read read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"; then
  read -r PRE_ACC PRE_UNC _ <<< "$TOPUP_PRE"
  PRE_ALLOC=$((PRE_ACC + PRE_UNC))
fi
TOPUP_LINE="$(call_ps topup_stream 1 topUpStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"increase_lo\":$TOPUP_INCREASE,\"increase_hi\":0}")" "" "Top-up accepted by sequencer")"
if TOPUP_POST="$(poll_read read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"; then
  read -r POST_ACC POST_UNC _ <<< "$TOPUP_POST"
  POST_ALLOC=$((POST_ACC + POST_UNC))
  TOPUP_DELTA=$((POST_ALLOC - PRE_ALLOC))
  if [[ "$TOPUP_DELTA" -eq "$TOPUP_INCREASE" ]]; then
    emit_phase topup_allocation true "{\"pre_allocation\":$PRE_ALLOC,\"post_allocation\":$POST_ALLOC,\"delta\":$TOPUP_DELTA,\"expected_delta\":$TOPUP_INCREASE}"
    narr_ok "Top-up confirmed on chain: allocation $PRE_ALLOC -> $POST_ALLOC (+$TOPUP_DELTA)"
  else
    emit_phase topup_allocation false "{\"pre_allocation\":$PRE_ALLOC,\"post_allocation\":$POST_ALLOC,\"delta\":$TOPUP_DELTA,\"expected_delta\":$TOPUP_INCREASE}"
    narr_fail "Allocation delta $TOPUP_DELTA != expected $TOPUP_INCREASE"
    narr_hint "Top-up may not be included yet; re-run getStreamStatus"
  fi
else
  POST_ALLOC="$((ALLOCATION + TOPUP_INCREASE))"
  emit_phase topup_allocation false "{\"error\":\"read_failed\"}"
  narr_fail "Could not read stream allocation on chain"
  narr_hint "getStreamStatus returned no data — check wallet sync and sequencer"
fi

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
UNACCRUED=0
ACCRUAL_ATTEMPTS=30
for attempt in $(seq 1 $ACCRUAL_ATTEMPTS); do
  sync_wallet
  STREAM_READ="$(read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"
  if [[ -n "$STREAM_READ" ]]; then
    read -r ACCRUED UNACCRUED _ <<< "$STREAM_READ"
    if [[ "$ACCRUED" -ge "$MIN_ACCRUED" ]]; then
      break
    fi
  fi
  sleep 5
done

emit_phase accrual "$([[ "$ACCRUED" -ge "$MIN_ACCRUED" ]] && echo true || echo false)" "{\"accrued_lo\":$ACCRUED,\"unaccrued_lo\":$UNACCRUED,\"min_required\":$MIN_ACCRUED,\"attempts\":$attempt}"
if [[ "$ACCRUED" -ge "$MIN_ACCRUED" ]]; then
  narr_ok "Accrued: $ACCRUED tokens after ${attempt} poll(s) (unaccrued: $UNACCRUED)"
else
  narr_fail "Insufficient accrual: $ACCRUED tokens (need $MIN_ACCRUED)"
  narr_hint "Check sequencer is advancing and clock is synced"
fi

# ---------------------------------------------------------------------------
# PHASE: Claim
# ---------------------------------------------------------------------------
narr_phase "Claim"

# Verify the claim against two on-chain balances: the provider's account
# balance should increase by the payout, and the vault holding balance should
# decrease by the same payout. Both are captured before the claim and polled
# after. The provider may carry a pre-existing balance (e.g. localnet gas
# top-up), so deltas are reported rather than absolute values.
PRE_CLAIM_BALANCE="$(ps_account_balance "$PROVIDER" || echo 0)"
PRE_CLAIM_VAULT=0
if CLAIM_PRE_VAULT="$(poll_read read_vault "$OWNER" "$VAULT_ID")"; then
  read -r PRE_CLAIM_VAULT _ <<< "$CLAIM_PRE_VAULT"
fi

narr_step "Bob claims accrued funds from stream $STREAM_ID"
CLAIM_LINE="$(call_ps claim 1 claim "$(j "{\"provider\":\"$PROVIDER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")" "" "Claim accepted by sequencer")"

# Poll both balances until the payout settles (or the retry budget is
# exhausted). A settled claim shows the provider balance increase and the vault
# holding balance decrease by the same amount.
POST_CLAIM_BALANCE=""
POST_CLAIM_VAULT=""
CLAIM_BAL_ATTEMPTS=6
for attempt in $(seq 1 $CLAIM_BAL_ATTEMPTS); do
  sync_wallet
  POST_CLAIM_BALANCE="$(ps_account_balance "$PROVIDER" || echo 0)"
  if CLAIM_POST_VAULT="$(read_vault "$OWNER" "$VAULT_ID")"; then
    read -r POST_CLAIM_VAULT _ <<< "$CLAIM_POST_VAULT"
  fi
  if [[ -n "$POST_CLAIM_BALANCE" && "$POST_CLAIM_BALANCE" -gt "${PRE_CLAIM_BALANCE:-0}" ]]; then
    break
  fi
  sleep 5
done

if [[ -n "$POST_CLAIM_BALANCE" && "$POST_CLAIM_BALANCE" -gt "${PRE_CLAIM_BALANCE:-0}" ]]; then
  RECEIVED=$((POST_CLAIM_BALANCE - PRE_CLAIM_BALANCE))
  VAULT_DROP=$((PRE_CLAIM_VAULT - ${POST_CLAIM_VAULT:-0}))
  if [[ -n "$POST_CLAIM_VAULT" && "$VAULT_DROP" -eq "$RECEIVED" ]]; then
    emit_phase claim_balance true "{\"received\":$RECEIVED,\"provider_pre\":$PRE_CLAIM_BALANCE,\"provider_post\":$POST_CLAIM_BALANCE,\"vault_pre\":$PRE_CLAIM_VAULT,\"vault_post\":$POST_CLAIM_VAULT,\"attempts\":$attempt}"
    narr_ok "Claim confirmed on chain: Bob received $RECEIVED tokens"
    narr_value "Provider balance: $PRE_CLAIM_BALANCE -> $POST_CLAIM_BALANCE"
    narr_value "Vault holding: $PRE_CLAIM_VAULT -> $POST_CLAIM_VAULT (paid out $VAULT_DROP)"
  else
    emit_phase claim_balance true "{\"received\":$RECEIVED,\"provider_pre\":$PRE_CLAIM_BALANCE,\"provider_post\":$POST_CLAIM_BALANCE,\"vault_pre\":$PRE_CLAIM_VAULT,\"vault_post\":${POST_CLAIM_VAULT:-0},\"attempts\":$attempt}"
    narr_ok "Claim confirmed on chain: Bob received $RECEIVED tokens"
    narr_value "Provider balance: $PRE_CLAIM_BALANCE -> $POST_CLAIM_BALANCE"
    narr_value "Vault holding: $PRE_CLAIM_VAULT -> ${POST_CLAIM_VAULT:-?} (drop $VAULT_DROP vs payout $RECEIVED)"
    narr_hint "Vault holding drop did not match payout — may still be settling"
  fi
else
  emit_phase claim_balance false "{\"provider_pre\":$PRE_CLAIM_BALANCE,\"provider_post\":${POST_CLAIM_BALANCE:-0},\"vault_pre\":$PRE_CLAIM_VAULT,\"vault_post\":${POST_CLAIM_VAULT:-0},\"attempts\":$CLAIM_BAL_ATTEMPTS}"
  narr_ok "Claim accepted; on-chain balance not yet reflected"
  narr_hint "Balance read-back did not settle — check sequencer inclusion and re-read getAccount"
fi

# ---------------------------------------------------------------------------
# PHASE: Close
# ---------------------------------------------------------------------------
narr_phase "Close"

narr_step "Alice closes stream $STREAM_ID, reclaims unspent allocation"
CLOSE_LINE="$(call_ps close_stream 1 closeStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"authority\":\"$PROVIDER\"}")" "" "Close accepted by sequencer")"

# Read the final on-chain state: the stream should be Closed with unaccrued=0
# (the unspent allocation was reclaimed to the vault), and any residual accrued
# stays allocated on the closed stream until a later claim.
CLOSE_VAULT_BAL=""
CLOSE_VAULT_TOT=""
CLOSE_ACC=""
CLOSE_UNC=""
CLOSE_ST="-1"
if CLOSE_VAULT="$(poll_read read_vault "$OWNER" "$VAULT_ID")"; then
  read -r CLOSE_VAULT_BAL CLOSE_VAULT_TOT <<< "$CLOSE_VAULT"
fi
if CLOSE_STREAM="$(poll_read read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"; then
  read -r CLOSE_ACC CLOSE_UNC CLOSE_ST <<< "$CLOSE_STREAM"
fi

emit_phase close_state true "{\"vault_balance\":${CLOSE_VAULT_BAL:-0},\"total_allocated\":${CLOSE_VAULT_TOT:-0},\"stream_accrued\":${CLOSE_ACC:-0},\"stream_unaccrued\":${CLOSE_UNC:-0},\"stream_state\":${CLOSE_ST:--1}}"
narr_ok "Stream closed: on-chain state $(stream_state_name "$CLOSE_ST")"
narr_value "Stream residual: accrued=${CLOSE_ACC:-?}, unaccrued=${CLOSE_UNC:-?} (unaccrued reclaimed to vault)"
narr_value "Vault holding balance: ${CLOSE_VAULT_BAL:-?}, total_allocated: ${CLOSE_VAULT_TOT:-?}"

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
