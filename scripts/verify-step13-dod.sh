#!/usr/bin/env bash
# Verify Step 13 definition of done (integration-index.md Step 13).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export REPO="${REPO:-$REPO_ROOT}"
export MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
VERIFY_LOGOSCORE="${VERIFY_LOGOSCORE:-1}"
PERSIST_DIR="${PERSIST_DIR:-$REPO/.scaffold/step13-persist}"
GUEST_BIN="${PAYMENT_STREAMS_GUEST_BIN:-$REPO/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
LOGOSCORE_E2E_TIMEOUT="${LOGOSCORE_E2E_TIMEOUT:-240}"
PROVIDER_PEER_ID="${PROVIDER_PEER_ID:-step13-demo-provider-peer}"

fail=0
ok() { echo "PASS: $*"; }
bad() { echo "FAIL: $*"; fail=1; }
skip() { echo "SKIP: $*"; }

echo "=== Step 13 DoD verification ==="

if cargo test -p lez-payment-streams-ffi ffi_parse_eligibility_proof_bytes_round_trip --quiet 2>/dev/null; then
  ok "FFI parse eligibility proof test"
else
  bad "FFI parse eligibility proof test"
fi

PS_PLUGIN="$MODULES/payment_streams_module/payment_streams_module_plugin.so"
if [[ -f "$PS_PLUGIN" ]]; then
  ok "payment_streams_module_plugin.so installed"
else
  bad "payment_streams_module_plugin.so not under MODULES=$MODULES"
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

if _lm_methods "$PS_PLUGIN" | grep -q verifyEligibilityForStoreQuery; then
  ok "lm methods lists verifyEligibilityForStoreQuery"
else
  bad "lm methods missing verifyEligibilityForStoreQuery"
fi

if [[ "$VERIFY_LOGOSCORE" != "1" ]]; then
  skip "VERIFY_LOGOSCORE=0 — skipping logoscore cross-test"
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
  skip "logoscore smoke (sequencer not reachable)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

N8_WIRE_HEX="$(cargo run -q -p lez-payment-streams-core --bin n8_canonical_wire_hex 2>/dev/null || true)"
N8_WIRE_LEN="${#N8_WIRE_HEX}"
if [[ -n "$N8_WIRE_HEX" && "$N8_WIRE_LEN" -gt 64 ]]; then
  :
else
  bad "N8 wire hex tool"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

PROVIDER_B58="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['provider_account_id'])")"

if [[ ! -f "$WALLET_CONFIG" || ! -f "$WALLET_STORAGE" || ! -f "$GUEST_BIN" ]]; then
  skip "logoscore smoke (wallet or guest ELF missing)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if ! python3 -c "import json; m=json.load(open('$MANIFEST')); exit(0 if m.get('stream_id') is not None else 1)" 2>/dev/null; then
  echo "--- create stream for Step 13 proof path ---"
  "$REPO_ROOT/scripts/create-localnet-stream-fixture.sh"
fi

STREAM_ID="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['stream_id'])")"

rm -rf "$PERSIST_DIR"
mkdir -p "$PERSIST_DIR"

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
  logoscore call payment_streams_module registerProviderMapping '$PROVIDER_PEER_ID' '$PROVIDER_B58' >/dev/null
  logoscore call payment_streams_module rediscoverStreams \"\$(python3 -c \"import json; print(json.load(open('$MANIFEST'))['vault_id'])\")\" >/dev/null
  sleep 3
  PREP=\$(logoscore call payment_streams_module prepareEligibilityProofWithStreamProofForStoreQuery '$N8_WIRE_HEX' '$PROVIDER_PEER_ID' '$STREAM_ID' 2>&1 | tail -1)
  echo PREP:\$PREP
  BYTES_HEX=\$(echo \"\$PREP\" | python3 -c \"import json,sys; d=json.loads(sys.stdin.read()); inner=json.loads(d.get('result','{}')); print(inner.get('bytes_hex',''))\" 2>/dev/null || true)
  echo BYTES:\$BYTES_HEX
  if [[ -n \"\$BYTES_HEX\" ]]; then
    echo VERIFY:\$(logoscore call payment_streams_module verifyEligibilityForStoreQuery \"\$BYTES_HEX\" '$N8_WIRE_HEX' '$PROVIDER_PEER_ID' 2>&1 | tail -1)
    TAMPERED=\$(python3 -c \"h='$N8_WIRE_HEX'; print(h[:-2]+('a' if h[-2:]!='aa' else 'bb'))\")
    echo BAD:\$(logoscore call payment_streams_module verifyEligibilityForStoreQuery \"\$BYTES_HEX\" \"\$TAMPERED\" '$PROVIDER_PEER_ID' 2>&1 | tail -1)
  fi
  logoscore stop 2>/dev/null || true
" >"$SMOKE_FILE" 2>&1 || echo LOGOSCORE_TIMEOUT >>"$SMOKE_FILE"

if rg -q 'LOGOSCORE_TIMEOUT' "$SMOKE_FILE" 2>/dev/null; then
  bad "logoscore Step 13 smoke timed out (${LOGOSCORE_E2E_TIMEOUT}s)"
  tail -20 "$SMOKE_FILE" >&2 || true
else
  PREP_LINE="$(rg '^PREP:' "$SMOKE_FILE" 2>/dev/null | tail -1 | sed 's/^PREP://' || true)"
  BYTES_LINE="$(rg '^BYTES:' "$SMOKE_FILE" 2>/dev/null | tail -1 | sed 's/^BYTES://' || true)"

  if echo "$PREP_LINE" | python3 -c "
import json,sys
line=sys.stdin.read().strip()
if not line: sys.exit(1)
outer=json.loads(line)
inner=json.loads(outer.get('result','{}'))
if inner.get('code')=='STREAM_DEPLETED':
  sys.exit(2)
sys.exit(0 if inner.get('status')=='ok' and inner.get('bytes_hex') else 1)
" 2>/dev/null; then
    :
  elif echo "$PREP_LINE" | python3 -c "
import json,sys
line=sys.stdin.read().strip()
outer=json.loads(line)
inner=json.loads(outer.get('result','{}'))
sys.exit(0 if inner.get('code')=='STREAM_DEPLETED' else 1)
" 2>/dev/null; then
    skip "prepare → verify cross-test (stream depleted; create fresh stream)"
    echo "=== done (exit $fail) ==="
    exit "$fail"
  else
    bad "prepareEligibilityProofWithStreamProofForStoreQuery failed before verify"
    echo "$PREP_LINE" >&2
    tail -15 "$SMOKE_FILE" >&2 || true
  fi

  if [[ -z "$BYTES_LINE" ]]; then
    bad "prepare returned no bytes_hex for verify cross-test"
  else
  VERIFY_LINE="$(rg '^VERIFY:' "$SMOKE_FILE" 2>/dev/null | tail -1 | sed 's/^VERIFY://' || true)"
  if echo "$VERIFY_LINE" | python3 -c "
import json,sys
line=sys.stdin.read().strip()
outer=json.loads(line)
inner=json.loads(outer.get('result','{}'))
sys.exit(0 if inner.get('status')=='ok' and inner.get('eligibility')=='OK' else 1)
" 2>/dev/null; then
    ok "prepare → verify cross-test eligibility OK"
  else
    bad "prepare → verify cross-test failed"
    echo "$VERIFY_LINE" >&2
    tail -15 "$SMOKE_FILE" >&2 || true
  fi

  BAD_LINE="$(rg '^BAD:' "$SMOKE_FILE" 2>/dev/null | tail -1 | sed 's/^BAD://' || true)"
  if echo "$BAD_LINE" | grep -q 'PROOF_INVALID' || echo "$BAD_LINE" | python3 -c "
import json,sys
line=sys.stdin.read().strip()
outer=json.loads(line)
inner=json.loads(outer.get('result','{}'))
sys.exit(0 if inner.get('eligibility')=='PROOF_INVALID' else 1)
" 2>/dev/null; then
    ok "tampered canonical → PROOF_INVALID"
  else
    bad "tampered canonical negative test"
    echo "$BAD_LINE" >&2
  fi
  fi
fi

echo "=== done (exit $fail) ==="
exit "$fail"
