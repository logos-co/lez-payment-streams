#!/usr/bin/env bash
# Build logos_execution_zone .lgx from the patched PR 19 wrapper flake (Step 10b).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FLAKE_DIR="$REPO_ROOT/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched"
OUT_DIR="$FLAKE_DIR/wallet-lgx-out"

cd "$FLAKE_DIR"
# --impure is required so Nix can read the LBC prebuilt libs from /tmp/lbc-pol-v0.5.0/.
# #portable bundler emits linux-amd64 (non-dev) variant required by lgpm 0.2.0 / logoscore.
nix bundle --impure --bundler github:logos-co/nix-bundle-lgx#portable .#lib -o ./wallet-lgx-out -L

LGX="$(readlink -f "$OUT_DIR"/*.lgx)"
echo "WALLET_LGX=$LGX"
test -f "$LGX"
