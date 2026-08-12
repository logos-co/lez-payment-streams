#!/usr/bin/env bash
# Moved to scripts/lib/testnet-common.sh (Step 45). This stub keeps archived
# scripts working until they are updated or deleted.
set -euo pipefail
if [[ -z "${REPO_ROOT:-}" ]]; then
  REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fi
# shellcheck source=scripts/lib/testnet-common.sh
source "$REPO_ROOT/scripts/lib/testnet-common.sh"
