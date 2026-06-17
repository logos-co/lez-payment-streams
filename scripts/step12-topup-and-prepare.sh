#!/usr/bin/env bash
# Top up stream 0 (when chain writes work), then verify prepareEligibilityForStoreQuery.
# Depleted stream: run ./scripts/demo-localnet-fresh.sh (see docs/demo-localnet-recovery.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export REPO="${REPO:-$REPO_ROOT}"
export MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
export WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
export WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
export PAYMENT_STREAMS_GUEST_BIN="${PAYMENT_STREAMS_GUEST_BIN:-$REPO/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
export MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
export PERSIST_DIR="${PERSIST_DIR:-$REPO/.scaffold/step12-persist-topup}"
PROVIDER_PEER_ID="${PROVIDER_PEER_ID:-step12-demo-provider-peer}"
DEMO_BYPASS="${PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF:-0}"

OWNER="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['owner_account_id'])")"
PROVIDER_B58="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['provider_account_id'])")"
VAULT_ID="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['vault_id'])")"
STREAM_ID="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['stream_id'])")"
N8_WIRE_HEX="$(cargo run -q -p lez-payment-streams-core --bin n8_canonical_wire_hex 2>/dev/null)"
TOPUP_JSON="$(python3 -c "import json; print(json.dumps({'signer':'$OWNER','vault_id':int('$VAULT_ID'),'stream_id':int('$STREAM_ID'),'increase_lo':10,'increase_hi':0}))")"

rm -rf "$PERSIST_DIR"
mkdir -p "$PERSIST_DIR"

nix shell github:logos-co/logos-logoscore-cli --command bash -c "
  export MODULES='$MODULES'
  export WALLET_CONFIG='$WALLET_CONFIG'
  export WALLET_STORAGE='$WALLET_STORAGE'
  export PAYMENT_STREAMS_GUEST_BIN='$PAYMENT_STREAMS_GUEST_BIN'
  export PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF='$DEMO_BYPASS'
  export PERSIST_DIR='$PERSIST_DIR'
  logoscore stop 2>/dev/null || true
  sleep 2
  logoscore -D -m \"\$MODULES\" --persistence-path \"\$PERSIST_DIR\" -q &
  sleep 4
  logoscore load-module logos_execution_zone >/dev/null
  logoscore load-module payment_streams_module >/dev/null
  logoscore call logos_execution_zone open \"\$WALLET_CONFIG\" \"\$WALLET_STORAGE\" >/dev/null

  logoscore call payment_streams_module registerProviderMapping '$PROVIDER_PEER_ID' '$PROVIDER_B58' >/dev/null

  if [[ \"\${TRY_TOPUP:-1}\" == \"1\" ]]; then
    echo '--- topUpStream (optional) ---'
    logoscore call payment_streams_module chainAction topUpStream '$TOPUP_JSON' 2>&1 | tail -1 || true
    sleep 2
  fi

  echo '--- prepareEligibilityForStoreQuery ---'
  PREP=\$(logoscore call payment_streams_module prepareEligibilityForStoreQuery '$N8_WIRE_HEX' '$PROVIDER_PEER_ID' 2>&1 | tail -1)
  echo \"\$PREP\"
  echo \"\$PREP\" | python3 -c \"
import json,sys
line=sys.stdin.read().strip()
outer=json.loads(line)
inner=json.loads(outer.get('result','{}'))
if inner.get('status')!='ok':
  print('VERIFY_FAIL', inner.get('code'), inner.get('message'))
  if inner.get('code')=='STREAM_DEPLETED':
    print('HINT: ./scripts/demo-localnet-fresh.sh')
  sys.exit(1)
if inner.get('kind')!='stream_proof':
  print('VERIFY_FAIL expected stream_proof got', inner.get('kind'))
  sys.exit(1)
if not inner.get('bytes_hex'):
  print('VERIFY_FAIL missing bytes_hex')
  sys.exit(1)
print('VERIFY_OK stream_proof bytes_hex_len', len(inner['bytes_hex']))
\"

  logoscore stop 2>/dev/null || true
"
