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
# shellcheck source=scripts/lib/chain_poll.sh
source "$REPO_ROOT/scripts/lib/chain_poll.sh"
# shellcheck source=scripts/lib/auth_transfer.sh
source "$REPO_ROOT/scripts/lib/auth_transfer.sh"
# shellcheck source=scripts/lib/fund_testnet.sh
source "$REPO_ROOT/scripts/lib/fund_testnet.sh"

CHAIN=testnet
export CHAIN

# ps_auth_transfer_init_one records phases to ARTIFACT; give it a fund log.
ARTIFACT="${ARTIFACT:-$(ps_e2e_artifacts_dir)/fund-testnet-$(date +%Y%m%dT%H%M%S).log}"
mkdir -p "$(dirname "$ARTIFACT")"
: > "$ARTIFACT"
export ARTIFACT

# Always resolve a testnet fixture: ignore a stale FIXTURE_MANIFEST pointing at
# a non-testnet manifest (e.g. localnet.json left in the shell env), which would
# fund accounts the testnet wallet does not own.
FIXTURE="$REPO_ROOT/fixtures/testnet-module.json"
[[ -f "$FIXTURE" ]] || FIXTURE="$REPO_ROOT/fixtures/testnet.json"
[[ -f "$FIXTURE" ]] || ps_fatal "Testnet fixture not found (run: make bootstrap-testnet-module)"

OWNER="${OWNER:-$(ps_json_get "$FIXTURE" owner_account_id)}"
PROVIDER="${PROVIDER:-$(ps_json_get "$FIXTURE" provider_account_id)}"
[[ -n "$OWNER" ]] || ps_fatal "fixture missing owner_account_id"
[[ -n "$PROVIDER" ]] || ps_fatal "fixture missing provider_account_id"

export LEE_WALLET_HOME_DIR="$(ps_chain_wallet_home)"

# The pinata faucet refuses uninitialized accounts, so AT-init both first.
# Idempotent: already-initialized accounts (the common case) short-circuit.
ps_log_info "Ensuring authenticated_transfer init for owner and provider"
if ! ps_auth_transfer_ensure "$OWNER" "$PROVIDER"; then
  ps_log_error "authenticated_transfer init failed (see $ARTIFACT)"
  exit 1
fi

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
