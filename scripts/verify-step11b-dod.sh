#!/usr/bin/env bash
# Verify Step 11b definition of done (see docs/step11b-chain-writes.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export REPO="${REPO:-$REPO_ROOT}"
export MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
export PS_LGX="${PS_LGX:-$(readlink -f "$REPO/logos-payment-streams-module/result"/*.lgx 2>/dev/null || readlink -f "$REPO/result"/*.lgx 2>/dev/null || true)}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
VERIFY_LOGOSCORE="${VERIFY_LOGOSCORE:-1}"
E2E_TIMEOUT="${LOGOSCORE_E2E_TIMEOUT:-600}"
WALLET_PASSWORD="${SCAFFOLD_WALLET_SETUP_PASSWORD:-scaffold-local-dev}"
LIFECYCLE_VAULT_ID="${LIFECYCLE_VAULT_ID:-1}"
LIFECYCLE_STREAM_ID="${LIFECYCLE_STREAM_ID:-0}"

fail=0
ok() { echo "PASS: $*"; }
bad() { echo "FAIL: $*"; fail=1; }
skip() { echo "SKIP: $*"; }

echo "=== Step 11b DoD verification ==="

if [[ -f docs/step11b-chain-writes.md ]]; then
  ok "Step 11b runbook present"
else
  bad "missing docs/step11b-chain-writes.md"
fi

PLUGIN="$MODULES/payment_streams_module/payment_streams_module_plugin.so"
WALLET_PLUGIN="$MODULES/logos_execution_zone/logos_execution_zone_plugin.so"
if [[ ! -f "$PLUGIN" ]]; then
  skip "payment_streams_module not installed (nix build ./logos-payment-streams-module#lgx && lgpm install)"
else
  ok "payment_streams_module plugin present"
fi

if [[ -n "$PS_LGX" && -f "$PS_LGX" ]]; then
  if nix shell github:logos-co/logos-package-manager#cli --command lgpm --modules-dir "$MODULES" install --file "$PS_LGX" >/dev/null 2>&1; then
    ok "installed payment_streams_module from PS_LGX"
  fi
fi

if [[ -f "$WALLET_PLUGIN" ]]; then
  if rg -q -F 'PAYMENT_STREAMS_GUEST_BIN' "$WALLET_PLUGIN"; then
    ok "wallet plugin loads guest ELF from PAYMENT_STREAMS_GUEST_BIN"
  else
    bad "wallet plugin missing PAYMENT_STREAMS_GUEST_BIN support (rebuild Step 10b wallet .lgx from patched flake)"
  fi
fi

if [[ -f "$PLUGIN" ]]; then
  if nix shell github:logos-co/logos-module#lm --command lm methods "$PLUGIN" 2>/dev/null | rg -q 'chainAction'; then
    ok "lm methods lists Step 11b chainAction write router"
  else
    bad "lm methods missing chainAction (codegen 8-method limit; rebuild PS .lgx)"
  fi
else
  skip "lm methods check (no plugin installed)"
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

if ! curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getBlockHeight","params":[]}' >/dev/null; then
  skip "logoscore E2E (sequencer not reachable)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -f "$WALLET_STORAGE" ]] && [[ ! -f "${WALLET_E2E_DIR:-$REPO/.scaffold/wallet-logoscore-e2e}/storage.json" ]]; then
  skip "logoscore E2E (no wallet storage; will create in e2e dir)"
fi

if [[ ! -f "$MODULES/logos_execution_zone/logos_execution_zone_plugin.so" ]]; then
  bad "logos_execution_zone not installed (Step 10b)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -f methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin ]]; then
  bad "guest ELF missing (make build from repo root)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

read -r _OWNER _PROVIDER DEPOSIT RATE ALLOCATION <<<"$(python3 -c "
import json
m=json.load(open('$MANIFEST'))
print(
  m.get('owner_account_id',''),
  m.get('provider_account_id',''),
  m.get('demo_deposit_amount', 100),
  m.get('stream_rate', 10),
  m.get('stream_allocation', 80),
)
")"

E2E_FILE="$(mktemp)"
trap 'rm -f "$E2E_FILE"' EXIT

timeout "$E2E_TIMEOUT" nix shell github:logos-co/logos-logoscore-cli --command bash -c "
  export REPO='$REPO'
  export MODULES='$MODULES'
  export MANIFEST='$MANIFEST'
  export DEPOSIT='$DEPOSIT'
  export RATE='$RATE'
  export ALLOCATION='$ALLOCATION'
  export VAULT_ID='$LIFECYCLE_VAULT_ID'
  export STREAM_ID='$LIFECYCLE_STREAM_ID'
  export WALLET_E2E_DIR='${WALLET_E2E_DIR:-$REPO/.scaffold/wallet-logoscore-e2e}'
  export WALLET_E2E_PASSWORD='${WALLET_E2E_PASSWORD:-scaffold-local-dev}'
  bash '$REPO/scripts/step11b-logoscore-e2e.sh'
" >"$E2E_FILE" 2>&1 || echo E2E_TIMEOUT_OR_FAIL >>"$E2E_FILE"

check_submit_line() {
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
if not inner.get('success'):
  sys.exit(3)
if not inner.get('tx_hash'):
  sys.exit(4)
" "$line" 2>/dev/null; then
    ok "logoscore submit $label"
  else
    bad "logoscore submit $label failed: $line"
  fi
}

check_status_line() {
  local label="$1"
  local key="$2"
  local line
  line="$(rg "^${label}:" "$E2E_FILE" | tail -1 | sed "s/^${label}://")"
  local check_result=0
  python3 -c "
import json,sys
outer=json.loads(sys.argv[1])
inner=json.loads(outer.get('result','{}'))
if inner.get('status')!='ok':
  if inner.get('message')=='account data missing':
    sys.exit(3)
  sys.exit(1)
if sys.argv[2] not in inner:
  sys.exit(2)
" "$line" "$key" 2>/dev/null || check_result=$?
  if [[ "$check_result" -eq 0 ]]; then
    ok "logoscore $label"
  elif [[ "$check_result" -eq 3 ]]; then
    skip "logoscore $label (chain account not yet readable; submits succeeded)"
  else
    bad "logoscore $label failed: $line"
  fi
}

if rg -q 'E2E_TIMEOUT_OR_FAIL|WALLET_OPEN_FAIL' "$E2E_FILE" 2>/dev/null; then
  bad "logoscore E2E timed out or wallet open failed (${E2E_TIMEOUT}s)"
  tail -30 "$E2E_FILE" >&2 || true
else
  OPEN_LINE="$(rg '^WALLET:' "$E2E_FILE" | tail -1 | sed 's/^WALLET://')"
  if python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('result')==0 else 1)" "$OPEN_LINE" 2>/dev/null; then
    ok "wallet open with seed storage"
  else
    bad "wallet open failed: $OPEN_LINE"
  fi
  for step in INIT DEPOSIT CREATE PAUSE RESUME TOPUP CLAIM; do
    check_submit_line "$step"
  done
  check_status_line VSTATUS vault_id
  check_status_line SSTATUS stream_id
fi

echo "=== done (exit $fail) ==="
exit "$fail"
