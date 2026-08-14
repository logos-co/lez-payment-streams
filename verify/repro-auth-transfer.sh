#!/usr/bin/env bash
# Step 9 helper: authenticated transfer for owner and provider (E2E auth_transfer.sh path).
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=verify/lib/repro-env.sh
source "$REPO_ROOT/verify/lib/repro-env.sh"

user_journey_require_shell
user_journey_require_tools

: "${OWNER:?Set OWNER (Step 7)}"
: "${PROVIDER:?Set PROVIDER (Step 7)}"
WALLET_HOME="${WALLET_HOME:-$(user_journey_default_wallet_home)}"
WALLET_CONFIG="${WALLET_CONFIG:-$WALLET_HOME/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$WALLET_HOME/storage.json}"

export CHAIN=testnet
export PS_AT_LOGOSCORE_WALLET_HANDOFF=1
export LEE_WALLET_HOME_DIR="$WALLET_HOME"
export WALLET_CONFIG WALLET_STORAGE
export ARTIFACT="$REPO_ROOT/.scaffold/e2e/repro-at.jsonl"
mkdir -p "$(dirname "$ARTIFACT")"
: > "$ARTIFACT"

exec "$REPO_ROOT/verify/lib/auth-transfer-ensure.sh" \
  --owner "$OWNER" \
  --provider "$PROVIDER" \
  --artifact "$ARTIFACT" \
  --wallet-home "$WALLET_HOME"
