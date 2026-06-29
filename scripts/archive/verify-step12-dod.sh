#!/usr/bin/env bash
# Verify Step 12 definition of done (see docs/step12-user-eligibility.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export REPO="${REPO:-$REPO_ROOT}"
export MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
VERIFY_LOGOSCORE="${VERIFY_LOGOSCORE:-1}"
PERSIST_DIR="${PERSIST_DIR:-$REPO/.scaffold/step12-persist}"
REQUIRE_STREAM_PROOF="${REQUIRE_STREAM_PROOF:-0}"
GUEST_BIN="${PAYMENT_STREAMS_GUEST_BIN:-$REPO/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
LOGOSCORE_E2E_TIMEOUT="${LOGOSCORE_E2E_TIMEOUT:-240}"

fail=0
ok() { echo "PASS: $*"; }
bad() { echo "FAIL: $*"; fail=1; }
skip() { echo "SKIP: $*"; }

echo "=== Step 12 DoD verification ==="

if [[ -f docs/step12-user-eligibility.md ]]; then
  ok "Step 12 runbook present"
else
  bad "missing docs/step12-user-eligibility.md"
fi

echo "--- Rust / FFI (offline) ---"
if cargo test -p lez-payment-streams-core store_eligibility_digest_matches_n8_reference_fixture --quiet 2>/dev/null; then
  ok "N8 digest fixture test"
else
  bad "N8 digest fixture test"
fi

if cargo test -p lez-payment-streams-ffi ffi_generate_session_keypair_sign_verify_round_trip --quiet 2>/dev/null &&
   cargo test -p lez-payment-streams-ffi ffi_eligibility_proof_wrapper_round_trip --quiet 2>/dev/null; then
  ok "FFI session keygen + eligibility wrapper tests"
else
  bad "FFI session keygen + eligibility wrapper tests"
fi

N8_WIRE_HEX="$(cargo run -q -p lez-payment-streams-core --bin n8_canonical_wire_hex 2>/dev/null || true)"
N8_WIRE_LEN="${#N8_WIRE_HEX}"
if [[ -n "$N8_WIRE_HEX" && "$N8_WIRE_LEN" -gt 64 ]]; then
  ok "N8 canonical wire hex tool ($N8_WIRE_LEN chars)"
else
  bad "N8 canonical wire hex tool"
fi

PS_PLUGIN="$MODULES/payment_streams_module/payment_streams_module_plugin.so"
if [[ -f "$PS_PLUGIN" ]]; then
  ok "payment_streams_module_plugin.so installed"
else
  bad "payment_streams_module_plugin.so not under MODULES=$MODULES (build/install .lgx)"
fi

LM_BIN="${LM_BIN:-}"
if [[ -z "$LM_BIN" ]]; then
  _local_lm="$HOME/Downloads/software/waku/lez-related/logos-cli/lm/bin/lm"
  if [[ -x "$_local_lm" ]]; then
    LM_BIN="$_local_lm"
  fi
fi

_lm_methods() {
  local plugin="$1"
  if [[ -n "$LM_BIN" ]]; then
    "$LM_BIN" methods "$plugin" 2>/dev/null
  else
    nix shell github:logos-co/logos-module#lm --command bash -c "lm methods '$plugin'" 2>/dev/null
  fi
}

for method in registerProviderMapping prepareEligibilityProofWithStreamProposalForStoreQuery prepareEligibilityProofWithStreamProofForStoreQuery listMyStreams rediscoverStreams; do
  if _lm_methods "$PS_PLUGIN" | grep -q "$method"; then
    ok "lm methods lists $method"
  else
    bad "lm methods missing $method"
  fi
done

if [[ "$VERIFY_LOGOSCORE" != "1" ]]; then
  skip "VERIFY_LOGOSCORE=0 — skipping logoscore eligibility smoke"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -f "$MANIFEST" ]]; then
  skip "logoscore smoke (no fixture manifest)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if ! curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' >/dev/null; then
  skip "logoscore smoke (sequencer not reachable; ./scripts/demo-localnet-fresh.sh)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ -z "$N8_WIRE_HEX" ]]; then
  bad "prepareEligibility smoke (missing N8 wire hex)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -f "$WALLET_CONFIG" || ! -f "$WALLET_STORAGE" ]]; then
  skip "logoscore smoke (wallet config/storage missing)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -f "$GUEST_BIN" ]]; then
  bad "guest ELF missing at PAYMENT_STREAMS_GUEST_BIN=$GUEST_BIN (make build)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

rm -rf "$PERSIST_DIR"
mkdir -p "$PERSIST_DIR"

if [[ "$REQUIRE_STREAM_PROOF" == "1" ]]; then
  echo "--- logoscore stream_proof (via step12-topup-and-prepare.sh) ---"
  if MODULES="$MODULES" WALLET_CONFIG="$WALLET_CONFIG" WALLET_STORAGE="$WALLET_STORAGE" \
    PAYMENT_STREAMS_GUEST_BIN="$GUEST_BIN" PERSIST_DIR="$PERSIST_DIR" MANIFEST="$MANIFEST" \
    TRY_TOPUP=1 PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF=0 \
    "$REPO_ROOT/scripts/step12-topup-and-prepare.sh" >/tmp/step12-verify-topup.log 2>&1; then
    ok "registerProviderMapping + topUpStream + prepareEligibility stream_proof"
  else
    bad "stream_proof path failed (see /tmp/step12-verify-topup.log)"
    tail -15 /tmp/step12-verify-topup.log >&2 || true
  fi
  if find "$PERSIST_DIR" -name payment_streams_state.json 2>/dev/null | grep -q .; then
    ok "persistence file written under instance path"
  else
    bad "missing payment_streams_state.json under $PERSIST_DIR"
  fi
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

PROVIDER_PEER_ID="${PROVIDER_PEER_ID:-step12-demo-provider-peer}"
PROVIDER_B58="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['provider_account_id'])")"

SMOKE_FILE="$(mktemp)"
trap 'rm -f "$SMOKE_FILE"' EXIT

timeout "$LOGOSCORE_E2E_TIMEOUT" nix shell github:logos-co/logos-logoscore-cli --command bash -c "
  set -uo pipefail
  export MODULES='$MODULES'
  export PAYMENT_STREAMS_GUEST_BIN='$GUEST_BIN'
  logoscore stop 2>/dev/null || true
  sleep 2
  logoscore -D -m \"\$MODULES\" --persistence-path '$PERSIST_DIR' -q &
  sleep 4
  logoscore load-module logos_execution_zone || true
  logoscore load-module payment_streams_module || true
  logoscore call logos_execution_zone open '$WALLET_CONFIG' '$WALLET_STORAGE' || true
  height=\$(curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
    -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getLastBlockId\",\"params\":[]}' \
    | python3 -c 'import json,sys; d=json.load(sys.stdin); r=d.get(\"result\"); print(r if isinstance(r,int) else (r or \"\"))' 2>/dev/null || true)
  if [[ -n \"\$height\" ]]; then
    logoscore call logos_execution_zone sync_to_block \"\$height\" >/dev/null 2>&1 || true
    sleep 2
  fi
  echo REG:\$(logoscore call payment_streams_module registerProviderMapping '$PROVIDER_PEER_ID' '$PROVIDER_B58' 2>&1 | tail -1)
  height=\$(curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
    -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getLastBlockId\",\"params\":[]}' \
    | python3 -c 'import json,sys; d=json.load(sys.stdin); r=d.get(\"result\"); print(r if isinstance(r,int) else (r or \"\"))' 2>/dev/null || true)
  if [[ -n \"\$height\" ]]; then
    logoscore call logos_execution_zone sync_to_block \"\$height\" >/dev/null 2>&1 || true
  fi
  sleep 2
  echo PREP:\$(logoscore call payment_streams_module prepareEligibilityProofWithStreamProposalForStoreQuery '$N8_WIRE_HEX' '$PROVIDER_PEER_ID' 2>&1 | tail -1)
  echo LIST:\$(logoscore call payment_streams_module listMyStreams \"\$(python3 -c \"import json; print(json.load(open('$MANIFEST'))['vault_id'])\")\" 2>&1 | tail -1)
  logoscore stop 2>/dev/null || true
" >"$SMOKE_FILE" 2>&1 || echo LOGOSCORE_TIMEOUT >>"$SMOKE_FILE"

if rg -q 'LOGOSCORE_TIMEOUT' "$SMOKE_FILE" 2>/dev/null; then
  bad "logoscore eligibility smoke timed out (${LOGOSCORE_E2E_TIMEOUT}s)"
  tail -25 "$SMOKE_FILE" >&2 || true
else
  REG_LINE="$(rg '^REG:' "$SMOKE_FILE" | tail -1 | sed 's/^REG://')"
  if echo "$REG_LINE" | grep -q '"status":"ok"'; then
    ok "registerProviderMapping"
  else
    bad "registerProviderMapping failed"
    tail -15 "$SMOKE_FILE" >&2 || true
  fi

  PREP_LINE="$(rg '^PREP:' "$SMOKE_FILE" | tail -1 | sed 's/^PREP://')"
  if echo "$PREP_LINE" | python3 -c "
import json,sys
line=sys.stdin.read().strip()
outer=json.loads(line)
inner=json.loads(outer.get('result','{}'))
if inner.get('status')=='ok' and inner.get('kind')=='stream_proposal':
  sys.exit(0)
sys.exit(1)
" 2>/dev/null; then
    ok "prepareEligibilityProofWithStreamProposalForStoreQuery stream_proposal (vault baseline)"
  elif echo "$PREP_LINE" | python3 -c "
import json,sys
line=sys.stdin.read().strip()
outer=json.loads(line)
inner=json.loads(outer.get('result','{}'))
if inner.get('code')=='NO_ELIGIBLE_VAULT':
  sys.exit(2)
sys.exit(1)
" 2>/dev/null; then
    skip "prepareEligibilityProofWithStreamProposalForStoreQuery (NO_ELIGIBLE_VAULT — restore vault snapshot)"
  else
    bad "prepareEligibilityProofWithStreamProposalForStoreQuery failed (expected stream_proposal on vault-only manifest)"
    echo "$PREP_LINE" >&2
  fi

  LIST_LINE="$(rg '^LIST:' "$SMOKE_FILE" | tail -1 | sed 's/^LIST://')"
  if echo "$LIST_LINE" | grep -qE 'streams|\\\"streams\\\"' && echo "$LIST_LINE" | grep -qE '"status":"ok"|\\\"status\\\":\\\"ok\\\"'; then
    ok "listMyStreams"
  else
    bad "listMyStreams failed"
    echo "$LIST_LINE" >&2
  fi
fi

if find "$PERSIST_DIR" -name payment_streams_state.json 2>/dev/null | grep -q .; then
  ok "persistence file written under instance path"
else
  bad "missing payment_streams_state.json under $PERSIST_DIR"
fi

if [[ "${SKIP_VERIFY_STREAM_TEARDOWN:-0}" != "1" ]] &&
  python3 -c "import json; m=json.load(open('$MANIFEST')); exit(0 if m.get('stream_id') is not None else 1)" 2>/dev/null; then
  FIXTURE_MANIFEST="$MANIFEST" "$REPO_ROOT/scripts/demo-stream-teardown-localnet.sh" || true
fi

echo "=== done (exit $fail) ==="
exit "$fail"
