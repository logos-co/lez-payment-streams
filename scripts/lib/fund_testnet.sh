#!/usr/bin/env bash
# fund_testnet.sh — Testnet pinata faucet funding helpers.
# Sourced by module-e2e.sh and scripts/fund-testnet-accounts.sh (do not execute).
#
# pinata claim pays ~150 tokens per call and is rate-limited, so claims are
# looped until the account reaches the target (or the attempt cap). Balance is
# read directly from the sequencer via getAccount, so no logoscore wallet sync
# is required here. Idempotent: accounts already above target short-circuit.

set -euo pipefail

[[ -n "${PS_FUND_TESTNET_SOURCED:-}" ]] && return 0
PS_FUND_TESTNET_SOURCED=1

# ps_fund_testnet_account <account> <target> [max_attempts]
# Claim pinata tokens for <account> until its sequencer balance >= <target>.
# Echoes the final balance on stdout; returns 0 when the target is met, 1
# otherwise (including when the scaffold wallet binary is missing). Safe under
# set -e when invoked in an if/|| context.
ps_fund_testnet_account() {
  local acct="$1" target="$2" max="${3:-6}"
  local scaffold_wallet bal attempts
  export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$(ps_chain_wallet_home)}"

  scaffold_wallet="$(ps_lez_cache)/target/release/wallet"
  if [[ ! -x "$scaffold_wallet" ]]; then
    ps_log_error "Scaffold wallet not found: $scaffold_wallet (run lgs setup)"
    echo 0
    return 1
  fi
  export PATH="$(dirname "$scaffold_wallet"):$PATH"

  bal="$(ps_account_balance "$acct" 2>/dev/null || echo 0)"
  attempts=0
  while (( bal < target && attempts < max )); do
    attempts=$((attempts + 1))
    "$scaffold_wallet" pinata claim --to "Public/$acct" >/dev/null 2>&1 || true
    sleep 2
    bal="$(ps_account_balance "$acct" 2>/dev/null || echo 0)"
  done
  echo "$bal"
  (( bal >= target ))
}
