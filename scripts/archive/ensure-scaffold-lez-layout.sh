#!/usr/bin/env bash
# Historical entrypoint; live helper is scripts/lib/ensure-scaffold-lez-layout.sh (Step 45 D45.18).
set -euo pipefail
exec "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/lib/ensure-scaffold-lez-layout.sh" "$@"
