#!/usr/bin/env bash
# Build and launch Basecamp with payment_streams_ui env (Step 21).
# NETWORK=testnet (default) or localnet. WALLET_HOME and FIXTURE_MANIFEST override defaults.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NETWORK="${NETWORK:-testnet}"
USER_DIR="${USER_DIR:-$REPO_ROOT/.scaffold/basecamp-ui}"
BASECAMP="${BASECAMP:-$REPO_ROOT/../logos-basecamp/result/bin/LogosBasecamp}"

ps_basecamp_ui_usage() {
  echo "Usage: NETWORK=testnet|localnet $0 build|run" >&2
  exit 2
}

ps_basecamp_ui_export_env() {
  case "$NETWORK" in
    testnet)
      : "${WALLET_HOME:=$REPO_ROOT/.scaffold/e2e/testnet-wallet}"
      : "${FIXTURE_MANIFEST:=$REPO_ROOT/verify/fixtures/testnet-module.json}"
      ;;
    localnet)
      : "${WALLET_HOME:=$REPO_ROOT/.scaffold/wallet}"
      : "${FIXTURE_MANIFEST:=$REPO_ROOT/verify/fixtures/localnet.json}"
      ;;
    *)
      echo "NETWORK must be testnet or localnet (got '$NETWORK')" >&2
      exit 1
      ;;
  esac
  export USER_DIR
  export REPO="$REPO_ROOT"
  export WALLET_HOME
  export LEE_WALLET_HOME_DIR="$WALLET_HOME"
  export NSSA_WALLET_HOME_DIR="$WALLET_HOME"
  export FIXTURE_MANIFEST
}

ps_basecamp_ui_build() {
  if [[ ! -x "$BASECAMP" ]]; then
    if [[ ! -f "$REPO_ROOT/../logos-basecamp/flake.nix" ]]; then
      echo "Nix-built Basecamp missing at $BASECAMP" >&2
      echo "Clone logos-basecamp as a sibling and run: (cd ../logos-basecamp && nix build '.#bin-bundle-dir')" >&2
      exit 1
    fi
    ( cd "$REPO_ROOT/../logos-basecamp" && nix build '.#bin-bundle-dir' )
  fi
  "$REPO_ROOT/verify/lib/build-wallet-lgx.sh"
  nix build "$REPO_ROOT/module#lgx-portable"
  nix build "$REPO_ROOT/ui#lgx-portable" -o "$REPO_ROOT/ui/result"
  echo "Packages ready. Install into Basecamp user dir $USER_DIR"
  echo "  wallet: $REPO_ROOT/module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/"
  echo "  module: $REPO_ROOT/result/"
  echo "  ui:     $REPO_ROOT/ui/result/"
}

ps_basecamp_ui_run() {
  ps_basecamp_ui_export_env
  if [[ ! -x "$BASECAMP" ]]; then
    echo "Basecamp binary missing at $BASECAMP. Run: make basecamp-ui-build" >&2
    exit 1
  fi
  if [[ ! -f "$FIXTURE_MANIFEST" ]]; then
    echo "FIXTURE_MANIFEST missing: $FIXTURE_MANIFEST" >&2
    exit 1
  fi
  mkdir -p "$USER_DIR"
  if command -v pgrep >/dev/null 2>&1 && pgrep -x logoscore >/dev/null 2>&1; then
    echo "logoscore is running. Stop it so storage.json is free for Basecamp (N52)." >&2
  fi
  echo "NETWORK=$NETWORK WALLET_HOME=$WALLET_HOME FIXTURE_MANIFEST=$FIXTURE_MANIFEST"
  exec "$BASECAMP" --user-dir "$USER_DIR"
}

cmd="${1:-}"
case "$cmd" in
  build) ps_basecamp_ui_build ;;
  run) ps_basecamp_ui_run ;;
  *) ps_basecamp_ui_usage ;;
esac
