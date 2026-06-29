#!/usr/bin/env bash
# Verify Step 11c definition of done (see docs/step11c-sign-public-payload.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export REPO="${REPO:-$REPO_ROOT}"
export MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
WALLET_FLAKE="$REPO/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched"
WALLET_LGX="${WALLET_LGX:-$(readlink -f "$WALLET_FLAKE/wallet-lgx-out"/*.lgx 2>/dev/null || true)}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
MANIFEST="${FIXTURE_MANIFEST:-fixtures/localnet.json}"
SMOKE_VERIFY="${SMOKE_VERIFY:-$REPO/target/debug/smoke_verify}"
VERIFY_LOGOSCORE="${VERIFY_LOGOSCORE:-1}"

fail=0
ok() { echo "PASS: $*"; }
bad() { echo "FAIL: $*"; fail=1; }
skip() { echo "SKIP: $*"; }

echo "=== Step 11c DoD verification ==="

if [[ -f "$WALLET_LGX" ]]; then
  ok "patched wallet .lgx present"
else
  bad "missing wallet .lgx (run ./scripts/archive/build-wallet-lgx.sh --impure)"
fi

if [[ -f docs/step11c-sign-public-payload.md ]]; then
  ok "Step 11c runbook present"
else
  bad "missing docs/step11c-sign-public-payload.md"
fi

PLUGIN="$MODULES/logos_execution_zone/logos_execution_zone_plugin.so"
if [[ -f "$PLUGIN" ]]; then
  ok "logos_execution_zone_plugin.so installed"
else
  bad "logos_execution_zone_plugin.so not installed under MODULES=$MODULES (run lgpm install)"
fi

LM_BIN="${LM_BIN:-}"
if [[ -z "$LM_BIN" ]]; then
  # Try well-known local path first to avoid GitHub rate limits from nix shell
  _local_lm="$HOME/Downloads/software/waku/lez-related/logos-cli/lm/bin/lm"
  if [[ -x "$_local_lm" ]]; then
    LM_BIN="$_local_lm"
  fi
fi
_lm_check() {
  if [[ -n "$LM_BIN" ]]; then
    "$LM_BIN" methods "$PLUGIN" 2>/dev/null | grep -q 'sign_public_payload'
  else
    nix shell github:logos-co/logos-module#lm --command bash -c \
      "lm methods '$PLUGIN' 2>/dev/null | grep -q 'sign_public_payload'" 2>/dev/null
  fi
}
if _lm_check; then
  ok "lm methods lists sign_public_payload"
else
  bad "lm methods missing sign_public_payload (reinstall patched .lgx)"
fi

if [[ -f "$SMOKE_VERIFY" ]]; then
  ok "smoke_verify binary present ($SMOKE_VERIFY)"
else
  bad "smoke_verify missing (run: cargo build -p lez-payment-streams-ffi in methods/guest)"
fi

if [[ "$VERIFY_LOGOSCORE" != "1" ]]; then
  skip "VERIFY_LOGOSCORE=0 — skipping sign-then-verify smoke test"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -f "$MANIFEST" ]]; then
  skip "sign-then-verify smoke (no fixture manifest; run Step 10a seed first)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if ! curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' >/dev/null; then
  skip "sign-then-verify smoke (sequencer not reachable; start Step 10a localnet first)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

# sha256("test") as a well-known 32-byte test digest
TEST_DIGEST="9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
E2E_FILE="$(mktemp)"
E2E_TIMEOUT="${LOGOSCORE_E2E_TIMEOUT:-120}"
trap 'rm -f "$E2E_FILE"' EXIT

timeout "$E2E_TIMEOUT" nix shell github:logos-co/logos-logoscore-cli --command bash -c "
  MODULES='$MODULES'
  WALLET_CONFIG='$WALLET_CONFIG'
  WALLET_STORAGE='$WALLET_STORAGE'
  TEST_DIGEST='$TEST_DIGEST'
  logoscore stop 2>/dev/null || true
  sleep 2
  logoscore -D -m \"\$MODULES\" -q &
  DAEMON_PID=\$!
  sleep 3
  logoscore load-module logos_execution_zone 2>&1 | tail -1 | sed 's/^/LOAD_W:/'
  logoscore call logos_execution_zone open \"\$WALLET_CONFIG\" \"\$WALLET_STORAGE\" 2>/dev/null | tail -1 | sed 's/^/OPEN:/'
  # Pick first public account from the wallet (has signing key by construction)
  HEX=\$(logoscore call logos_execution_zone list_accounts 2>/dev/null | tail -1 \
    | python3 -c \"
import sys,json
outer=json.load(sys.stdin)
accounts=outer.get('result')
if isinstance(accounts,str): accounts=json.loads(accounts)
pub=[a for a in accounts if a.get('is_public')]
print(pub[0]['account_id'] if pub else '')
\" || true)
  echo \"HEX:\$HEX\"
  logoscore call logos_execution_zone sign_public_payload \"\$HEX\" \"\$TEST_DIGEST\" 2>/dev/null | tail -1 | sed 's/^/SIGN:'
  logoscore call logos_execution_zone get_public_account_key \"\$HEX\" 2>/dev/null | tail -1 | sed 's/^/PUBKEY:'
  logoscore stop 2>/dev/null || true
  wait \"\$DAEMON_PID\" 2>/dev/null || true
" >"$E2E_FILE" 2>&1 || echo E2E_TIMEOUT_OR_FAIL >>"$E2E_FILE"

if rg -q 'E2E_TIMEOUT_OR_FAIL' "$E2E_FILE" 2>/dev/null; then
  bad "logoscore E2E timed out or failed (${E2E_TIMEOUT}s)"
  tail -15 "$E2E_FILE" >&2 || true
fi

LOAD_W_LINE="$(rg '^LOAD_W:' "$E2E_FILE" | tail -1 | sed 's/^LOAD_W://')"
if python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('status')=='ok' else 1)" "$LOAD_W_LINE" 2>/dev/null; then
  ok "logoscore load-module logos_execution_zone"
else
  bad "logoscore load-module failed: $LOAD_W_LINE"
fi

OPEN_LINE="$(rg '^OPEN:' "$E2E_FILE" | tail -1 | sed 's/^OPEN://')"
if python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('status')=='ok' else 1)" "$OPEN_LINE" 2>/dev/null; then
  ok "open wallet (Step 10a .scaffold/wallet)"
else
  bad "open wallet failed: $OPEN_LINE"
fi

HEX_LINE="$(rg '^HEX:' "$E2E_FILE" | tail -1 | sed 's/^HEX://')"
if [[ -n "$HEX_LINE" && ${#HEX_LINE} -ge 32 ]]; then
  ok "account_id_from_base58 for fixture vault_config PDA"
else
  bad "account_id_from_base58 failed (hex=$HEX_LINE)"
fi

SIGN_LINE="$(rg '^SIGN:' "$E2E_FILE" | tail -1 | sed 's/^SIGN://')"
SIG_HEX="$(python3 -c "
import json,sys
outer=json.loads(sys.argv[1])
if outer.get('status')!='ok':
  print('')
  sys.exit(0)
# sign_public_payload returns a JSON envelope string in result
inner=json.loads(outer.get('result','{}'))
print(inner.get('result',''))
" "$SIGN_LINE" 2>/dev/null || true)"

if [[ ${#SIG_HEX} -eq 128 ]]; then
  ok "sign_public_payload returns 64-byte Schnorr sig (128 hex chars)"
else
  bad "sign_public_payload failed or wrong sig length: $SIGN_LINE"
fi

PUBKEY_LINE="$(rg '^PUBKEY:' "$E2E_FILE" | tail -1 | sed 's/^PUBKEY://')"
PUBKEY_HEX="$(python3 -c "
import json,sys
d=json.loads(sys.argv[1])
if d.get('status')!='ok':
  print('')
  sys.exit(0)
# get_public_account_key returns the hex key directly in result
print(d.get('result',''))
" "$PUBKEY_LINE" 2>/dev/null || true)"

if [[ ${#PUBKEY_HEX} -eq 64 ]]; then
  ok "get_public_account_key returns 32-byte key (64 hex chars)"
else
  bad "get_public_account_key failed or wrong key length: $PUBKEY_LINE"
fi

if [[ -n "$SIG_HEX" && -n "$PUBKEY_HEX" ]]; then
  if "$SMOKE_VERIFY" "$PUBKEY_HEX" "$TEST_DIGEST" "$SIG_HEX"; then
    ok "smoke_verify: Schnorr signature verifies against public key and digest"
  else
    bad "smoke_verify: signature does not verify"
  fi
else
  skip "smoke_verify (missing sig or pubkey from earlier steps)"
fi

echo "=== done (exit $fail) ==="
exit "$fail"
