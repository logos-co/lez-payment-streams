#!/usr/bin/env bash
# Create per-run stream (if needed), then verify prepareEligibilityProofWithStreamProofForStoreQuery → stream_proof.
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

python3 -c "
import json
from pathlib import Path
p = Path('''$MANIFEST''')
m = json.loads(p.read_text())
for k in ('stream_id', 'stream_config_account_id'):
    m.pop(k, None)
p.write_text(json.dumps(m, indent=2) + '\n')
"

echo "--- create stream for proof path (Step 24c, chain next_stream_id) ---"
"$REPO/scripts/sync-seed-wallet-after-logoscore.sh" || true
logoscore stop 2>/dev/null || true
sleep 2
if [[ -f "$REPO/scripts/e2e/continuation-owner-topup.sh" ]]; then
  E2E_CONTINUATION_PINATA_ROUNDS="${VERIFY_CREATE_PINATA_ROUNDS:-6}" \
    "$REPO/scripts/e2e/continuation-owner-topup.sh" >/dev/null 2>&1 || true
fi
if [[ -f "$WALLET_CONFIG" ]]; then
  python3 -c "
import json
from pathlib import Path
p = Path('$WALLET_CONFIG')
d = json.loads(p.read_text())
d['seq_poll_timeout'] = '${VERIFY_WALLET_POLL_TIMEOUT:-8s}'
d['seq_tx_poll_max_blocks'] = int('${VERIFY_WALLET_POLL_MAX_BLOCKS:-22}')
p.write_text(json.dumps(d, indent=4) + '\n')
"
fi
export CREATE_FORCE=1
export E2E_PER_RUN_STREAM=1
"$REPO/scripts/archive/create-localnet-stream-fixture.sh"

teardown_verify_stream() {
  FIXTURE_MANIFEST="$MANIFEST" "$REPO/scripts/archive/demo-stream-teardown-localnet.sh" || true
}
trap teardown_verify_stream EXIT

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
  height=\$(curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
    -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getLastBlockId\",\"params\":[]}' \
    | python3 -c 'import json,sys; d=json.load(sys.stdin); r=d.get(\"result\"); print(r if isinstance(r,int) else (r or \"\"))' 2>/dev/null || true)
  if [[ -n \"\$height\" ]]; then
    logoscore call logos_execution_zone sync_to_block \"\$height\" >/dev/null 2>&1 || true
    sleep 2
  fi

  logoscore call payment_streams_module registerProviderMapping '$PROVIDER_PEER_ID' '$PROVIDER_B58' >/dev/null
  logoscore call payment_streams_module rediscoverStreams '$VAULT_ID' >/dev/null

  if [[ \"\${TRY_TOPUP:-1}\" == \"1\" ]]; then
    echo '--- topUpStream (optional) ---'
    logoscore call payment_streams_module chainAction topUpStream '$TOPUP_JSON' 2>&1 | tail -1 || true
    sleep 2
  fi

  echo '--- prepareEligibilityProofWithStreamProofForStoreQuery ---'
  PREP=\$(logoscore call payment_streams_module prepareEligibilityProofWithStreamProofForStoreQuery '$N8_WIRE_HEX' '$PROVIDER_PEER_ID' '$STREAM_ID' 2>&1 | tail -1)
  echo \"\$PREP\"
  echo \"\$PREP\" | python3 -c \"
import json,sys
line=sys.stdin.read().strip()
outer=json.loads(line)
inner=json.loads(outer.get('result','{}'))
if inner.get('status')!='ok':
  print('VERIFY_FAIL', inner.get('code'), inner.get('message'))
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
