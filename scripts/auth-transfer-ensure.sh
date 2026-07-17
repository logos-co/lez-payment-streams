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
SKIP_OWNER=0
SKIP_PROVIDER=0

usage() {
  echo "Usage: $0 --owner <base58> --provider <base58> --artifact <file> --wallet-home <dir> [--skip-owner] [--skip-provider]" >&2
  exit 2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --owner) OWNER="$2"; shift 2 ;;
    --provider) PROVIDER="$2"; shift 2 ;;
    --artifact) ARTIFACT="$2"; shift 2 ;;
    --wallet-home) WALLET_HOME="$2"; shift 2 ;;
    --skip-owner) SKIP_OWNER=1; shift ;;
    --skip-provider) SKIP_PROVIDER=1; shift ;;
    -h|--help) usage ;;
    *) echo "Unknown arg: $1" >&2; usage ;;
  esac
done

[[ -n "$ARTIFACT" && -n "$WALLET_HOME" ]] || usage
if [[ "$SKIP_OWNER" != "1" && -z "$OWNER" ]]; then usage; fi
if [[ "$SKIP_PROVIDER" != "1" && -z "$PROVIDER" ]]; then usage; fi

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

# D37.11 / D38.8: AT-init public accounts only. Private owner/provider skip.
if [[ "$SKIP_OWNER" == "1" && "$SKIP_PROVIDER" == "1" ]]; then
  echo '{"phase":"auth_init_skip","ok":true,"extra":{"reason":"both_private"}}' >> "$ARTIFACT"
  exit 0
fi
if [[ "$SKIP_OWNER" != "1" ]]; then
  ps_auth_transfer_init_one "$OWNER" auth_init_owner || exit 1
fi
if [[ "$SKIP_PROVIDER" != "1" ]]; then
  ps_auth_transfer_init_one "$PROVIDER" auth_init_provider || exit 1
fi
exit 0
