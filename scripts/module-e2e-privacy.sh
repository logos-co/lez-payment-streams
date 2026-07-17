#!/usr/bin/env bash
# module-e2e-privacy.sh — owner-privacy module lifecycle (Flow A + OWNER_PRIVACY=1).
#
# Wrapper around module-e2e.sh: private vault owner, public provider, pre-shield funding.
# PRIVACY=1 is accepted as an alias for OWNER_PRIVACY=1.
#
# Usage:
#   ./scripts/module-e2e-privacy.sh
#   MODE=module CHAIN=local OWNER_PRIVACY=1 ./scripts/e2e.sh local run
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export OWNER_PRIVACY=1
export PRIVACY=1
export CHAIN="${CHAIN:-local}"
exec "$REPO_ROOT/scripts/module-e2e.sh" "$@"
