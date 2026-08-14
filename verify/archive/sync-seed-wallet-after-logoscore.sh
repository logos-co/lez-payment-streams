#!/usr/bin/env bash
# Close logoscore wallet and sync scaffold storage so seed_localnet_fixture txs confirm after verify smokes.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO_ROOT/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO_ROOT/.scaffold/wallet/storage.json}"
SEQ_URL="${SEQUENCER_URL:-http://127.0.0.1:3040}"

logoscore stop 2>/dev/null || true
sleep 2

if [[ ! -f "$WALLET_CONFIG" || ! -f "$WALLET_STORAGE" ]]; then
  exit 0
fi

if ! curl -sf -X POST "$SEQ_URL" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' >/dev/null; then
  exit 0
fi

nix shell github:logos-co/logos-logoscore-cli --command bash -c "
  set -uo pipefail
  export MODULES='$MODULES'
  logoscore -D -m \"\$MODULES\" --persistence-path '$REPO_ROOT/.scaffold/verify-wallet-sync-persist' -q &
  sleep 3
  logoscore load-module logos_execution_zone >/dev/null 2>&1 || true
  logoscore call logos_execution_zone open '$WALLET_CONFIG' '$WALLET_STORAGE' >/dev/null 2>&1 || true
  height=\$(curl -sf -X POST '$SEQ_URL' -H 'Content-Type: application/json' \
    -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getLastBlockId\",\"params\":[]}' \
    | python3 -c 'import json,sys; d=json.load(sys.stdin); r=d.get(\"result\"); print(r if isinstance(r,int) else (r or \"\"))' 2>/dev/null || true)
  if [[ -n \"\$height\" ]]; then
    logoscore call logos_execution_zone sync_to_block \"\$height\" >/dev/null 2>&1 || true
  fi
  logoscore call logos_execution_zone close >/dev/null 2>&1 || true
  logoscore stop 2>/dev/null || true
" >/dev/null 2>&1 || true

sleep 1
