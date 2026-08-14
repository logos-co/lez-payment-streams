#!/usr/bin/env bash
# Logoscore lifecycle for Step 11b (env: REPO, MODULES, MANIFEST, WALLET_E2E_*).
set -euo pipefail

cd "${REPO:?}"

export FIXTURE_MANIFEST="${MANIFEST:?}"
export REPO="${REPO:?}"
export PAYMENT_STREAMS_GUEST_BIN="${REPO}/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
WALLET_E2E_DIR="${WALLET_E2E_DIR:-$REPO/.scaffold/wallet-logoscore-e2e}"
WALLET_E2E_PASSWORD="${WALLET_E2E_PASSWORD:-scaffold-local-dev}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$WALLET_E2E_DIR/storage.json}"
VAULT_ID="${VAULT_ID:-1}"
STREAM_ID="${STREAM_ID:-0}"
DEPOSIT="${DEPOSIT:-100}"
RATE="${RATE:-10}"
ALLOCATION="${ALLOCATION:-80}"

SEED_STORAGE="${WALLET_SEED_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
mkdir -p "$WALLET_E2E_DIR"
rm -f "$WALLET_E2E_DIR/storage.json"
if [[ -f "$SEED_STORAGE" ]]; then
  cp "$SEED_STORAGE" "$WALLET_E2E_DIR/storage.json"
fi
cp "$WALLET_CONFIG" "$WALLET_E2E_DIR/wallet_config.json"
WALLET_CONFIG="$WALLET_E2E_DIR/wallet_config.json"
WALLET_STORAGE="$WALLET_E2E_DIR/storage.json"

sync_wallet() {
  local height
  height=$(curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' \
    | python3 -c 'import json,sys; d=json.load(sys.stdin); r=d.get("result"); print(r if isinstance(r,int) else (r or ""))' 2>/dev/null || true)
  if [[ -n "$height" ]]; then
    logoscore call logos_execution_zone sync_to_block "$height" 2>/dev/null >/dev/null || true
  fi
  sleep 3
}

call_ps() {
  local label=$1
  local op=$2
  local params_json=$3
  logoscore call payment_streams_module chainAction "$op" "$params_json" 2>/dev/null | tail -1 | sed "s/^/${label}:/" || true
}

call_ps_status_retry() {
  local label=$1
  local op=$2
  local params_json=$3
  local key=$4
  local attempt line
  for attempt in 1 2 3 4 5 6; do
    line=$(logoscore call payment_streams_module chainAction "$op" "$params_json" 2>/dev/null | tail -1 | sed "s/^/${label}:/")
    if python3 -c "
import json,sys
outer=json.loads(sys.argv[1].split(':',1)[1])
inner=json.loads(outer.get('result','{}'))
sys.exit(0 if inner.get('status')=='ok' and sys.argv[2] in inner else 1)
" "$line" "$key" 2>/dev/null; then
      echo "$line"
      return 0
    fi
    sleep 10
  done
  echo "$line"
}

parse_inner_result_json() {
  python3 -c 'import json,sys; outer=json.loads(sys.argv[1]); print(outer.get("result",""))' "$1"
}

logoscore stop 2>/dev/null || true
sleep 2
logoscore -D -m "$MODULES" -q &
DAEMON_PID=$!
sleep 3
logoscore load-module logos_execution_zone >/dev/null
logoscore load-module payment_streams_module >/dev/null

if [[ ! -f "$WALLET_STORAGE" ]]; then
  OPEN_LINE=$(logoscore call logos_execution_zone create_new "$WALLET_CONFIG" "$WALLET_STORAGE" "$WALLET_E2E_PASSWORD" 2>/dev/null | tail -1)
else
  OPEN_LINE=$(logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE" 2>/dev/null | tail -1)
  if ! python3 -c 'import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get("result")==0 else 1)' "$OPEN_LINE" 2>/dev/null; then
    rm -f "$WALLET_STORAGE"
    OPEN_LINE=$(logoscore call logos_execution_zone create_new "$WALLET_CONFIG" "$WALLET_STORAGE" "$WALLET_E2E_PASSWORD" 2>/dev/null | tail -1)
  fi
fi
echo "WALLET:$OPEN_LINE"
if ! python3 -c 'import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get("result")==0 else 1)' "$OPEN_LINE" 2>/dev/null; then
  echo WALLET_OPEN_FAIL
  logoscore stop 2>/dev/null || true
  exit 1
fi
logoscore call logos_execution_zone save 2>/dev/null >/dev/null || true

to_base58() {
  local hex_id=$1
  local line b58
  line=$(logoscore call logos_execution_zone account_id_to_base58 "$hex_id" 2>/dev/null | tail -1)
  b58=$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$line" 2>/dev/null || true)
  if [[ -n "$b58" ]]; then
    echo "$b58"
  else
    echo "$hex_id"
  fi
}

read -r MANIFEST_OWNER MANIFEST_PROVIDER <<<"$(python3 -c "
import json
m=json.load(open('${MANIFEST:?}'))
print(m.get('owner_account_id',''), m.get('provider_account_id',''))
")"

ACCOUNTS_LINE=$(logoscore call logos_execution_zone list_accounts 2>/dev/null | tail -1)
OWNER="$MANIFEST_OWNER"
PROVIDER="$MANIFEST_PROVIDER"

if [[ -z "$OWNER" ]]; then
OWNER=$(python3 -c '
import json,sys
outer=json.loads(sys.argv[1])
arr=outer.get("result")
if isinstance(arr,str):
  arr=json.loads(arr) if arr.startswith("[") else []
if not isinstance(arr,list):
  sys.exit(0)
for item in arr:
  if isinstance(item,str):
    s=item.replace("Public/","").strip()
    if s.startswith("/ Public"): continue
    if s: print(s); break
  elif isinstance(item,dict):
    for k in ("account_id","accountId","id"):
      if k in item:
        print(str(item[k]).replace("Public/","").strip())
        sys.exit(0)
' "$ACCOUNTS_LINE")
fi

if [[ -z "$OWNER" ]]; then
  echo WALLET_NO_OWNER
  logoscore stop 2>/dev/null || true
  exit 1
fi
if [[ ${#OWNER} -eq 64 ]]; then
  OWNER=$(to_base58 "$OWNER")
fi

if [[ -z "$PROVIDER" ]]; then
PROVIDER_LINE=$(logoscore call logos_execution_zone create_account_public 2>/dev/null | tail -1)
PROVIDER=$(python3 -c '
import json,sys
outer=json.loads(sys.argv[1])
inner=outer.get("result","")
if isinstance(inner,str) and inner.startswith("{"):
  inner=json.loads(inner)
if isinstance(inner,dict):
  for k in ("account_id","accountId","base58","account_id_base58"):
    if k in inner:
      print(str(inner[k]).replace("Public/","").strip())
      sys.exit(0)
s=str(inner).replace("Public/","").strip()
if s: print(s)
' "$PROVIDER_LINE")
fi

if [[ -z "$PROVIDER" ]]; then
  PROVIDER="$OWNER"
elif [[ ${#PROVIDER} -eq 64 ]]; then
  PROVIDER=$(to_base58 "$PROVIDER")
fi
logoscore call logos_execution_zone save 2>/dev/null >/dev/null || true

LEZ_PIN=$(grep -A2 '\[repos.lez\]' "$REPO/scaffold.toml" | grep '^pin' | sed 's/.*"\([^"]*\)".*/\1/')
SCAFFOLD_WALLET="$HOME/.cache/logos-scaffold/repos/lez/${LEZ_PIN}/target/release/wallet"
if [[ -x "$SCAFFOLD_WALLET" ]]; then
  export PATH="$(dirname "$SCAFFOLD_WALLET"):$PATH"
  export LEE_WALLET_HOME_DIR="$WALLET_E2E_DIR"
  timeout 30 lgs wallet topup --address "Public/$OWNER" 2>/dev/null | sed 's/^/TOPUP:/' || true
  timeout 30 lgs wallet topup --address "Public/$PROVIDER" 2>/dev/null | sed 's/^/TOPUP_PROVIDER:/' || true
fi

echo "OWNER:$OWNER"
echo "PROVIDER:$PROVIDER"

sync_wallet

call_ps INIT initializeVault "$(python3 -c "import json; print(json.dumps({'signer':'$OWNER','vault_id':int('$VAULT_ID')}))")"
sync_wallet
call_ps DEPOSIT deposit "$(python3 -c "import json; print(json.dumps({'signer':'$OWNER','vault_id':int('$VAULT_ID'),'amount_lo':int('$DEPOSIT'),'amount_hi':0}))")"
sync_wallet
call_ps CREATE createStream "$(python3 -c "import json; print(json.dumps({'signer':'$OWNER','vault_id':int('$VAULT_ID'),'stream_id':int('$STREAM_ID'),'provider':'$PROVIDER','rate':int('$RATE'),'allocation_lo':int('$ALLOCATION'),'allocation_hi':0}))")"
sync_wallet
call_ps PAUSE pauseStream "$(python3 -c "import json; print(json.dumps({'signer':'$OWNER','vault_id':int('$VAULT_ID'),'stream_id':int('$STREAM_ID')}))")"
sync_wallet
call_ps RESUME resumeStream "$(python3 -c "import json; print(json.dumps({'signer':'$OWNER','vault_id':int('$VAULT_ID'),'stream_id':int('$STREAM_ID')}))")"
sync_wallet
call_ps TOPUP topUpStream "$(python3 -c "import json; print(json.dumps({'signer':'$OWNER','vault_id':int('$VAULT_ID'),'stream_id':int('$STREAM_ID'),'increase_lo':1,'increase_hi':0}))")"
sync_wallet
call_ps CLAIM claim "$(python3 -c "import json; print(json.dumps({'owner':'$OWNER','provider':'$PROVIDER','vault_id':int('$VAULT_ID'),'stream_id':int('$STREAM_ID')}))")"
sync_wallet
call_ps_status_retry VSTATUS getVaultStatus "$(python3 -c "import json; print(json.dumps({'owner':'$OWNER','vault_id':int('$VAULT_ID')}))")" vault_id
call_ps_status_retry SSTATUS getStreamStatus "$(python3 -c "import json; print(json.dumps({'owner':'$OWNER','vault_id':int('$VAULT_ID'),'stream_id':int('$STREAM_ID')}))")" stream_id

logoscore stop 2>/dev/null || true
wait "$DAEMON_PID" 2>/dev/null || true
