#!/usr/bin/env bash
# CLI wrapper for ps_auth_transfer_ensure (Step 32).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/auth_transfer.sh
source "$REPO_ROOT/scripts/lib/auth_transfer.sh"

OWNER=""
PROVIDER=""
ARTIFACT=""
WALLET_HOME=""

usage() {
  echo "Usage: $0 --owner <base58> --provider <base58> --artifact <file> --wallet-home <dir>" >&2
  exit 2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --owner) OWNER="$2"; shift 2 ;;
    --provider) PROVIDER="$2"; shift 2 ;;
    --artifact) ARTIFACT="$2"; shift 2 ;;
    --wallet-home) WALLET_HOME="$2"; shift 2 ;;
    -h|--help) usage ;;
    *) echo "Unknown arg: $1" >&2; usage ;;
  esac
done

[[ -n "$OWNER" && -n "$PROVIDER" && -n "$ARTIFACT" && -n "$WALLET_HOME" ]] || usage

export LEE_WALLET_HOME_DIR="$WALLET_HOME"
if [[ -z "${CHAIN:-}" ]] && [[ "$WALLET_HOME" == *testnet-wallet* ]]; then
  export CHAIN=testnet
fi
if [[ -f "$WALLET_HOME/wallet_config.json" ]]; then
  export WALLET_CONFIG="$WALLET_HOME/wallet_config.json"
fi
if [[ -f "$WALLET_HOME/storage.json" ]]; then
  export WALLET_STORAGE="$WALLET_HOME/storage.json"
fi

export ARTIFACT
: > "$ARTIFACT"

ps_auth_transfer_ensure "$OWNER" "$PROVIDER"
