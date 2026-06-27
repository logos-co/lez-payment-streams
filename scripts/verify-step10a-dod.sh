#!/usr/bin/env bash
# Verify Step 10a definition of done (see docs/step10a-local-chain-fixture.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}"
LEZ_PIN="$(grep -A2 '\[repos.lez\]' scaffold.toml | grep '^pin' | sed 's/.*"\([^"]*\)".*/\1/')"
export PATH="$HOME/.cache/logos-scaffold/repos/lez/${LEZ_PIN}/target/release:$PATH"

fail=0
ok() { echo "PASS: $*"; }
bad() { echo "FAIL: $*"; fail=1; }

account_on_chain_data_len() {
  local account_id="$1"
  curl -s -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$account_id\"]}" \
    | python3 -c "import sys,json; d=json.load(sys.stdin).get('result',{}).get('data') or []; print(len(d))"
}

check_pda_initialized() {
  local label="$1"
  local account_id="$2"
  local data_len
  data_len="$(account_on_chain_data_len "$account_id")"
  if [[ "$data_len" -gt 0 ]]; then
    ok "$label PDA has on-chain data ($account_id)"
  else
    bad "$label PDA empty ($account_id) — on-chain seed not complete"
  fi
}

echo "=== Step 10a DoD verification ==="

if lgs localnet status 2>/dev/null | grep -q 'listener 127.0.0.1:3040: reachable'; then
  ok "sequencer reachable on 127.0.0.1:3040"
else
  bad "sequencer not reachable on 127.0.0.1:3040"
fi

if lgs wallet -- check-health >/dev/null 2>&1; then
  ok "wallet check-health"
else
  bad "wallet check-health (see foreign localnet / program id troubleshooting)"
fi

MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
if [[ -f "$MANIFEST" ]]; then
  ok "fixture manifest present ($MANIFEST)"
  PROG_MANIFEST="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['program_id_hex'])")"
  PROG_MAKE="$(make program-id 2>/dev/null | grep 'ImageID (hex bytes)' | awk '{print $NF}' || true)"
  if [[ -n "$PROG_MAKE" && "$PROG_MANIFEST" == "$PROG_MAKE" ]]; then
    ok "program_id_hex matches make program-id ImageID"
  else
    bad "program_id_hex mismatch (manifest=$PROG_MANIFEST make=$PROG_MAKE)"
  fi
  CLOCK="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['clock_10_account_id'])")"
  if [[ "$CLOCK" == "4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWSs" ]]; then
    ok "CLOCK_10 in manifest"
  else
    bad "CLOCK_10 unexpected: $CLOCK"
  fi
  VC="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['vault_config_account_id'])")"
  check_pda_initialized "vault_config" "$VC"
  VH="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['vault_holding_account_id'])")"
  check_pda_initialized "vault_holding" "$VH"
  if python3 -c "import json; m=json.load(open('$MANIFEST')); exit(0 if 'stream_config_account_id' in m and m.get('stream_config_account_id') else 1)" 2>/dev/null; then
    SC="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['stream_config_account_id'])")"
    check_pda_initialized "stream_config" "$SC"
  else
    ok "vault-only manifest (no stream_config PDA check)"
  fi
else
  bad "missing $MANIFEST (run ./scripts/seed-localnet-fixture.sh)"
fi

if [[ -f docs/step10a-local-chain-fixture.md ]] && grep -q 'Persist vs reset' docs/step10a-local-chain-fixture.md; then
  ok "reset procedure documented"
else
  bad "reset procedure doc"
fi

echo "=== done (exit $fail) ==="
exit "$fail"
