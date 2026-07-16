#!/usr/bin/env bash
# Step 5 helper: install journey modules (dev variants for pinned logoscore/lgpm).
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/user-journey-env.sh
source "$REPO_ROOT/scripts/lib/user-journey-env.sh"

user_journey_require_shell
user_journey_require_tools
user_journey_export_module_env

MODULES="${MODULES:-$(user_journey_default_modules_dir)}"
WALLET_HOME="${WALLET_HOME:-$(user_journey_default_wallet_home)}"
WALLET_CONFIG="${WALLET_CONFIG:-$WALLET_HOME/wallet_config.json}"

mkdir -p "$WALLET_HOME" "$MODULES"
cp "$REPO_ROOT/fixtures/testnet-wallet_config.example.json" "$WALLET_CONFIG"

rm -rf "${MODULES:?}/"*

echo "Building payment_streams_module (#lgx, linux-amd64-dev)..." >&2
PS_LGX_OUT="$(nix build "$REPO_ROOT/logos-payment-streams-module#lgx" -L --no-link --print-out-paths | tail -1)"
lgpm --modules-dir "$MODULES" install --file "$PS_LGX_OUT"/*.lgx --force

WALLET_FLAKE="$REPO_ROOT/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched"
echo "Building logos_execution_zone wallet module (dev bundle)..." >&2
(
  cd "$WALLET_FLAKE"
  nix bundle --impure --bundler github:logos-co/nix-bundle-lgx .#lib -o wallet-lgx-out -L
)
WALLET_LGX="$(readlink -f "$WALLET_FLAKE/wallet-lgx-out/"*.lgx)"
lgpm --modules-dir "$MODULES" install --file "$WALLET_LGX" --force

echo "Installed under: $MODULES"
ls -1 "$MODULES"
