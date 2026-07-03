#!/usr/bin/env bash
# Component tests for vault_scan helpers (localnet must be running for live probes).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"

export LEE_WALLET_HOME_DIR="$(ps_chain_wallet_home)"
export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

pass() {
  echo "PASS: $*"
}

owner="$("$REPO_ROOT/scripts/fixture.sh" vault resolve-id 2>/dev/null | head -1 || true)"
# resolve-id needs owner; use manifest
if [[ -f "$FIXTURE_MANIFEST" ]]; then
  owner="$(ps_json_get "$FIXTURE_MANIFEST" owner_account_id)"
fi
[[ -n "$owner" ]] || fail "no owner in manifest (run prepare-localnet first)"

# vault_config_is_empty on a high unused id should be true when localnet is up
if "$REPO_ROOT/scripts/lifecycle.sh" localnet status 2>/dev/null | grep -q running; then
  high_id=99999
  if "$REPO_ROOT/scripts/fixture.sh" vault config-is-empty "$high_id"; then
    pass "vault config-is-empty $high_id"
  else
    fail "expected vault $high_id config empty on fresh scan id"
  fi
else
  echo "SKIP: localnet not running (vault config-is-empty live probe)"
fi

resolved="$("$REPO_ROOT/scripts/fixture.sh" vault resolve-id)"
[[ "$resolved" =~ ^[0-9]+$ ]] || fail "resolve-id returned non-numeric: $resolved"
pass "vault resolve-id => $resolved"

echo "All vault_scan component checks done."
