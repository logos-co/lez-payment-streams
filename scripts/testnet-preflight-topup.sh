#!/usr/bin/env bash
# Step 18 — restore testnet stream unaccrued before verify (no depleted-stream bypass).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/testnet-common.sh
source "$REPO_ROOT/scripts/testnet-common.sh"

require_testnet_rpc
ensure_testnet_wallet

export CHAIN=testnet
export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet.json}"
export WALLET_CONFIG="${WALLET_CONFIG:-$TESTNET_WALLET_DIR/wallet_config.json}"
export WALLET_STORAGE="${WALLET_STORAGE:-$TESTNET_WALLET_DIR/storage.json}"
export LEZ_TESTNET_WALLET_CONFIG="${LEZ_TESTNET_WALLET_CONFIG:-$WALLET_CONFIG}"
export LEZ_TESTNET_WALLET_STORAGE="${LEZ_TESTNET_WALLET_STORAGE:-$WALLET_STORAGE}"
export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
export LEE_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
export PAYMENT_STREAMS_GUEST_BIN="${PAYMENT_STREAMS_GUEST_BIN:-$PROGRAM_BIN}"
export PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF=0
export TESTNET_AUTH_TRANSFER_ELF_PATH="${TESTNET_AUTH_TRANSFER_ELF_PATH:-$(testnet_auth_transfer_elf_path)}"

SUBMIT_BIN="$(lez_testnet_submit_bin)"
export LEZ_TESTNET_SUBMIT="$SUBMIT_BIN"
export PATH="$(dirname "$SUBMIT_BIN"):$PATH"

if [[ ! -f "$FIXTURE_MANIFEST" ]]; then
  echo "testnet-preflight-topup: missing $FIXTURE_MANIFEST" >&2
  exit 2
fi

MODULES="${MODULES:-$REPO_ROOT/.scaffold/e2e/user/modules}"
if [[ ! -d "$MODULES/payment_streams_module" ]]; then
  echo "testnet-preflight-topup: install modules first (make verify-step18 with SKIP_BUILD=0 once)" >&2
  exit 2
fi

OWNER="$(python3 -c "import json; print(json.load(open('$FIXTURE_MANIFEST'))['owner_account_id'])")"
VAULT_ID="$(python3 -c "import json; print(json.load(open('$FIXTURE_MANIFEST'))['vault_id'])")"
STREAM_ID="$(python3 -c "import json; print(json.load(open('$FIXTURE_MANIFEST'))['stream_id'])")"
INCREASE_LO="${TESTNET_PREFLIGHT_TOPUP_LO:-250}"
SEQ_URL="$(python3 -c "import json; print(json.load(open('$FIXTURE_MANIFEST')).get('sequencer_url','https://testnet.lez.logos.co/'))")"
TOPUP_JSON="$(python3 -c "import json; print(json.dumps({'signer':'$OWNER','vault_id':int('$VAULT_ID'),'stream_id':int('$STREAM_ID'),'increase_lo':int('$INCREASE_LO'),'increase_hi':0}))")"

PERSIST="${PERSIST_DIR:-$REPO_ROOT/.scaffold/e2e/testnet-preflight-persist}"
rm -rf "$PERSIST"
mkdir -p "$PERSIST"

echo "=== testnet-preflight-topup vault=$VAULT_ID stream=$STREAM_ID increase_lo=$INCREASE_LO ==="

BLOCK="$(testnet_rpc_last_block)"
TIMEOUT="${LOGOSCORE_CHAIN_ACTION_TIMEOUT:-360}"

nix shell github:logos-co/logos-logoscore-cli --command bash -c "
  set -euo pipefail
  export CHAIN=testnet
  export FIXTURE_MANIFEST='$FIXTURE_MANIFEST'
  export WALLET_CONFIG='$WALLET_CONFIG'
  export WALLET_STORAGE='$WALLET_STORAGE'
  export LEZ_TESTNET_WALLET_CONFIG='$LEZ_TESTNET_WALLET_CONFIG'
  export LEZ_TESTNET_WALLET_STORAGE='$LEZ_TESTNET_WALLET_STORAGE'
  export PAYMENT_STREAMS_GUEST_BIN='$PAYMENT_STREAMS_GUEST_BIN'
  export LEZ_TESTNET_SUBMIT='$LEZ_TESTNET_SUBMIT'
  export PATH='$(dirname "$SUBMIT_BIN")':\"\$PATH\"
  export TESTNET_AUTH_TRANSFER_ELF_PATH='$TESTNET_AUTH_TRANSFER_ELF_PATH'
  logoscore stop 2>/dev/null || true
  sleep 2
  logoscore -D -m '$MODULES' --persistence-path '$PERSIST' -q &
  sleep 5
  logoscore load-module logos_execution_zone >/dev/null
  logoscore load-module payment_streams_module >/dev/null
  logoscore call logos_execution_zone open '$WALLET_CONFIG' '$WALLET_STORAGE' >/dev/null
  logoscore call logos_execution_zone sync_to_block '$BLOCK' >/dev/null
  sleep 2
  echo '--- chainAction topUpStream ---'
  OUT=\$(timeout '$TIMEOUT' logoscore call payment_streams_module chainAction topUpStream '$TOPUP_JSON' 2>&1 | tail -1)
  echo \"\$OUT\"
  echo \"\$OUT\" | python3 -c \"
import json,sys
line=sys.stdin.read().strip()
if not line.startswith('{'):
  print('TOPUP_FAIL: no JSON', line[:200])
  sys.exit(1)
outer=json.loads(line)
if outer.get('status')!='ok':
  print('TOPUP_FAIL outer', outer)
  sys.exit(1)
inner=outer.get('result')
if isinstance(inner,str):
  inner=json.loads(inner)
if isinstance(inner,dict) and inner.get('success') is False:
  print('TOPUP_FAIL inner', inner)
  sys.exit(1)
print('TOPUP_OK')
\"
  logoscore call logos_execution_zone sync_to_block '$BLOCK' >/dev/null 2>&1 || true
  logoscore call payment_streams_module rediscoverStreams '$VAULT_ID' >/dev/null 2>&1 || true
  logoscore stop 2>/dev/null || true
"

echo "=== testnet-preflight-topup done (sequencer $SEQ_URL block $BLOCK) ==="
