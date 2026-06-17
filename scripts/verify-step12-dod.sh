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

for method in registerProviderMapping prepareEligibilityForStoreQuery listMyStreams rediscoverStreams; do
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

if [[ ! -f "$WALLET_CONFIG" || ! -f "$WALLET_STORAGE" ]]; then
  skip "logoscore smoke (wallet config/storage missing)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

PROVIDER_PEER_ID="${PROVIDER_PEER_ID:-step12-demo-provider-peer}"
PROVIDER_B58="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['provider_account_id'])")"

rm -rf "$PERSIST_DIR"
mkdir -p "$PERSIST_DIR"

if ! command -v logoscore >/dev/null 2>&1; then
  skip "logoscore not in PATH"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

logoscore -D -m "$MODULES" --persistence-path "$PERSIST_DIR" -q &
LC_PID=$!
cleanup() { logoscore stop 2>/dev/null || kill "$LC_PID" 2>/dev/null || true; }
trap cleanup EXIT
sleep 3

logoscore load-module logos_execution_zone
logoscore load-module payment_streams_module
logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE"

REG_JSON="$(logoscore call payment_streams_module registerProviderMapping "$PROVIDER_PEER_ID" "$PROVIDER_B58")"
if echo "$REG_JSON" | grep -q '"status":"ok"'; then
  ok "registerProviderMapping"
else
  bad "registerProviderMapping: $REG_JSON"
fi

if [[ -z "$N8_WIRE_HEX" ]]; then
  bad "prepareEligibility (missing N8 wire hex)"
else
  PREP_JSON="$(logoscore call payment_streams_module prepareEligibilityForStoreQuery "$N8_WIRE_HEX" "$PROVIDER_PEER_ID")"
  if echo "$PREP_JSON" | grep -qE 'stream_proof|\\\"kind\\\":\\\"stream_proof\\\"'; then
    ok "prepareEligibilityForStoreQuery stream_proof (seeded stream)"
  elif echo "$PREP_JSON" | grep -q 'STREAM_DEPLETED'; then
    if [[ "$REQUIRE_STREAM_PROOF" == "1" ]]; then
      bad "prepareEligibilityForStoreQuery STREAM_DEPLETED (run ./scripts/demo-localnet-fresh.sh)"
    else
      skip "prepareEligibilityForStoreQuery (stream 0 depleted; ./scripts/demo-localnet-fresh.sh then retry or REQUIRE_STREAM_PROOF=1)"
    fi
  else
    bad "prepareEligibilityForStoreQuery: $PREP_JSON"
  fi
fi

VAULT_ID="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['vault_id'])")"
LIST_JSON="$(logoscore call payment_streams_module listMyStreams "$VAULT_ID")"
if echo "$LIST_JSON" | grep -qE 'streams|\\\"streams\\\"'; then
  if echo "$LIST_JSON" | grep -qE '"status":"ok"|\\\"status\\\":\\\"ok\\\"'; then
    ok "listMyStreams"
  else
    bad "listMyStreams: $LIST_JSON"
  fi
else
  bad "listMyStreams: $LIST_JSON"
fi

if find "$PERSIST_DIR" -name payment_streams_state.json 2>/dev/null | grep -q .; then
  ok "persistence file written under instance path"
else
  bad "missing payment_streams_state.json under $PERSIST_DIR"
fi

logoscore stop
trap - EXIT

echo "=== done (exit $fail) ==="
exit "$fail"
