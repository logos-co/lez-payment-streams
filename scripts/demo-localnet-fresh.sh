#!/usr/bin/env bash
# Blank-slate localnet fixture for demos (see docs/demo-localnet-recovery.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

REINIT_WALLET="${REINIT_WALLET:-0}"
SKIP_VERIFY="${SKIP_VERIFY:-0}"

export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}"

echo "=== demo localnet fresh (blank slate) ==="

if command -v lgs >/dev/null 2>&1; then
  lgs localnet stop 2>/dev/null || true
else
  echo "WARN: lgs not on PATH; skipping localnet stop" >&2
fi

if [[ -d .scaffold/state ]]; then
  rm -rf .scaffold/state/
  echo "removed .scaffold/state/"
fi

rm -f fixtures/localnet.json .lez_payment_streams-state .lez_payment_streams-state.tmp
echo "cleared fixture manifest and .lez_payment_streams-state"

"$REPO_ROOT/scripts/clear-demo-module-persist.sh"

if [[ "$REINIT_WALLET" == "1" ]]; then
  echo "--- reinit scaffold wallet ---"
  "$REPO_ROOT/scripts/reinit-scaffold-wallet.sh"
else
  rm -f .lez_payment_streams-fixture-provider
  echo "cleared .lez_payment_streams-fixture-provider (set REINIT_WALLET=1 if deploy/storage fails)"
fi

echo "--- seed localnet fixture ---"
"$REPO_ROOT/scripts/seed-localnet-fixture.sh"

if [[ "$SKIP_VERIFY" == "1" ]]; then
  echo "SKIP_VERIFY=1 — skipping verify-step10a-dod.sh"
else
  echo "--- verify step 10a ---"
  "$REPO_ROOT/scripts/verify-step10a-dod.sh"
fi

echo "=== done ==="
echo "Next: logoscore with a fresh --persistence-path, then Step 12 or ./scripts/verify-step12-dod.sh"
echo "  export PERSIST_DIR=\"$REPO_ROOT/.scaffold/step12-persist-\$(date +%s)\""
