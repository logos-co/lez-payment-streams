#!/usr/bin/env bash
# Step 9 helper: authenticated transfer for payer and payee (E2E auth_transfer.sh path).
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/user-journey-env.sh
source "$REPO_ROOT/scripts/lib/user-journey-env.sh"

user_journey_require_shell
user_journey_require_tools

: "${PAYER:?Set PAYER (Step 7)}"
: "${PAYEE:?Set PAYEE (Step 7)}"
WALLET_HOME="${WALLET_HOME:-$(user_journey_default_wallet_home)}"
WALLET_CONFIG="${WALLET_CONFIG:-$WALLET_HOME/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$WALLET_HOME/storage.json}"

export CHAIN=testnet
export PS_AT_LOGOSCORE_WALLET_HANDOFF=1
export LEE_WALLET_HOME_DIR="$WALLET_HOME"
export WALLET_CONFIG WALLET_STORAGE
export ARTIFACT="$REPO_ROOT/.scaffold/e2e/user-journey-at.jsonl"
mkdir -p "$(dirname "$ARTIFACT")"
: > "$ARTIFACT"

exec "$REPO_ROOT/scripts/auth-transfer-ensure.sh" \
  --owner "$PAYER" \
  --provider "$PAYEE" \
  --artifact "$ARTIFACT" \
  --wallet-home "$WALLET_HOME"
