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
  DATA_LEN="$(curl -s -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$VC\"]}" \
    | python3 -c "import sys,json; d=json.load(sys.stdin).get('result',{}).get('data') or []; print(len(d))")"
  if [[ "$DATA_LEN" -gt 0 ]]; then
    ok "vault_config PDA has on-chain data ($VC)"
  else
    bad "vault_config PDA empty ($VC) — on-chain seed not complete"
  fi
  SC="$(python3 -c "import json; print(json.load(open('$MANIFEST'))['stream_config_account_id'])")"
  SD_LEN="$(curl -s -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$SC\"]}" \
    | python3 -c "import sys,json; d=json.load(sys.stdin).get('result',{}).get('data') or []; print(len(d))")"
  if [[ "$SD_LEN" -gt 0 ]]; then
    ok "stream_config PDA has on-chain data ($SC)"
  else
    bad "stream_config PDA empty ($SC)"
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
