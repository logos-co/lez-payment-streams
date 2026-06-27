#!/usr/bin/env bash
# Pinata top-up for vault owner between back-to-back E2E legs (leg 2 deposit / create).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/localnet-snapshot-common.sh
source "$REPO_ROOT/scripts/localnet-snapshot-common.sh"

export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}"
ROUNDS="${E2E_CONTINUATION_PINATA_ROUNDS:-12}"

if [[ ! -f "$REPO_ROOT/.lez_payment_streams-state" ]]; then
  echo "ERROR: missing .lez_payment_streams-state" >&2
  exit 1
fi
# shellcheck disable=SC1090
source "$REPO_ROOT/.lez_payment_streams-state"
OWNER="${SIGNER_ID:-}"
if [[ -z "$OWNER" ]]; then
  echo "ERROR: SIGNER_ID missing" >&2
  exit 1
fi

"$REPO_ROOT/scripts/ensure-scaffold-lez-layout.sh"
echo "Continuation owner pinata (${ROUNDS} rounds) for Public/${OWNER}…"
for round in $(seq 1 "$ROUNDS"); do
  echo "  topup round ${round}/${ROUNDS}…"
  lgs wallet topup --address "Public/${OWNER}"
done
