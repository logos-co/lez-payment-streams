#!/usr/bin/env bash
# Verify Step 11a definition of done (see docs/step11a-chain-reads.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export REPO="${REPO:-$REPO_ROOT}"
export MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
export PS_LGX="${PS_LGX:-$(readlink -f "$REPO/result"/*.lgx 2>/dev/null || true)}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
VERIFY_LOGOSCORE="${VERIFY_LOGOSCORE:-1}"

fail=0
ok() { echo "PASS: $*"; }
bad() { echo "FAIL: $*"; fail=1; }
skip() { echo "SKIP: $*"; }

echo "=== Step 11a DoD verification ==="

LEZ_PIN="$(grep -E '^pin = ' scaffold.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')"
if [[ "$LEZ_PIN" == a8c81f5445166b22672a614b159a1c38a5907a65 ]]; then
  ok "scaffold.toml LEZ pin is main (491 merge)"
else
  bad "scaffold.toml LEZ pin expected a8c81f54… got $LEZ_PIN"
fi

if rg -q 'a8c81f5445166b22672a614b159a1c38a5907a65' nix/payment-streams-ffi.nix; then
  ok "payment-streams-ffi.nix pins LEZ main"
else
  bad "payment-streams-ffi.nix missing LEZ main rev"
fi

if [[ -f docs/step11a-chain-reads.md ]]; then
  ok "Step 11a runbook present"
else
  bad "missing docs/step11a-chain-reads.md"
fi

PLUGIN="$MODULES/payment_streams_module/payment_streams_module_plugin.so"
if [[ ! -f "$PLUGIN" ]]; then
  if [[ -f "$PS_LGX" ]]; then
    skip "payment_streams_module not installed; build with nix build ./logos-payment-streams-module#lgx && lgpm install"
  else
    bad "missing PS .lgx (nix build ./logos-payment-streams-module#lgx)"
  fi
else
  ok "payment_streams_module plugin present"
fi

if [[ -f "$PLUGIN" ]] && nix shell github:logos-co/logos-module#lm --command bash -c "
  set -euo pipefail
  out=\$(lm methods '$PLUGIN')
  echo \"\$out\" | rg -q 'readVaultConfigDecoded'
  echo \"\$out\" | rg -q 'readClock10Decoded'
  echo \"\$out\" | rg -q 'chainAction'
"; then
  ok "lm methods lists Step 11a read API"
else
  if [[ -f "$PLUGIN" ]]; then
    bad "lm methods missing Step 11a methods (reinstall PS .lgx after rebuild)"
  else
    skip "lm methods check (no plugin installed)"
  fi
fi

if [[ "$VERIFY_LOGOSCORE" != "1" ]]; then
  skip "VERIFY_LOGOSCORE=0 — skipping logoscore E2E"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -f "$MANIFEST" ]]; then
  skip "logoscore E2E (no fixture manifest)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if python3 -c "
import json,sys
m=json.load(open('$MANIFEST'))
for k in ('vault_config_account_id','vault_holding_account_id','stream_config_account_id'):
  v=m.get(k,'')
  if not v or v.startswith('REPLACE_'):
    sys.exit(1)
" 2>/dev/null; then
  ok "fixture manifest has seeded account ids"
else
  bad "fixtures/localnet.json missing real PDAs (run ./scripts/seed-localnet-fixture.sh)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if ! curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getBlockHeight","params":[]}' >/dev/null; then
  skip "logoscore E2E (sequencer not reachable)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -f "$MODULES/logos_execution_zone/logos_execution_zone_plugin.so" ]]; then
  bad "logos_execution_zone not installed (Step 10b)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

read -r VC VH SC CLOCK <<<"$(python3 -c "
import json
m=json.load(open('$MANIFEST'))
print(m['vault_config_account_id'], m['vault_holding_account_id'], m['stream_config_account_id'], m.get('clock_10_account_id',''))
")"

E2E_FILE="$(mktemp)"
E2E_TIMEOUT="${LOGOSCORE_E2E_TIMEOUT:-180}"
WALLET_E2E_DIR="${WALLET_E2E_DIR:-$REPO/.scaffold/wallet-logoscore-e2e}"
WALLET_E2E_PASSWORD="${WALLET_E2E_PASSWORD:-scaffold-local-dev}"
trap 'rm -f "$E2E_FILE"' EXIT

mkdir -p "$WALLET_E2E_DIR"
cp "$WALLET_CONFIG" "$WALLET_E2E_DIR/wallet_config.json"
WALLET_E2E_STORAGE="$WALLET_E2E_DIR/storage.json"

timeout "$E2E_TIMEOUT" nix shell github:logos-co/logos-logoscore-cli --command bash -c "
  MODULES='$MODULES'
  WALLET_CONFIG='$WALLET_E2E_DIR/wallet_config.json'
  WALLET_STORAGE='$WALLET_E2E_STORAGE'
  WALLET_E2E_PASSWORD='$WALLET_E2E_PASSWORD'
  VC='$VC'
  VH='$VH'
  SC='$SC'
  logoscore stop 2>/dev/null || true
  sleep 2
  logoscore -D -m \"\$MODULES\" -q &
  DAEMON_PID=\$!
  sleep 3
  logoscore load-module logos_execution_zone >/dev/null
  logoscore load-module payment_streams_module >/dev/null
  if [[ ! -f \"\$WALLET_STORAGE\" ]]; then
    logoscore call logos_execution_zone create_new \"\$WALLET_CONFIG\" \"\$WALLET_STORAGE\" \"\$WALLET_E2E_PASSWORD\" 2>/dev/null | tail -1 | sed 's/^/WALLET:/'
  else
    WALLET_LINE=\$(logoscore call logos_execution_zone open \"\$WALLET_CONFIG\" \"\$WALLET_STORAGE\" 2>/dev/null | tail -1)
    echo \"\$WALLET_LINE\" | sed 's/^/WALLET:/'
    if ! python3 -c \"import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('result')==0 else 1)\" \"\$WALLET_LINE\" 2>/dev/null; then
      rm -f \"\$WALLET_STORAGE\"
      logoscore call logos_execution_zone create_new \"\$WALLET_CONFIG\" \"\$WALLET_STORAGE\" \"\$WALLET_E2E_PASSWORD\" 2>/dev/null | tail -1 | sed 's/^/WALLET:/'
    fi
  fi
  logoscore call payment_streams_module readVaultConfigDecoded \"\$VC\" 2>/dev/null | tail -1 | sed 's/^/VAULT:/'
  logoscore call payment_streams_module readVaultHoldingDecoded \"\$VH\" 2>/dev/null | tail -1 | sed 's/^/HOLD:/'
  logoscore call payment_streams_module readStreamConfigDecoded \"\$SC\" 2>/dev/null | tail -1 | sed 's/^/STREAM:/'
  logoscore call payment_streams_module readClock10Decoded 2>/dev/null | tail -1 | sed 's/^/CLOCK:/'
  logoscore stop 2>/dev/null || true
  wait \"\$DAEMON_PID\" 2>/dev/null || true
" >"$E2E_FILE" 2>&1 || echo E2E_TIMEOUT_OR_FAIL >>"$E2E_FILE"

check_decode_line() {
  local label="$1"
  local line
  line="$(rg "^${label}:" "$E2E_FILE" | tail -1 | sed "s/^${label}://")"
  if python3 -c "
import json,sys
outer=json.loads(sys.argv[1])
if outer.get('status')!='ok':
  sys.exit(1)
inner=json.loads(outer.get('result','{}'))
if inner.get('status')!='ok':
  sys.exit(2)
if 'decoded' not in inner:
  sys.exit(3)
" "$line" 2>/dev/null; then
    ok "logoscore $label decode"
  else
    bad "logoscore $label failed: $line"
  fi
}

if rg -q 'E2E_TIMEOUT_OR_FAIL' "$E2E_FILE" 2>/dev/null; then
  bad "logoscore E2E timed out (${E2E_TIMEOUT}s)"
  tail -20 "$E2E_FILE" >&2 || true
else
  OPEN_LINE="$(rg '^WALLET:' "$E2E_FILE" | tail -1 | sed 's/^WALLET://')"
  if python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('result')==0 else 1)" "$OPEN_LINE" 2>/dev/null; then
    ok "wallet ready before reads (create_new or open)"
  else
    bad "wallet not ready: $OPEN_LINE"
  fi
  check_decode_line VAULT
  check_decode_line HOLD
  check_decode_line STREAM
  check_decode_line CLOCK
fi

echo "=== done (exit $fail) ==="
exit "$fail"
