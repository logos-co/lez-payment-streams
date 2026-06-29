#!/usr/bin/env bash
# Step 18 — read smoke: rc5 logos_execution_zone against public testnet RPC.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
WALLET_CONFIG_BASE="${WALLET_CONFIG:-$REPO_ROOT/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO_ROOT/.scaffold/wallet/storage.json}"
TESTNET_SEQUENCER="${TESTNET_SEQUENCER:-https://testnet.lez.logos.co/}"
MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
E2E_TIMEOUT="${LOGOSCORE_E2E_TIMEOUT:-180}"

fail=0
ok() { echo "PASS: $*"; }
bad() { echo "FAIL: $*"; fail=1; }
skip() { echo "SKIP: $*"; }

echo "=== Step 18 testnet read smoke (rc5 wallet module → public sequencer) ==="

if [[ ! -f "$WALLET_CONFIG_BASE" || ! -f "$WALLET_STORAGE" ]]; then
  bad "wallet config/storage missing (WALLET_CONFIG=$WALLET_CONFIG_BASE)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -d "$MODULES/logos_execution_zone" ]]; then
  bad "logos_execution_zone not under MODULES=$MODULES (lgpm install)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

BLOCK_JSON="$(curl -sf -m 20 -X POST "$TESTNET_SEQUENCER" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' 2>/dev/null || true)"
if [[ -z "$BLOCK_JSON" ]] || ! python3 -c "import json,sys; json.loads(sys.argv[1])" "$BLOCK_JSON" 2>/dev/null; then
  skip "testnet sequencer unreachable or non-JSON ($TESTNET_SEQUENCER); cannot validate read path"
  echo "=== done (exit 0 skipped) ==="
  exit 0
fi

BLOCK_HEIGHT="$(python3 -c "
import json, sys
d = json.loads(sys.argv[1])
r = d.get('result')
if isinstance(r, int):
    print(r)
elif isinstance(r, str) and r.isdigit():
    print(int(r))
else:
    sys.exit(1)
" "$BLOCK_JSON")" || {
  bad "getLastBlockId unexpected shape: $BLOCK_JSON"
  echo "=== done (exit $fail) ==="
  exit "$fail"
}
ok "getLastBlockId = $BLOCK_HEIGHT"

CLOCK_BASE58="${TESTNET_CLOCK_ACCOUNT_ID:-}"
if [[ -z "$CLOCK_BASE58" && -f "$MANIFEST" ]]; then
  CLOCK_BASE58="$(python3 -c "import json; print(json.load(open('$MANIFEST')).get('clock_10_account_id',''))")"
fi
if [[ -z "$CLOCK_BASE58" ]]; then
  CLOCK_BASE58="4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWNU"
fi

WALLET_CONFIG_TESTNET="$(mktemp)"
trap 'rm -f "$WALLET_CONFIG_TESTNET" "$E2E_FILE"' EXIT
python3 -c "
import json, sys
src, dst, url = sys.argv[1], sys.argv[2], sys.argv[3]
c = json.load(open(src))
c['sequencer_addr'] = url
json.dump(c, open(dst, 'w'), indent=2)
" "$WALLET_CONFIG_BASE" "$WALLET_CONFIG_TESTNET" "$TESTNET_SEQUENCER"

E2E_FILE="$(mktemp)"

timeout "$E2E_TIMEOUT" nix shell github:logos-co/logos-logoscore-cli --command bash -c "
  MODULES='$MODULES'
  WALLET_CONFIG='$WALLET_CONFIG_TESTNET'
  WALLET_STORAGE='$WALLET_STORAGE'
  CLOCK='$CLOCK_BASE58'
  HEIGHT='$BLOCK_HEIGHT'
  logoscore stop 2>/dev/null || true
  sleep 2
  logoscore -D -m \"\$MODULES\" -q &
  DAEMON_PID=\$!
  sleep 3
  logoscore load-module logos_execution_zone 2>&1 | tail -1 | sed 's/^/LOAD_W:/'
  logoscore call logos_execution_zone open \"\$WALLET_CONFIG\" \"\$WALLET_STORAGE\" 2>/dev/null | tail -1 | sed 's/^/OPEN:/'
  logoscore call logos_execution_zone sync_to_block \"\$HEIGHT\" 2>/dev/null | tail -1 | sed 's/^/SYNC:/'
  HEX=\$(logoscore call logos_execution_zone account_id_from_base58 \"\$CLOCK\" 2>/dev/null | tail -1 \
    | python3 -c \"import sys,json; print(json.load(sys.stdin).get('result',''))\" || true)
  echo \"HEX:\$HEX\"
  logoscore call logos_execution_zone get_account_public \"\$HEX\" 2>/dev/null | tail -1 | sed 's/^/ACCT:/'
  logoscore stop 2>/dev/null || true
  wait \"\$DAEMON_PID\" 2>/dev/null || true
" >"$E2E_FILE" 2>&1 || echo E2E_TIMEOUT_OR_FAIL >>"$E2E_FILE"

if rg -q 'E2E_TIMEOUT_OR_FAIL' "$E2E_FILE" 2>/dev/null; then
  bad "logoscore smoke timed out or failed (${E2E_TIMEOUT}s)"
  tail -20 "$E2E_FILE" >&2 || true
fi

LOAD_W_LINE="$(rg '^LOAD_W:' "$E2E_FILE" | tail -1 | sed 's/^LOAD_W://')"
if python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('status')=='ok' else 1)" "$LOAD_W_LINE" 2>/dev/null; then
  ok "load-module logos_execution_zone"
else
  bad "load-module failed: $LOAD_W_LINE"
fi

OPEN_LINE="$(rg '^OPEN:' "$E2E_FILE" | tail -1 | sed 's/^OPEN://')"
if python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('status')=='ok' else 1)" "$OPEN_LINE" 2>/dev/null; then
  ok "open wallet with testnet sequencer_addr (510 .lgx)"
else
  bad "open failed: $OPEN_LINE"
fi

SYNC_LINE="$(rg '^SYNC:' "$E2E_FILE" | tail -1 | sed 's/^SYNC://')"
if python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('status')=='ok' else 1)" "$SYNC_LINE" 2>/dev/null; then
  ok "sync_to_block $BLOCK_HEIGHT"
else
  bad "sync_to_block failed: $SYNC_LINE"
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
  ok "get_account_public on CLOCK_10 ($CLOCK_BASE58) returned on-chain data"
else
  bad "get_account_public failed (wrong clock id on testnet or RPC error): $ACCT_LINE"
fi

echo "=== done (exit $fail) ==="
exit "$fail"
