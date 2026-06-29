#!/usr/bin/env bash
# Step 24c — close the manifest stream on localnet after verify smokes (provider-signed close).
# Releases unaccrued allocation back to vault unallocated liquidity. Optional claim when accrued > 0.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
PROGRAM_BIN="${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
STATE_FILE="${REPO_ROOT}/.lez_payment_streams-state"
export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}"
MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO_ROOT/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO_ROOT/.scaffold/wallet/storage.json}"

if [[ "${SKIP_VERIFY_STREAM_TEARDOWN:-0}" == "1" ]]; then
  exit 0
fi

if [[ ! -f "$MANIFEST" ]]; then
  exit 0
fi

if ! python3 -c "import json; m=json.load(open('$MANIFEST')); exit(0 if m.get('stream_id') is not None else 1)" 2>/dev/null; then
  exit 0
fi

OWNER="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['owner_account_id'])")"
PROVIDER="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['provider_account_id'])")"
VAULT_ID="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['vault_id'])")"
STREAM_ID="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['stream_id'])")"

if [[ -f "$STATE_FILE" ]]; then
  # shellcheck disable=SC1090
  source "$STATE_FILE"
  OWNER="${SIGNER_ID:-$OWNER}"
fi

logoscore stop 2>/dev/null || true
sleep 1

"$REPO_ROOT/scripts/sync-seed-wallet-after-logoscore.sh" || true

# Step 24c: settle the chain so the close tx uses an authoritative committed nonce
# (a pending owner tx from the verify smoke would otherwise drop the close as a duplicate).
SEQUENCER_URL="${SEQUENCER_URL:-}" FIXTURE_MANIFEST="$MANIFEST" \
  "$REPO_ROOT/scripts/archive/wait-chain-settle.sh" "$OWNER" || true

echo "--- teardown: close stream $STREAM_ID (provider authority) ---"
if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "WARN: guest ELF missing; skip close ($PROGRAM_BIN)" >&2
else
  cargo run -q --manifest-path examples/Cargo.toml --bin seed_localnet_fixture -- \
    close-stream-onchain \
    --program-bin "$PROGRAM_BIN" \
    --owner "$OWNER" \
    --provider "$PROVIDER" \
    --vault-id "$VAULT_ID" \
    --stream-id "$STREAM_ID" || echo "WARN: close-stream-onchain failed (stream may already be closed)" >&2
fi

if [[ "${VERIFY_TEARDOWN_CLAIM:-0}" == "1" ]] && command -v logoscore >/dev/null 2>&1; then
  CLAIM_JSON="$(python3 -c "import json; print(json.dumps({'provider':'$PROVIDER','vault_id':int('$VAULT_ID'),'stream_id':int('$STREAM_ID')}))")"
  logoscore stop 2>/dev/null || true
  sleep 1
  nix shell github:logos-co/logos-logoscore-cli --command bash -c "
    set -uo pipefail
    export MODULES='$MODULES'
    logoscore -D -m \"\$MODULES\" --persistence-path '$REPO_ROOT/.scaffold/verify-teardown-persist' -q &
    sleep 3
    logoscore load-module logos_execution_zone >/dev/null 2>&1 || true
    logoscore load-module payment_streams_module >/dev/null 2>&1 || true
    logoscore call logos_execution_zone open '$WALLET_CONFIG' '$WALLET_STORAGE' >/dev/null 2>&1 || true
    logoscore call payment_streams_module chainAction claim '$CLAIM_JSON' 2>&1 | tail -1 || true
    logoscore stop 2>/dev/null || true
  " >/dev/null 2>&1 || true
fi

python3 -c "
import json
from pathlib import Path
p = Path('$MANIFEST')
m = json.loads(p.read_text())
for k in ('stream_id', 'stream_config_account_id'):
    m.pop(k, None)
p.write_text(json.dumps(m, indent=2) + '\n')
"
echo "teardown: manifest stream fields cleared (vault baseline)"
