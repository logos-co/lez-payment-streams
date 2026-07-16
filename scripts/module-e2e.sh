#!/usr/bin/env bash
# module-e2e.sh — User Journey (Flow A, module only) single-host happy path.
#
# Exercises payment_streams_module chainAction end-to-end through logoscore:
# vault init, deposit, stream create, optional top-up (MODULE_E2E_TOPUP=1), accrual,
# close, then claim residual on the closed stream.
# No delivery_module, no Store, no eligibility. This is the module-only cell of
# the 2x2 verification matrix (Flow A x localnet or testnet).
#
# Scenario: Alice creates a payment stream to Bob, funds accrue, Alice closes
# the stream, Bob claims residual accrued on the closed stream.
#
# Usage:
#   CHAIN=local   ./scripts/module-e2e.sh   (default)
#   CHAIN=testnet ./scripts/module-e2e.sh
#   ./scripts/module-e2e.sh --verbosity quiet|normal|verbose
# Driven by: MODE=module CHAIN=<chain> ./scripts/e2e.sh <chain> run
# Privacy-enhanced (Step 36): PRIVACY=1 or scripts/module-e2e-privacy.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"
# shellcheck source=scripts/lib/chain_poll.sh
source "$REPO_ROOT/scripts/lib/chain_poll.sh"
# shellcheck source=scripts/lib/auth_transfer.sh
source "$REPO_ROOT/scripts/lib/auth_transfer.sh"
# shellcheck source=scripts/lib/fund_testnet.sh
source "$REPO_ROOT/scripts/lib/fund_testnet.sh"

ps_is_privacy_e2e() {
  [[ "${PRIVACY:-0}" == "1" ]]
}

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
_narr_ts() { date +%Y-%m-%dT%H:%M:%S; }

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
  echo "[$(_narr_ts)] Payment Streams E2E: User Journey ($([ "$CHAIN" = "testnet" ] && echo TestNet || echo LocalNet))$(ps_is_privacy_e2e && echo ' — PseudonymousFunder (PRIVACY=1)' || true)" >&2
  echo "[$(_narr_ts)] Scenario: Alice creates a stream to Bob, funds accrue," >&2
  echo "[$(_narr_ts)]          Alice closes stream, Bob claims residual accrued" >&2
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
  _narr always "  ! $*"
}

narr_complete() {
  if [[ "$VERBOSITY" == "quiet" ]]; then return 0; fi
  local line="============================================"
  echo "" >&2
  echo "[$(_narr_ts)] $line" >&2
  echo "[$(_narr_ts)] E2E COMPLETE: All phases succeeded" >&2
  echo "[$(_narr_ts)] Artifact: $ARTIFACT" >&2
  echo "[$(_narr_ts)] Daemon log: $DAEMON_LOG" >&2
  echo "[$(_narr_ts)] $line" >&2
}

narr_complete_fail() {
  if [[ "$VERBOSITY" == "quiet" ]]; then return 0; fi
  local line="============================================"
  echo "" >&2
  echo "[$(_narr_ts)] $line" >&2
  echo "[$(_narr_ts)] E2E FAILED: $FAILURES phase(s) failed" >&2
  echo "[$(_narr_ts)] Artifact: $ARTIFACT" >&2
  echo "[$(_narr_ts)] Daemon log: $DAEMON_LOG" >&2
  echo "[$(_narr_ts)] $line" >&2
}

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------
MODULES="${MODULES:-${MODULES_USER:-$(ps_e2e_user_modules_dir)}}"

# Capture explicit overrides before applying defaults so testnet can auto-resolve
# fresh ids when the caller did not pin them.
VAULT_ID_EXPLICIT="${VAULT_ID+x}"
STREAM_ID_EXPLICIT="${STREAM_ID+x}"
VAULT_ID="${VAULT_ID:-0}"
STREAM_ID="${STREAM_ID:-0}"
RATE="${RATE:-1}"
TOPUP_INCREASE="${TOPUP_INCREASE:-1}"
# Default 0: skip topUpStream to keep the demo shorter. Set MODULE_E2E_TOPUP=1 to include it.
MODULE_E2E_TOPUP="${MODULE_E2E_TOPUP:-0}"
# Set MODULE_E2E_SKIP_CLOSE=1 to skip settlement (close + claim; saves testnet txs).
MODULE_E2E_SKIP_CLOSE="${MODULE_E2E_SKIP_CLOSE:-0}"
# Set MODULE_E2E_SKIP_FUND=1 to skip inline testnet pinata funding (assumes the
# fixture owner/provider were pre-funded via scripts/fund-testnet-accounts.sh).
MODULE_E2E_SKIP_FUND="${MODULE_E2E_SKIP_FUND:-0}"
# Testnet fixture pins a single owner, so hardcoded vault 0 / stream 0 reuse
# stale, already-closed/claimed state across runs. Auto-resolve a fresh empty
# vault (and stream 0 within it) unless VAULT_ID is pinned or this is disabled.
# Localnet uses an isolated per-run wallet, so the defaults already start fresh.
MODULE_E2E_FRESH_VAULT="${MODULE_E2E_FRESH_VAULT:-1}"

# Chain-specific demo sizing and poll budgets. Public testnet blocks advance irregularly
# (often tens of seconds between heights); serial txs dominate wall clock via inclusion wait.
if ps_is_testnet; then
  DEPOSIT="${DEPOSIT:-500}"
  ALLOCATION="${ALLOCATION:-400}"
  MIN_ACCRUED="${MIN_ACCRUED:-1}"
  INCLUSION_ATTEMPTS="${INCLUSION_ATTEMPTS:-45}"
  INCLUSION_SLEEP="${INCLUSION_SLEEP:-2}"
  ACCRUAL_ATTEMPTS="${ACCRUAL_ATTEMPTS:-24}"
  ACCRUAL_POLL_SLEEP="${ACCRUAL_POLL_SLEEP:-3}"
  POLL_READ_ATTEMPTS="${POLL_READ_ATTEMPTS:-8}"
  POLL_READ_SLEEP="${POLL_READ_SLEEP:-3}"
else
  DEPOSIT="${DEPOSIT:-500}"
  ALLOCATION="${ALLOCATION:-400}"
  MIN_ACCRUED="${MIN_ACCRUED:-$((RATE * 3))}"
  INCLUSION_ATTEMPTS="${INCLUSION_ATTEMPTS:-20}"
  INCLUSION_SLEEP="${INCLUSION_SLEEP:-5}"
  ACCRUAL_ATTEMPTS="${ACCRUAL_ATTEMPTS:-30}"
  ACCRUAL_POLL_SLEEP="${ACCRUAL_POLL_SLEEP:-5}"
  POLL_READ_ATTEMPTS="${POLL_READ_ATTEMPTS:-6}"
  POLL_READ_SLEEP="${POLL_READ_SLEEP:-5}"
fi

ARTIFACT="${ARTIFACT:-$(ps_e2e_artifacts_dir)/module-e2e-$(date +%Y%m%dT%H%M%S).log}"
mkdir -p "$(dirname "$ARTIFACT")"
: > "$ARTIFACT"

# Daemon log: logoscore prints per-block "Stored persistent accounts at ..." and
# sync lines that are noise for the narrative. Redirect to a logfile (mirroring
# the Python orchestrator's start_daemon) and surface its path on failure.
DAEMON_LOG="${DAEMON_LOG:-$(dirname "$ARTIFACT")/module-e2e-daemon.log}"
: > "$DAEMON_LOG"

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

  # Always resolve a testnet fixture: ignore a stale FIXTURE_MANIFEST pointing
  # at a non-testnet manifest (e.g. localnet.json left in the shell env), which
  # would target accounts the testnet wallet does not own. Honor an explicit
  # FIXTURE_MANIFEST only if it actually points to a testnet fixture file.
  FIXTURE=""
  if [[ -n "${FIXTURE_MANIFEST:-}" ]] && [[ "${FIXTURE_MANIFEST##*/}" == testnet*.json ]] && [[ -f "$FIXTURE_MANIFEST" ]]; then
    FIXTURE="$FIXTURE_MANIFEST"
  fi
  [[ -n "$FIXTURE" ]] || FIXTURE="$REPO_ROOT/fixtures/testnet-module.json"
  [[ -f "$FIXTURE" ]] || FIXTURE="$REPO_ROOT/fixtures/testnet.json"
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
  WALLET_E2E_DIR="${WALLET_E2E_DIR:-$(ps_e2e_user_wallet_local_dir)}"
  WALLET_E2E_PASSWORD="${WALLET_E2E_PASSWORD:-scaffold-local-dev}"

  mkdir -p "$WALLET_E2E_DIR"
  rm -f "$WALLET_E2E_DIR/storage.json"
  cp "${WALLET_CONFIG:-$(ps_scaffold_localnet_wallet_dir)/wallet_config.json}" "$WALLET_E2E_DIR/wallet_config.json"
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
logoscore -D -m "$MODULES" -q >>"$DAEMON_LOG" 2>&1 &
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

account_id_to_hex() {
  local id="$1" line hex
  if [[ ${#id} -eq 64 && "$id" =~ ^[0-9a-fA-F]+$ ]]; then
    echo "${id,,}"
    return 0
  fi
  line="$(logoscore call logos_execution_zone account_id_from_base58 "$id" 2>/dev/null | tail -1)"
  hex="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$line" 2>/dev/null || true)"
  [[ -n "$hex" ]] && echo "${hex,,}"
}

amount_le16_hex() {
  python3 -c 'import sys; print(int(sys.argv[1]).to_bytes(16, "little").hex())' "$1"
}

logoscore_string_arg() {
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"
}

ps_pre_shield_to_private_owner() {
  local from_hex="$1" to_hex="$2" amount="$3"
  local amt_hex line tx_hash
  amt_hex="s:$(amount_le16_hex "$amount")"
  line="$(logoscore call logos_execution_zone transfer_shielded_owned \
    "$from_hex" "$to_hex" \
    "$(logoscore_string_arg "$amt_hex")" 2>/dev/null | tail -1)"
  tx_hash="$(python3 -c '
import json,sys
try:
    outer=json.loads(sys.argv[1])
    inner=outer.get("result", outer)
    if isinstance(inner,str):
        inner=json.loads(inner) if inner.strip().startswith("{") else {}
    if not isinstance(inner,dict):
        sys.exit(0)
    ok = inner.get("status") == "ok" or inner.get("success") is True
    if not ok:
        sys.exit(0)
    print(inner.get("tx_hash") or inner.get("txHash") or "")
except Exception:
    pass
' "$line" 2>/dev/null || true)"
  if [[ -n "$tx_hash" ]]; then
    await_inclusion "$tx_hash" || true
  fi
  if python3 -c '
import json,sys
try:
    outer=json.loads(sys.argv[1])
    inner=outer.get("result", outer)
    if isinstance(inner,str):
        inner=json.loads(inner) if inner.strip().startswith("{") else {}
    ok = isinstance(inner,dict) and (inner.get("status") == "ok" or inner.get("success") is True)
    sys.exit(0 if ok else 1)
except Exception:
    sys.exit(1)
' "$line" 2>/dev/null; then
    emit_phase pre_shield true "{\"amount\":$amount,\"from_hex\":\"$from_hex\",\"to_hex\":\"$to_hex\"$( [[ -n "$tx_hash" ]] && echo ",\"tx_hash\":\"$tx_hash\"" )}"
    narr_ok "Pre-shielded $amount tokens into vault owner private account"
    sync_wallet
    return 0
  fi
  emit_phase pre_shield false "{\"raw\":$(python3 -c 'import json,sys;print(json.dumps(sys.argv[1]))' "${line:-}")}"
  narr_fail "transfer_shielded_owned failed"
  return 1
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
  if ps_is_privacy_e2e; then
    narr_step "Creating public funder, private vault owner, and public provider (PRIVACY=1)"
    PUBLIC_FUNDER="$(parse_new_account "$(logoscore call logos_execution_zone create_account_public 2>/dev/null | tail -1)")"
    [[ -z "$PUBLIC_FUNDER" ]] && ps_fatal "could not create public funder account"
    [[ ${#PUBLIC_FUNDER} -eq 64 ]] && PUBLIC_FUNDER="$(to_base58 "$PUBLIC_FUNDER")"

    OWNER_HEX="$(parse_new_account "$(logoscore call logos_execution_zone create_account_private 2>/dev/null | tail -1)")"
    [[ -z "$OWNER_HEX" || ${#OWNER_HEX} -ne 64 ]] && ps_fatal "could not create private vault owner (expected 32-byte hex id)"
    OWNER="$(to_base58 "$OWNER_HEX")"

    PROVIDER="$(parse_new_account "$(logoscore call logos_execution_zone create_account_public 2>/dev/null | tail -1)")"
    [[ -z "$PROVIDER" ]] && ps_fatal "could not create provider account"
    [[ ${#PROVIDER} -eq 64 ]] && PROVIDER="$(to_base58 "$PROVIDER")"
    logoscore call logos_execution_zone save >/dev/null 2>&1 || true
  else
    OWNER="$(parse_new_account "$(logoscore call logos_execution_zone create_account_public 2>/dev/null | tail -1)")"
    [[ -z "$OWNER" ]] && { narr_fail "Could not create owner account"; ps_fatal "could not create owner account"; }
    [[ ${#OWNER} -eq 64 ]] && OWNER="$(to_base58 "$OWNER")"

    PROVIDER="$(parse_new_account "$(logoscore call logos_execution_zone create_account_public 2>/dev/null | tail -1)")"
    [[ -z "$PROVIDER" ]] && { narr_fail "Could not create provider account"; ps_fatal "could not create provider account"; }
    [[ ${#PROVIDER} -eq 64 ]] && PROVIDER="$(to_base58 "$PROVIDER")"
    logoscore call logos_execution_zone save >/dev/null 2>&1 || true
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

# auth_transfer_init — see scripts/lib/auth_transfer.sh (ps_auth_transfer_ensure).

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

# Extract the tx hash from a chainAction response. The field name varies by
# chain/wallet build (tx_hash, txHash, or nested under wallet), so check all
# known shapes (mirrors run_local_e2e.py chain_action_tx_hash). Prints empty on
# miss. extract_tx_hash <json-line>
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

# call_ps <phase> <required:0|1> <op> <params-json> [status-key] [success-label] [verify-fn]
# narr_step should describe intent (→). On success, prints ✓ success-label.
# On failure, prints ✗ phase failed: … and ! clarification (never reuses success-label).
#
# When getTransaction stays null for a submitted tx (observed on public testnet
# for txs that are nevertheless confirmed), an optional verify-fn is polled: if
# it observes the expected on-chain state the phase is recorded as succeeded
# with inclusion:"state_verified" instead of a false timeout failure.
call_ps() {
  local phase="$1" required="$2" op="$3" params="$4" key="${5:-}" success_label="${6:-$phase}" verify_fn="${7:-}"
  local attempt line="" tx_hash=""
  for attempt in 1 2 3 4 5 6; do
    line="$(logoscore call payment_streams_module chainAction "$op" "$params" 2>/dev/null | tail -1)"
    if inner_status_ok "$line" "$key"; then
      tx_hash="$(extract_tx_hash "$line")"
      if [[ -n "$tx_hash" ]] && ! await_inclusion "$tx_hash"; then
        if [[ -n "$verify_fn" ]] && ps_poll_verify "$verify_fn"; then
          emit_phase "$phase" true "{\"op\":\"$op\",\"attempt\":$attempt,\"tx_hash\":\"$tx_hash\",\"inclusion\":\"state_verified\"}"
          narr_ok "$success_label"
          if [[ -n "$tx_hash" ]]; then
            narr_value "tx published on chain: $tx_hash (verified via state read; getTransaction returned null)"
          fi
          sync_wallet
          echo "$line"
          return 0
        fi
        emit_phase "$phase" false "{\"op\":\"$op\",\"attempt\":$attempt,\"tx_hash\":\"$tx_hash\",\"inclusion\":\"timeout\"}"
        narr_fail "$phase failed: transaction not included on chain"
        narr_hint "Submitted tx_hash=$tx_hash but getTransaction returned null — check mempool, nonce, and sequencer"
        if [[ "$required" == "1" ]]; then
          FAILURES=$((FAILURES + 1))
        fi
        sync_wallet
        echo ""
        return 0
      fi
      emit_phase "$phase" true "{\"op\":\"$op\",\"attempt\":$attempt$( [[ -n "$tx_hash" ]] && echo ",\"tx_hash\":\"$tx_hash\"" )}"
      narr_ok "$success_label"
      if [[ -n "$tx_hash" ]]; then
        narr_value "tx published on chain: $tx_hash"
      fi
      sync_wallet
      echo "$line"
      return 0
    fi
    sleep 8
  done
  emit_phase "$phase" false "{\"op\":\"$op\",\"raw\":$(python3 -c 'import json,sys;print(json.dumps(sys.argv[1]))' "${line:-}")}"
  narr_fail "$phase failed: chainAction rejected or module RPC error"
  narr_hint "Check sequencer height, wallet sync, gas balance, and logoscore module load"
  if [[ "$required" == "1" ]]; then
    FAILURES=$((FAILURES + 1))
  fi
  sync_wallet
  echo ""
  return 0
}

# ps_poll_verify <fn> -> poll a state-verification function (returns 0 when the
# expected on-chain state is observed) for VERIFY_ATTEMPTS attempts. Returns 0
# on success, 1 if the budget is exhausted. Used by call_ps as a fallback when
# getTransaction returns null for a confirmed tx.
ps_poll_verify() {
  local fn="$1" attempt
  for attempt in $(seq 1 "${VERIFY_ATTEMPTS:-6}"); do
    sync_wallet
    if "$fn"; then
      return 0
    fi
    sleep "${VERIFY_SLEEP:-3}"
  done
  return 1
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
if ps_is_local || ps_is_testnet; then
  export LEE_WALLET_HOME_DIR="$WALLET_HOME"
  export WALLET_CONFIG="${WALLET_CONFIG:-$WALLET_HOME/wallet_config.json}"
  export WALLET_STORAGE="${WALLET_STORAGE:-$WALLET_HOME/storage.json}"
  export PS_AT_LOGOSCORE_WALLET_HANDOFF=1
  narr_step "Initializing accounts under authenticated_transfer program"
  if ps_is_privacy_e2e; then
    if ps_auth_transfer_init_one "$PROVIDER" auth_init_provider; then
      narr_ok "Provider verified under authenticated_transfer (private owner skips AT init)"
    else
      narr_fail "authenticated_transfer ensure failed for provider (see artifact auth_init_provider)"
      narr_hint "register_public_account or wallet auth-transfer init did not settle"
      FAILURES=$((FAILURES + 1))
    fi
  elif ps_auth_transfer_ensure "$OWNER" "$PROVIDER"; then
    narr_ok "Owner and provider verified under authenticated_transfer"
  else
    narr_fail "authenticated_transfer ensure failed (see artifact auth_init_* phases)"
    narr_hint "register_public_account or wallet auth-transfer init did not settle"
    FAILURES=$((FAILURES + 1))
  fi
  sync_wallet
fi

if ps_is_local; then
  SCAFFOLD_WALLET="$(ps_lez_cache)/target/release/wallet"
  if [[ -x "$SCAFFOLD_WALLET" ]]; then
    export PATH="$(dirname "$SCAFFOLD_WALLET"):$PATH"
    export LEE_WALLET_HOME_DIR="$WALLET_HOME"
    if ps_is_privacy_e2e; then
      narr_step "Funding public funder and pre-shielding private vault owner"
      owner_target=$((DEPOSIT + 50))
      funder_bal=0
      funder_attempts=0
      funder_max=$((owner_target / 150 + 3))
      while (( funder_bal < owner_target )); do
        funder_attempts=$((funder_attempts + 1))
        if (( funder_attempts > funder_max )); then
          narr_fail "Public funder not funded after $funder_max faucet claims (balance=$funder_bal, target=$owner_target)"
          FAILURES=$((FAILURES + 1))
          break
        fi
        timeout 30 lgs wallet topup --address "Public/$PUBLIC_FUNDER" >/dev/null 2>&1 || true
        sync_wallet
        funder_bal="$(ps_account_balance "$PUBLIC_FUNDER" 2>/dev/null || echo 0)"
      done
      narr_verbose "Funder balance $funder_bal (target $owner_target) after $funder_attempts faucet claim(s)"
      FUNDER_HEX="$(account_id_to_hex "$PUBLIC_FUNDER")"
      OWNER_HEX="$(account_id_to_hex "$OWNER")"
      [[ -z "$FUNDER_HEX" || -z "$OWNER_HEX" ]] && ps_fatal "could not resolve hex account ids for pre-shield"
      if ! ps_pre_shield_to_private_owner "$FUNDER_HEX" "$OWNER_HEX" "$owner_target"; then
        FAILURES=$((FAILURES + 1))
      fi
      timeout 30 lgs wallet topup --address "Public/$PROVIDER" >/dev/null 2>&1 || true
    else
      narr_step "Funding owner and provider for gas"
      # The pinata faucet pays ~150 tokens per claim. The owner must hold at least
      # DEPOSIT (+ buffer) so the deposit instruction can debit it; a single claim
      # is not enough for the fixture's demo_deposit_amount, so claim repeatedly
      # until the owner balance covers the deposit.
      owner_target=$((DEPOSIT + 50))
      owner_attempts=0
      owner_max=$((owner_target / 150 + 3))
      owner_bal="$(ps_account_balance "$OWNER" 2>/dev/null || echo 0)"
      while (( owner_bal < owner_target )); do
        owner_attempts=$((owner_attempts + 1))
        if (( owner_attempts > owner_max )); then
          narr_fail "Owner not funded after $owner_max faucet claims (balance=$owner_bal, target=$owner_target)"
          break
        fi
        timeout 30 lgs wallet topup --address "Public/$OWNER" >/dev/null 2>&1 || true
        sync_wallet
        owner_bal="$(ps_account_balance "$OWNER" 2>/dev/null || echo 0)"
      done
      narr_verbose "Owner balance $owner_bal (target $owner_target) after $owner_attempts faucet claim(s)"
      # Provider only needs gas for the claim signature; one claim is plenty.
      timeout 30 lgs wallet topup --address "Public/$PROVIDER" >/dev/null 2>&1 || true
    fi
  fi
  sync_wallet
fi

if ps_is_testnet; then
  export LEE_WALLET_HOME_DIR="$WALLET_HOME"
  if [[ "${MODULE_E2E_SKIP_FUND:-0}" == "1" ]]; then
    narr_step "Skipping testnet funding (MODULE_E2E_SKIP_FUND=1; assuming pre-funded)"
    narr_hint "Pre-fund with ./scripts/fund-testnet-accounts.sh before the demo"
  else
    narr_step "Funding owner and provider on testnet (wallet pinata)"
    owner_target=$((DEPOSIT + 50))
    owner_bal="0"
    if ! owner_bal="$(ps_fund_testnet_account "$OWNER" "$owner_target" 6)"; then
      narr_fail "Owner balance ${owner_bal:-0} below deposit target $owner_target"
      FAILURES=$((FAILURES + 1))
    else
      narr_verbose "Owner balance $owner_bal (target $owner_target)"
    fi
    provider_bal="0"
    if ! provider_bal="$(ps_fund_testnet_account "$PROVIDER" "${PROVIDER_MIN:-50}" 3)"; then
      narr_verbose "Provider balance ${provider_bal:-0} (min ${PROVIDER_MIN:-50})"
    else
      narr_verbose "Provider balance after pinata: $provider_bal (min ${PROVIDER_MIN:-50})"
    fi
    if [[ -z "$provider_bal" || "$provider_bal" == "0" ]]; then
      narr_fail "Provider has zero balance after pinata (claim signer needs gas)"
      FAILURES=$((FAILURES + 1))
    fi
  fi
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
  for attempt in $(seq 1 "${POLL_READ_ATTEMPTS:-6}"); do
    sync_wallet
    out="$("$@")"
    if [[ -n "$out" ]]; then
      printf '%s' "$out"
      return 0
    fi
    sleep "${POLL_READ_SLEEP:-5}"
  done
  return 1
}

# State-verification functions used by call_ps when getTransaction returns null
# for a confirmed tx. Each returns 0 once the expected on-chain state is
# observed via the module read ops (getVaultStatus / getStreamStatus) or the
# sequencer getAccount. They close over OWNER / VAULT_ID / STREAM_ID globals.
verify_vault_init()      { [[ -n "$(read_vault "$OWNER" "$VAULT_ID")" ]]; }

verify_deposit() {
  local bal
  read -r bal _ <<< "$(read_vault "$OWNER" "$VAULT_ID")"
  [[ -n "${bal:-}" && "$bal" -ge "$DEPOSIT" ]]
}

verify_create_stream() {
  local read _ _ st
  read="$(read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"
  [[ -n "$read" ]] || return 1
  read -r _ _ st <<< "$read"
  [[ "${st:-}" == "0" ]]   # Active
}

verify_close_stream() {
  local read _ _ st
  read="$(read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"
  [[ -n "$read" ]] || return 1
  read -r _ _ st <<< "$read"
  [[ "${st:-}" == "2" ]]   # Closed
}

# Claim credits the provider via authenticated_transfer; confirm the provider
# account balance rose above the snapshot captured before the claim call.
verify_claim() {
  local bal
  bal="$(ps_account_balance "$PROVIDER" || echo 0)"
  [[ -n "${bal:-}" && "$bal" -gt "${PRE_CLAIM_BALANCE:-0}" ]]
}

# ---------------------------------------------------------------------------
# PHASE: Vault Initialization
# ---------------------------------------------------------------------------
narr_phase "Vault Initialization"

# Testnet: resolve a fresh empty vault under the fixture owner so each run
# starts from a clean vault/stream instead of reusing the pinned vault 0 /
# stream 0, which holds stale, already-closed/claimed state from prior runs.
# Localnet is exempt: its isolated per-run wallet already guarantees fresh
# vault 0 / stream 0. read_vault returns empty for an uninitialized vault
# config account, so the scan finds the first empty vault id under the owner.
if ps_is_testnet && [[ "$MODULE_E2E_FRESH_VAULT" != "0" ]] \
   && [[ "$VAULT_ID_EXPLICIT" != "x" ]]; then
  narr_step "Resolving fresh vault under owner $OWNER"
  fresh_vid="$VAULT_ID"
  while [[ -n "$(read_vault "$OWNER" "$fresh_vid")" ]]; do
    fresh_vid=$((fresh_vid + 1))
  done
  VAULT_ID="$fresh_vid"
  if [[ "$STREAM_ID_EXPLICIT" != "x" ]]; then
    fresh_sid="$STREAM_ID"
  else
    # A freshly resolved vault is uninitialized, so its first stream is 0.
    fresh_sid=0
  fi
  STREAM_ID="$fresh_sid"
  emit_phase plan_vault true "{\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"source\":\"vault_config_scan\",\"owner\":\"$OWNER\"}"
  narr_ok "Fresh vault resolved: vault_id=$VAULT_ID stream_id=$STREAM_ID"
  narr_value "owner=$OWNER provider=$PROVIDER vault=$VAULT_ID stream=$STREAM_ID chain=${CHAIN:-local}"
fi

narr_step "Alice creates vault $VAULT_ID"
if ps_is_privacy_e2e; then
  call_ps vault_init 1 initializeVault "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"privacy_tier\":1}")" "" "Vault $VAULT_ID created on chain (PseudonymousFunder)" verify_vault_init
else
  call_ps vault_init 1 initializeVault "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID}")" "" "Vault $VAULT_ID created on chain" verify_vault_init
fi

narr_step "Depositing $DEPOSIT tokens into vault"
DEPOSIT_LINE="$(call_ps deposit 1 deposit "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"amount_lo\":$DEPOSIT,\"amount_hi\":0}")" "" "Deposit transaction included on chain" verify_deposit)"

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
call_ps create_stream 1 createStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"provider\":\"$PROVIDER\",\"rate\":$RATE,\"allocation_lo\":$ALLOCATION,\"allocation_hi\":0}")" "" "Stream $STREAM_ID created (ACTIVE)" verify_create_stream

# ---------------------------------------------------------------------------
# PHASE: Stream Lifecycle (optional top-up)
# ---------------------------------------------------------------------------
if [[ "$MODULE_E2E_TOPUP" == "1" ]]; then
  narr_phase "Stream Lifecycle"

  narr_step "Topping up stream $STREAM_ID by $TOPUP_INCREASE token"
  # Capture on-chain allocation (accrued + unaccrued) before the top-up so we can
  # verify the delta equals the top-up amount.
  PRE_ALLOC=0
  if TOPUP_PRE="$(poll_read read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"; then
    read -r PRE_ACC PRE_UNC _ <<< "$TOPUP_PRE"
    PRE_ALLOC=$((PRE_ACC + PRE_UNC))
  fi
  TOPUP_LINE="$(call_ps topup_stream 1 topUpStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"increase_lo\":$TOPUP_INCREASE,\"increase_hi\":0}")" "" "Top-up transaction included on chain")"
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
    emit_phase topup_allocation false "{\"error\":\"read_failed\"}"
    narr_fail "Could not read allocation on chain"
    narr_hint "getStreamStatus returned no data — check wallet sync and sequencer"
  fi
else
  narr_verbose "Skipping stream top-up (set MODULE_E2E_TOPUP=1 to include topUpStream)"
fi

# ---------------------------------------------------------------------------
# PHASE: Accrual
# ---------------------------------------------------------------------------
narr_phase "Accrual"

narr_step "Waiting for funds to accrue (rate=$RATE tokens/sec)"
narr_verbose "Accrual is timestamp-based: derived from on-chain accrued_as_of field."
if ps_is_testnet; then
  narr_verbose "Public testnet often advances a block every ~15–60s; accrual follows chain time, not wall clock."
else
  narr_verbose "Localnet folded clock advances quickly between polls."
fi

narr_value "Need at least $MIN_ACCRUED tokens accrued before claim"

ACCRUED=0
UNACCRUED=0
for attempt in $(seq 1 "${ACCRUAL_ATTEMPTS:-30}"); do
  sync_wallet
  STREAM_READ="$(read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"
  if [[ -n "$STREAM_READ" ]]; then
    read -r ACCRUED UNACCRUED _ <<< "$STREAM_READ"
    if [[ "$ACCRUED" -ge "$MIN_ACCRUED" ]]; then
      break
    fi
  fi
  sleep "${ACCRUAL_POLL_SLEEP:-5}"
done

emit_phase accrual "$([[ "$ACCRUED" -ge "$MIN_ACCRUED" ]] && echo true || echo false)" "{\"accrued_lo\":$ACCRUED,\"unaccrued_lo\":$UNACCRUED,\"min_required\":$MIN_ACCRUED,\"attempts\":$attempt}"
if [[ "$ACCRUED" -ge "$MIN_ACCRUED" ]]; then
  narr_ok "Accrued: $ACCRUED tokens after ${attempt} poll(s) (unaccrued: $UNACCRUED)"
else
  narr_fail "Insufficient accrual: $ACCRUED tokens (need $MIN_ACCRUED)"
  narr_hint "Check sequencer is advancing and clock is synced"
fi

# ---------------------------------------------------------------------------
# PHASE: Settlement (close then claim residual)
# ---------------------------------------------------------------------------
if [[ "$MODULE_E2E_SKIP_CLOSE" == "1" ]]; then
  narr_verbose "Skipping settlement (MODULE_E2E_SKIP_CLOSE=1); close and claim omitted"
  emit_phase close_stream true "{\"skipped\":true,\"reason\":\"MODULE_E2E_SKIP_CLOSE\"}"
  emit_phase claim true "{\"skipped\":true,\"reason\":\"MODULE_E2E_SKIP_CLOSE\"}"
else
  narr_phase "Close"

  narr_step "Alice closes stream $STREAM_ID, reclaims unspent allocation"
  CLOSE_LINE="$(call_ps close_stream 1 closeStream "$(j "{\"signer\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"authority\":\"$PROVIDER\"}")" "" "Close transaction included on chain" verify_close_stream)"

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

  narr_phase "Claim"

  CLAIM_ACCRUED=""
  if CLAIM_STREAM="$(poll_read read_stream "$OWNER" "$VAULT_ID" "$STREAM_ID")"; then
    read -r CLAIM_ACCRUED _ _ <<< "$CLAIM_STREAM"
  fi
  CLAIM_ACCRUED="${CLAIM_ACCRUED:-0}"

  if [[ "${CLAIM_ACCRUED:-0}" -le 0 ]]; then
    emit_phase claim true "{\"skipped\":true,\"reason\":\"zero_accrued\"}"
    narr_ok "No residual accrued to claim after close"
  else
    PRE_CLAIM_BALANCE="$(ps_account_balance "$PROVIDER" || echo 0)"
    PRE_CLAIM_VAULT=0
    if CLAIM_PRE_VAULT="$(poll_read read_vault "$OWNER" "$VAULT_ID")"; then
      read -r PRE_CLAIM_VAULT _ <<< "$CLAIM_PRE_VAULT"
    fi

    narr_step "Bob claims residual accrued ($CLAIM_ACCRUED) from closed stream $STREAM_ID"
    CLAIM_LINE="$(call_ps claim 1 claim "$(j "{\"owner\":\"$OWNER\",\"provider\":\"$PROVIDER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}")" "" "Claim transaction included on chain" verify_claim)"

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
      narr_fail "Claim failed: provider balance did not increase on chain"
      narr_hint "If claim tx included, re-read getAccount and getVaultStatus after wallet sync"
    fi
  fi
fi

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
