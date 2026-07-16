#!/usr/bin/env bash
# module-e2e-privacy.sh — Step 36 PseudonymousFunder lifecycle (Flow A + PRIVACY=1).
#
# Wrapper around module-e2e.sh: private vault owner, public provider, pre-shield funding.
#
# Usage:
#   ./scripts/module-e2e-privacy.sh
#   MODE=module CHAIN=local PRIVACY=1 ./scripts/e2e.sh local run
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PRIVACY=1
export CHAIN="${CHAIN:-local}"
exec "$REPO_ROOT/scripts/module-e2e.sh" "$@"
