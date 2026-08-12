#!/usr/bin/env bash
# Historical entrypoint; live builder is scripts/build-wallet-lgx.sh (Step 45 D45.18).
set -euo pipefail
exec "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/build-wallet-lgx.sh" "$@"
