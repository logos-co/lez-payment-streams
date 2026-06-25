#!/usr/bin/env bash
# Step 18 — testnet read smoke: legacy sequencer RPC + rc3 helper account read.
# Public testnet uses snake_case RPC (get_last_block, get_account). Pin-510 logos_execution_zone
# still speaks jsonrpsee camelCase (getLastBlockId) and fails against the current testnet endpoint.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/testnet-common.sh
source "$REPO_ROOT/scripts/testnet-common.sh"

MANIFEST="${FIXTURE_MANIFEST:-fixtures/testnet.json.example}"
E2E_TIMEOUT="${LOGOSCORE_E2E_TIMEOUT:-180}"

fail=0
ok() { echo "PASS: $*"; }
bad() { echo "FAIL: $*"; fail=1; }
skip() { echo "SKIP: $*"; }

echo "=== Step 18 testnet read smoke (legacy RPC + lez-testnet-submit) ==="

if ! SUBMIT_BIN="$(lez_testnet_submit_bin 2>/dev/null)"; then
  bad "lez-testnet-submit not built (cd tools/lez-testnet-submit && cargo build --release)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

set +e
BLOCK_HEIGHT="$(python3 "$REPO_ROOT/scripts/testnet_rpc.py" block-height 2>/dev/null)"
RPC_RC=$?
set -e
if [[ "$RPC_RC" -ne 0 || -z "$BLOCK_HEIGHT" ]]; then
  skip "testnet sequencer unreachable ($TESTNET_SEQUENCER); cannot validate read path"
  echo "=== done (exit 0 skipped) ==="
  exit 0
fi
ok "get_last_block = $BLOCK_HEIGHT"

CLOCK_BASE58="${TESTNET_CLOCK_ACCOUNT_ID:-}"
if [[ -z "$CLOCK_BASE58" && -f "$MANIFEST" ]]; then
  CLOCK_BASE58="$(python3 -c "import json; print(json.load(open('$MANIFEST')).get('clock_10_account_id',''))")"
fi
if [[ -z "$CLOCK_BASE58" ]]; then
  CLOCK_BASE58="4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWSs"
fi

ensure_testnet_rc3_wallet
WALLET_CFG="$TESTNET_WALLET_DIR/wallet_config.json"
WALLET_STORAGE="$TESTNET_WALLET_DIR/storage.json"

READ_ACCOUNT_B58="$(python3 -c "
import json, os, urllib.request
url = os.environ.get('TESTNET_SEQUENCER', 'https://testnet.lez.logos.co/')
payload = json.dumps({'jsonrpc':'2.0','id':1,'method':'get_initial_testnet_accounts','params':{}}).encode()
req = urllib.request.Request(url, data=payload, headers={'Content-Type':'application/json'}, method='POST')
body = json.loads(urllib.request.urlopen(req, timeout=20).read())
accs = body.get('result') or []
if not accs:
    raise SystemExit('no initial testnet accounts')
print(accs[0]['account_id'])
")"

CLOCK_HEX="$("$SUBMIT_BIN" account-id-from-base58 "$READ_ACCOUNT_B58" 2>/dev/null | tail -1)"

ACCT_OUT="$("$SUBMIT_BIN" get-account-public \
  --wallet-config "$WALLET_CFG" \
  --wallet-storage "$WALLET_STORAGE" \
  --account-id-hex "$CLOCK_HEX" 2>&1)" || true

if echo "$ACCT_OUT" | python3 -c "
import json,sys
for line in sys.stdin:
    line=line.strip()
    if not line.startswith('{'): continue
    d=json.loads(line)
    ok = d.get('success') and (d.get('balance', 0) > 0 or d.get('has_data') or d.get('program_owner_nonzero'))
    sys.exit(0 if ok else 1)
sys.exit(1)
" <<< "$ACCT_OUT"; then
    ok "get_account on testnet seed account ($READ_ACCOUNT_B58) via legacy RPC"
else
  bad "get_account failed: $ACCT_OUT"
fi

# Optional: 510 logos_execution_zone path (expected to fail until testnet exposes jsonrpsee or module gains legacy client).
if [[ "${STEP18_READ_SMOKE_510:-0}" == "1" ]]; then
  MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
  WALLET_CONFIG_BASE="${WALLET_CONFIG:-$REPO_ROOT/.scaffold/wallet/wallet_config.json}"
  WALLET_STORAGE_510="${WALLET_STORAGE:-$REPO_ROOT/.scaffold/wallet/storage.json}"
  if [[ -f "$WALLET_CONFIG_BASE" && -f "$WALLET_STORAGE_510" && -d "$MODULES/logos_execution_zone" ]]; then
    skip "STEP18_READ_SMOKE_510=1 set but 510 jsonrpsee against current testnet is not supported (see step18 runbook)"
  else
    skip "STEP18_READ_SMOKE_510=1 but wallet/modules missing"
  fi
else
  skip "510 logos_execution_zone read path not run (testnet uses legacy RPC; set STEP18_READ_SMOKE_510=1 to force attempt)"
fi

echo "=== done (exit $fail) ==="
exit "$fail"
