#!/usr/bin/env bash
# fund-testnet-accounts.sh — Pre-fund testnet fixture owner/provider via pinata.
#
# Run this before a live module-e2e demo so the run does not pay faucet-wait
# latency inline. Idempotent: accounts already above target are left alone.
# After pre-funding, launch the demo with MODULE_E2E_SKIP_FUND=1.
#
# Usage:
#   ./scripts/fund-testnet-accounts.sh
#   OWNER_TARGET=700 PROVIDER_MIN=100 ./scripts/fund-testnet-accounts.sh
#   OWNER=<base58> PROVIDER=<base58> ./scripts/fund-testnet-accounts.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"
# shellcheck source=scripts/lib/fund_testnet.sh
source "$REPO_ROOT/scripts/lib/fund_testnet.sh"

CHAIN=testnet
export CHAIN

FIXTURE="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet-module.json}"
[[ -f "$FIXTURE" ]] || FIXTURE="$REPO_ROOT/fixtures/testnet.json"
[[ -f "$FIXTURE" ]] || ps_fatal "Testnet fixture not found (run: make bootstrap-testnet-module)"

OWNER="${OWNER:-$(ps_json_get "$FIXTURE" owner_account_id)}"
PROVIDER="${PROVIDER:-$(ps_json_get "$FIXTURE" provider_account_id)}"
[[ -n "$OWNER" ]] || ps_fatal "fixture missing owner_account_id"
[[ -n "$PROVIDER" ]] || ps_fatal "fixture missing provider_account_id"

DEPOSIT="${DEPOSIT:-500}"
OWNER_TARGET="${OWNER_TARGET:-$((DEPOSIT + 50))}"
PROVIDER_MIN="${PROVIDER_MIN:-50}"
OWNER_MAX="${OWNER_MAX:-6}"
PROVIDER_MAX="${PROVIDER_MAX:-3}"

ps_log_info "Funding testnet accounts via pinata faucet"
ps_log_info "  owner=$OWNER target=$OWNER_TARGET (max $OWNER_MAX claims)"
ps_log_info "  provider=$PROVIDER min=$PROVIDER_MIN (max $PROVIDER_MAX claims)"

bal=""
if ! bal="$(ps_fund_testnet_account "$OWNER" "$OWNER_TARGET" "$OWNER_MAX")"; then
  ps_log_error "Owner funding short: balance=${bal:-0} target=$OWNER_TARGET"
  exit 1
fi
ps_log_info "Owner balance: $bal"

bal=""
if ! bal="$(ps_fund_testnet_account "$PROVIDER" "$PROVIDER_MIN" "$PROVIDER_MAX")"; then
  ps_log_error "Provider funding short: balance=${bal:-0} min=$PROVIDER_MIN"
  exit 1
fi
ps_log_info "Provider balance: $bal"
ps_log_info "Testnet accounts funded. Run the demo with MODULE_E2E_SKIP_FUND=1."
