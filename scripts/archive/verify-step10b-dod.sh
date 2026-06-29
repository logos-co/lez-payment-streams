#!/usr/bin/env bash
# Verify Step 10b definition of done (see docs/step10b-wallet-runtime.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export REPO="${REPO:-$REPO_ROOT}"
export MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
WALLET_FLAKE="$REPO/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched"
WALLET_LGX="${WALLET_LGX:-$(readlink -f "$WALLET_FLAKE/wallet-lgx-out"/*.lgx 2>/dev/null || true)}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
VERIFY_LOGOSCORE="${VERIFY_LOGOSCORE:-1}"

fail=0
ok() { echo "PASS: $*"; }
bad() { echo "FAIL: $*"; fail=1; }
skip() { echo "SKIP: $*"; }

echo "=== Step 10b DoD verification ==="

if [[ -f "$WALLET_LGX" ]]; then
  ok "patched wallet .lgx present ($WALLET_LGX)"
else
  bad "missing wallet .lgx (run ./scripts/build-wallet-lgx.sh)"
fi

if [[ -f docs/step10b-wallet-runtime.md ]]; then
  ok "Step 10b runbook present"
else
  bad "missing docs/step10b-wallet-runtime.md"
fi

if [[ ! -d "$MODULES/logos_execution_zone" ]]; then
  bad "logos_execution_zone not installed under MODULES=$MODULES"
else
  ok "logos_execution_zone install dir"
fi

if [[ -f "$MODULES/logos_execution_zone/manifest.json" ]]; then
  META_NAME="$(python3 -c "import json; print(json.load(open('$MODULES/logos_execution_zone/manifest.json'))['name'])")"
  if [[ "$META_NAME" == "logos_execution_zone" ]]; then
    ok "installed manifest name logos_execution_zone"
  else
    bad "manifest name is $META_NAME (reinstall from patched .lgx)"
  fi
else
  bad "missing $MODULES/logos_execution_zone/manifest.json (run lgpm install)"
fi

if [[ -f "$MODULES/logos_execution_zone/logos_execution_zone_plugin.so" ]]; then
  ok "logos_execution_zone_plugin.so present"
else
  bad "missing logos_execution_zone_plugin.so under MODULES"
fi

if [[ -f "$MODULES/payment_streams_module/payment_streams_module_plugin.so" ]]; then
  ok "payment_streams_module present (load-order pair)"
else
  skip "payment_streams_module not installed (optional for static checks)"
fi

if [[ ! -f "$WALLET_CONFIG" || ! -f "$WALLET_STORAGE" ]]; then
  bad "wallet config/storage missing ($WALLET_CONFIG)"
elif python3 -c "import json; c=json.load(open('$WALLET_CONFIG')); exit(0 if c.get('sequencer_addr')=='http://127.0.0.1:3040' else 1)"; then
  ok "wallet_config sequencer_addr matches Step 10a (:3040)"
else
  bad "wallet_config sequencer_addr must be http://127.0.0.1:3040 for Step 10a localnet"
fi

if [[ -f "$MANIFEST" ]]; then
  ok "fixture manifest for RPC checks ($MANIFEST)"
else
  skip "no $MANIFEST (logoscore RPC checks need Step 10a seed)"
fi

PLUGIN="$MODULES/logos_execution_zone/logos_execution_zone_plugin.so"
if nix shell github:logos-co/logos-module#lm --command bash -c "
  set -euo pipefail
  out=\$(lm methods '$PLUGIN')
  echo \"\$out\" | rg -q 'send_generic_public_transaction'
  echo \"\$out\" | rg -q 'get_account_public'
  echo \"\$out\" | rg -q '^int open\\('
"; then
  ok "lm methods lists PR 19 send_generic_public_transaction (+ open, get_account_public)"
else
  bad "lm methods missing PR 19 surface (reinstall patched .lgx)"
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
  -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' >/dev/null; then
  skip "logoscore E2E (sequencer not reachable; start Step 10a localnet first)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

FIXTURE_ACCOUNT="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['vault_config_account_id'])")"
E2E_FILE="$(mktemp)"
E2E_TIMEOUT="${LOGOSCORE_E2E_TIMEOUT:-120}"
trap 'rm -f "$E2E_FILE"' EXIT

timeout "$E2E_TIMEOUT" nix shell github:logos-co/logos-logoscore-cli --command bash -c "
  MODULES='$MODULES'
  WALLET_CONFIG='$WALLET_CONFIG'
  WALLET_STORAGE='$WALLET_STORAGE'
  FIXTURE_ACCOUNT='$FIXTURE_ACCOUNT'
  logoscore stop 2>/dev/null || true
  sleep 2
  logoscore -D -m \"\$MODULES\" -q &
  DAEMON_PID=\$!
  sleep 3
  LOAD_W=\$(logoscore load-module logos_execution_zone 2>&1 | tail -1)
  echo \"LOAD_W:\$LOAD_W\"
  LOAD_P=\$(logoscore load-module payment_streams_module 2>&1 | tail -1)
  echo \"LOAD_P:\$LOAD_P\"
  logoscore call logos_execution_zone open \"\$WALLET_CONFIG\" \"\$WALLET_STORAGE\" 2>/dev/null | tail -1 | sed 's/^/OPEN:/'
  HEX=\$(logoscore call logos_execution_zone account_id_from_base58 \"\$FIXTURE_ACCOUNT\" 2>/dev/null | tail -1 \
    | python3 -c \"import sys,json; print(json.load(sys.stdin).get('result',''))\" || true)
  echo \"HEX:\$HEX\"
  logoscore call logos_execution_zone get_account_public \"\$HEX\" 2>/dev/null | tail -1 | sed 's/^/ACCT:/'
  logoscore stop 2>/dev/null || true
  wait \"\$DAEMON_PID\" 2>/dev/null || true
" >"$E2E_FILE" 2>&1 || echo E2E_TIMEOUT_OR_FAIL >>"$E2E_FILE"

if rg -q 'E2E_TIMEOUT_OR_FAIL' "$E2E_FILE" 2>/dev/null; then
  bad "logoscore E2E timed out or failed (${E2E_TIMEOUT}s); stop stray daemons and retry"
  tail -15 "$E2E_FILE" >&2 || true
fi

LOAD_W_LINE="$(rg '^LOAD_W:' "$E2E_FILE" | tail -1 | sed 's/^LOAD_W://')"
LOAD_P_LINE="$(rg '^LOAD_P:' "$E2E_FILE" | tail -1 | sed 's/^LOAD_P://')"
if python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('status')=='ok' else 1)" "$LOAD_W_LINE" 2>/dev/null \
  && python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('status')=='ok' else 1)" "$LOAD_P_LINE" 2>/dev/null; then
  ok "logoscore load-module logos_execution_zone then payment_streams_module"
else
  bad "logoscore load-module failed (wallet=$LOAD_W_LINE ps=$LOAD_P_LINE)"
fi

OPEN_LINE="$(rg '^OPEN:' "$E2E_FILE" | tail -1 | sed 's/^OPEN://')"
if python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('status')=='ok' else 1)" "$OPEN_LINE" 2>/dev/null; then
  ok "logoscore open wallet (Step 10a .scaffold/wallet)"
else
  bad "logoscore open failed: $OPEN_LINE"
fi

HEX_LINE="$(rg '^HEX:' "$E2E_FILE" | tail -1 | sed 's/^HEX://')"
if [[ -n "$HEX_LINE" && ${#HEX_LINE} -ge 32 ]]; then
  ok "account_id_from_base58 for fixture vault_config PDA"
else
  bad "account_id_from_base58 failed (hex=$HEX_LINE)"
fi

ACCT_LINE="$(rg '^ACCT:' "$E2E_FILE" | tail -1 | sed 's/^ACCT://')"
if python3 -c "
import json,sys
d=json.loads(sys.argv[1])
if d.get('status')!='ok':
  sys.exit(1)
inner=json.loads(d.get('result','{}'))
if not inner.get('program_owner') or inner.get('data') in (None,''):
  sys.exit(2)
" "$ACCT_LINE" 2>/dev/null; then
  ok "get_account_public returns JSON with program_owner and data for fixture PDA"
else
  bad "get_account_public failed: $ACCT_LINE"
fi

echo "=== done (exit $fail) ==="
exit "$fail"
