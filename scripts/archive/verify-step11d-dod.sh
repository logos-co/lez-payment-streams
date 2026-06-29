#!/usr/bin/env bash
# Verify Step 11d definition of done (see docs/step11d-wallet-510.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export REPO="${REPO:-$REPO_ROOT}"
export MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
LEZ_RC5=27360cb7d6ccb2bfbcca7d171bab8a3938490264
VERIFY_LOGOSCORE="${VERIFY_LOGOSCORE:-1}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
PROGRAM_BIN="${PROGRAM_BIN:-methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"

fail=0
ok() { echo "PASS: $*"; }
bad() { echo "FAIL: $*"; fail=1; }
skip() { echo "SKIP: $*"; }

echo "=== Step 11d DoD verification ==="

LEZ_PIN="$(grep -A2 '\[repos.lez\]' scaffold.toml | grep '^pin' | sed 's/.*"\([^"]*\)".*/\1/')"
if [[ "$LEZ_PIN" == "$LEZ_RC5" ]]; then
  ok "scaffold.toml LEZ pin is v0.2.0-rc5"
else
  bad "scaffold.toml LEZ pin expected $LEZ_RC5 got $LEZ_PIN"
fi

if rg -q "$LEZ_RC5" nix/payment-streams-ffi.nix; then
  ok "payment-streams-ffi.nix pins LEZ rc5"
else
  bad "payment-streams-ffi.nix missing LEZ rc5 rev"
fi

if rg -q "$LEZ_RC5" logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/lez-wallet-ffi-patched/flake.nix; then
  ok "wallet wrapper LEZ input is rc5"
else
  bad "lez-wallet-ffi-patched flake.nix missing rc5 rev"
fi

if [[ -f docs/step11d-wallet-510.md ]]; then
  ok "Step 11d runbook present"
else
  bad "missing docs/step11d-wallet-510.md"
fi

JSON_PATCH=logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-qt-send-generic-public-transaction-json.patch
if [[ -f "$JSON_PATCH" ]] && rg -q 'send_generic_public_transaction_json' "$JSON_PATCH"; then
  ok "wallet JSON submit patch present"
else
  bad "missing or incomplete $JSON_PATCH"
fi

if nix build --no-link "./logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched#lib" >/dev/null 2>&1; then
  ok "patched wallet lib flake evaluates and builds"
else
  bad "nix build logos-execution-zone-module-patched#lib failed"
fi

WALLET_PLUGIN="$MODULES/logos_execution_zone/logos_execution_zone_plugin.so"
if [[ ! -f "$WALLET_PLUGIN" ]]; then
  skip "logos_execution_zone not installed (./scripts/archive/build-wallet-lgx.sh && lgpm install)"
else
  ok "logos_execution_zone plugin present"
  if nix shell github:logos-co/logos-module#lm --command bash -c "
    set -euo pipefail
    out=\$(lm methods '$WALLET_PLUGIN')
    echo \"\$out\" | rg -q 'send_program_deployment_transaction'
    echo \"\$out\" | rg -q 'send_generic_public_transaction_json'
  "; then
    ok "lm methods lists 510 deploy + JSON public submit"
  else
    bad "wallet plugin missing Step 11d methods (reinstall wallet .lgx from patched flake)"
  fi
fi

if [[ "$VERIFY_LOGOSCORE" != "1" ]]; then
  skip "VERIFY_LOGOSCORE=0 — skipping logoscore deploy smoke"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -f "$WALLET_PLUGIN" ]]; then
  skip "logoscore deploy smoke (no wallet plugin)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if ! curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' >/dev/null; then
  skip "logoscore deploy smoke (sequencer not reachable)"
  echo "=== done (exit $fail) ==="
  exit "$fail"
fi

if [[ ! -f "$PROGRAM_BIN" ]]; then
  skip "logoscore deploy smoke (guest ELF missing; optional for empty-ELF wiring test)"
fi

if [[ -x "$REPO_ROOT/scripts/deploy-program-logoscore.sh" ]]; then
  if MODULES="$MODULES" WALLET_CONFIG="$WALLET_CONFIG" WALLET_STORAGE="$WALLET_STORAGE" \
    PROGRAM_BIN="$PROGRAM_BIN" "$REPO_ROOT/scripts/deploy-program-logoscore.sh" >/tmp/step11d-deploy.log 2>&1; then
    ok "logoscore send_program_deployment_transaction smoke"
  else
    bad "logoscore deploy failed (see /tmp/step11d-deploy.log)"
    tail -20 /tmp/step11d-deploy.log >&2 || true
  fi
else
  skip "deploy-program-logoscore.sh not present"
fi

echo "=== done (exit $fail) ==="
exit "$fail"
