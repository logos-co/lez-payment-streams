#!/usr/bin/env bash
# fixture.sh — Chain state operations (vaults, streams, funding)
# Usage: ./scripts/fixture.sh <command> [args]
#
# Commands:
#   prefund                    — Fund owner and provider accounts
#   vault ensure              — Initialize vault and deposit
#   stream create             — Create stream at next available id
#   stream close <vault> <id> — Close specific stream
#   stream claim <vault> <id> — Claim from specific stream

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"
# shellcheck source=scripts/lib/auth_transfer.sh
source "$REPO_ROOT/scripts/lib/auth_transfer.sh"

# Default configuration
export SEED_DEPOSIT_AMOUNT="${SEED_DEPOSIT_AMOUNT:-1000}"
export SEED_ALLOCATION="${SEED_ALLOCATION:-${SEED_STREAM_ALLOCATION:-200}}"
export SEED_STREAM_RATE="${SEED_STREAM_RATE:-1}"

# Seed CLI sequencer follows the wallet home, which MUST match CHAIN. Set it
# unconditionally: a localnet LEE_WALLET_HOME_DIR inherited from the environment
# would otherwise silently route testnet chain ops to 127.0.0.1:3040.
export LEE_WALLET_HOME_DIR="$(ps_chain_wallet_home)"

# Wait for Clock10 to track wall time before submitting transactions.
#
# Skew tolerance is chain-dependent. Localnet mints blocks every few seconds so
# the clock tracks wall time tightly. Public testnet lands a block only ~once
# per minute, so Clock10 legitimately trails wall time by up to a block
# interval; a 5s tolerance there is unsatisfiable except in the brief window
# right after each block. The seed binary already polls internally until its
# own timeout, so call it once and surface its output instead of wrapping it in
# a second swallow-stderr loop.
wait_clock_synced() {
  local max_skew timeout_s
  if ps_is_testnet; then
    max_skew="${CLOCK_MAX_SKEW_S:-120}"
    timeout_s="${CLOCK_SYNC_TIMEOUT_S:-300}"
  else
    max_skew="${CLOCK_MAX_SKEW_S:-5}"
    timeout_s="${CLOCK_SYNC_TIMEOUT_S:-120}"
  fi
  ps_log_info "Waiting for clock sync (max_skew=${max_skew}s, timeout=${timeout_s}s)..."
  if cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
      --bin seed_localnet_fixture -- wait-clock-synced \
      --max-skew-s "$max_skew" --timeout-s "$timeout_s" >&2; then
    ps_log_info "Clock synced"
    return 0
  fi
  ps_fatal "Clock sync failed (max_skew=${max_skew}s, timeout=${timeout_s}s)"
}

# Chain settle (wait for blocks to be committed)
wait_chain_settle() {
  ps_log_info "Waiting for chain settle..."
  sleep 3
}

# Pinata prize per successful claim (matches lez/programs/pinata PRIZE).
PINATA_PRIZE="${PINATA_PRIZE:-150}"

# Fund an account from the pinata faucet so it has at least `target` balance.
# Under LEZ v0.2.0 the account must first be initialized under the
# authenticated_transfer program (auth-transfer init) before the pinata
# program can credit it; an uninitialized or non-default-but-default-owned
# account is rejected with NonDefaultAccountWithDefaultOwner.
#
# Uses the pinned LEZ wallet on PATH (see make deploy / scripts/lib/common.sh).
fund_owner_account() {
  local owner="$1" target="${2:-$SEED_DEPOSIT_AMOUNT}" bal
  [[ -z "$owner" ]] && ps_fatal "fund_owner_account: owner is empty"

  # The wallet CLI reads its config/storage from LEE_WALLET_HOME_DIR (default
  # ~/.lee/wallet), and the cargo-installed wallet (0.1.0) cannot parse the
  # v0.2.0 storage format. Point the CLI at the chain wallet home used by the
  # logoscore daemons and put the LEZ-built wallet first on PATH so faucet /
  # deploy calls hit the same wallet the run uses.
  : "${LEE_WALLET_HOME_DIR:=$(ps_chain_wallet_home)}"
  export LEE_WALLET_HOME_DIR
  ps_prepend_lez_wallet_path

  bal="$(ps_account_balance "$owner" 2>/dev/null || echo 0)"
  if (( bal >= target )); then
    ps_log_info "Owner $owner already funded (balance=$bal >= $target); skipping faucet"
    return 0
  fi

  ps_log_info "Initializing owner $owner under authenticated_transfer program..."
  if ! wallet auth-transfer init --account-id "Public/$owner" >/dev/null 2>&1; then
    ps_log_info "auth-transfer init for $owner returned non-zero (may already be initialized); continuing"
  fi
  wait_chain_settle

  ps_log_info "Funding owner $owner via pinata faucet to >= $target (balance now $bal, prize=$PINATA_PRIZE per claim)..."
  local attempts=0 max_attempts
  max_attempts=$(( (target / PINATA_PRIZE) + 2 ))
  while (( bal < target )); do
    attempts=$((attempts + 1))
    if (( attempts > max_attempts )); then
      ps_fatal "Owner $owner not funded after $max_attempts pinata claims (balance=$bal, target=$target)"
    fi
    if ! wallet pinata claim --to "Public/$owner" >/dev/null 2>&1; then
      ps_log_info "pinata claim attempt $attempts returned non-zero; retrying"
      wait_chain_settle
      continue
    fi
    wait_chain_settle
    bal="$(ps_account_balance "$owner" 2>/dev/null || echo 0)"
    ps_log_info "pinata claim $attempts done; owner balance=$bal (target=$target)"
  done
  ps_log_info "Owner $owner funded: balance=$bal"
}

# ============================================================================
# Account funding (harness machinery; pinata faucet)
# ============================================================================

cmd_account_fund_owner() {
  local owner="${1:-}"
  local target="${2:-$SEED_DEPOSIT_AMOUNT}"
  if [[ -z "$owner" ]]; then
    owner="$(resolve_owner)"
  fi
  [[ -z "$owner" ]] && ps_fatal "account fund-owner: no owner account"
  fund_owner_account "$owner" "$target"
}

# ============================================================================
# Prefund — Initial funding of owner and vault deposit (baseline snapshot)
# ============================================================================

cmd_prefund() {
  ps_log_info "Prefunding vault baseline..."

  # Load owner from state file or manifest
  local owner
  if [[ -f "$REPO_ROOT/.lez_payment_streams-state" ]]; then
    # shellcheck source=/dev/null
    source "$REPO_ROOT/.lez_payment_streams-state"
    owner="${SIGNER_ID:-}"
  fi

  if [[ -z "$owner" ]]; then
    local manifest
    manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
    [[ -f "$manifest" ]] && owner="$(ps_json_get "$manifest" owner_account_id)"
  fi

  [[ -z "$owner" ]] && ps_fatal "No owner account found in state or manifest"

  ps_log_info "Prefunding vault for owner: $owner"
  ps_log_info "Deposit amount: $SEED_DEPOSIT_AMOUNT"

  # The pinned LEZ wallet (v0.2.0 storage format) must be on PATH for the
  # faucet and deploy steps. The cargo-installed wallet (0.1.0) cannot read
  # v0.2.0 storage. Prefer the LEZ-built binary from the scaffold cache.
  local lez_wallet_dir
  lez_wallet_dir="$(ps_lez_cache)/target/release"
  if [[ -x "$lez_wallet_dir/wallet" ]]; then
    export PATH="$lez_wallet_dir:$PATH"
  fi
  ps_prepend_lez_wallet_path

  # Deploy the guest program if not already on chain (idempotent).
  # prefund-onchain submits initialize_vault + deposit, both of which
  # reference the guest program id, so the program must exist first.
  if ! wallet deploy-program "${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}" >/dev/null 2>&1; then
    ps_log_info "Program deploy returned non-zero (may already be deployed); continuing"
  fi
  wait_chain_settle

  # LEZ v0.2.0: the deposit instruction chains into authenticated_transfer
  # to debit the owner, so the owner must hold at least the deposit amount
  # before prefund-onchain runs. The localnet genesis does not pre-fund
  # wallet-derived accounts, so pull funds from the pinata faucet first.
  # See docs/plan/upcoming/step-27-claim-fix-verification.md (Symptom A).
  fund_owner_account "$owner" "$SEED_DEPOSIT_AMOUNT"

  local manifest provider
  manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
  if [[ -f "$manifest" ]]; then
    provider="$(ps_json_get "$manifest" provider_account_id)"
  fi
  if [[ -n "${provider:-}" ]]; then
    FIXTURE_ARTIFACT="${FIXTURE_ARTIFACT:-$(ps_e2e_artifacts_dir)/fixture-prefund-$(date +%Y%m%dT%H%M%S).log}"
    export ARTIFACT="$FIXTURE_ARTIFACT"
    : > "$ARTIFACT"
    ps_log_info "Ensuring authenticated_transfer for owner and provider (artifact=$ARTIFACT)"
    ps_auth_transfer_ensure "$owner" "$provider"
  fi

  # Run prefund via seed binary (prefund-onchain initializes vault + deposits)
  cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- \
    prefund-onchain \
    --program-bin "${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}" \
    --owner "$owner" \
    --deposit-amount "$SEED_DEPOSIT_AMOUNT"

  wait_chain_settle
  ps_log_info "Prefund complete"
}

# ============================================================================
# Vault operations
# ============================================================================

# Resolve the owner account id from the state marker first (survives a manifest
# reset on restore), then fall back to the fixture manifest.
resolve_owner() {
  local owner=""
  # The localnet SIGNER_ID state file does not apply to testnet, whose owner is
  # the manifest vault owner; a stale localnet signer would redirect vault probes
  # to the wrong account and make resolve_store_vault_id return 0 unconditionally.
  if ! ps_is_testnet && [[ -f "$REPO_ROOT/.lez_payment_streams-state" ]]; then
    owner="$(grep '^SIGNER_ID=' "$REPO_ROOT/.lez_payment_streams-state" 2>/dev/null |
      cut -d= -f2 | tr -d '"'\''')"
  fi
  if [[ -z "$owner" ]]; then
    local manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
    [[ -f "$manifest" ]] && owner="$(ps_json_get "$manifest" owner_account_id)"
  fi
  echo "$owner"
}

# shellcheck source=scripts/lib/vault_scan.sh
source "$REPO_ROOT/scripts/lib/vault_scan.sh"

# A vault is "funded" when its config account is initialized (next_stream_id is
# readable) and its unallocated balance (holding - total_allocated) covers at
# least one createStream at the configured allocation. Reads unallocated
# directly from chain via the seed binary so the check is correct for any
# vault id, not just the one whose PDAs are in the manifest.
vault_is_funded() {
  local vault_id="${1:-0}"
  local owner next unallocated min_balance
  owner="$(resolve_owner)"
  [[ -z "$owner" ]] && return 1

  next="$(ps_vault_next_stream_id "$owner" "$vault_id")" || return 1
  [[ -n "$next" ]] || return 1

  unallocated="$(ps_vault_unallocated_lo "$owner" "$vault_id" 2>/dev/null)" || return 1
  min_balance="${SEED_ALLOCATION:-200}"
  [[ "${unallocated:-0}" -ge "$min_balance" ]]
}

cmd_vault_is_funded() {
  local vault_id="${1:-0}"
  if vault_is_funded "$vault_id"; then
    ps_log_info "Vault $vault_id is funded"
    return 0
  fi
  ps_log_info "Vault $vault_id is not funded"
  return 1
}

cmd_vault_config_is_empty() {
  local vault_id="${1:-0}"
  local owner
  owner="$(resolve_owner)"
  [[ -n "$owner" ]] || ps_fatal "No owner in state/manifest"
  if vault_config_is_empty "$owner" "$vault_id"; then
    ps_log_info "Vault $vault_id config is empty (uninitialized)"
    return 0
  fi
  ps_log_info "Vault $vault_id config is initialized"
  return 1
}

cmd_vault_resolve_id() {
  resolve_store_vault_id
}

# Write a vault-baseline fixture manifest (schema v2, no per-run stream fields)
# from the restored owner/provider markers. The orchestrator reads this for
# owner/provider/program_id and then creates the per-run stream from chain.
cmd_manifest_write() {
  local owner provider manifest guest wallet_home
  owner="$(resolve_owner)"
  [[ -z "$owner" ]] && ps_fatal "No owner in state/manifest for manifest write"
  [[ -f "$REPO_ROOT/.lez_payment_streams-fixture-provider" ]] ||
    ps_fatal "Missing provider marker (.lez_payment_streams-fixture-provider)"
  provider="$(cat "$REPO_ROOT/.lez_payment_streams-fixture-provider")"
  manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
  guest="${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
  wallet_home="${LEE_WALLET_HOME_DIR:-$(ps_chain_wallet_home)}"

  local vault_id="${1:-${VAULT_ID:-0}}"
  ps_log_info "Writing vault baseline manifest: $manifest (vault_id=$vault_id)"
  LEE_WALLET_HOME_DIR="$wallet_home" cargo run -q \
    --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- write-vault-manifest \
    --program-bin "$guest" \
    --owner "$owner" \
    --provider "$provider" \
    --vault-id "$vault_id" \
    --deposit-amount "$SEED_DEPOSIT_AMOUNT" \
    --stream-rate "$SEED_STREAM_RATE" \
    --allocation "$SEED_ALLOCATION" \
    --sequencer-url "$(ps_seq_url)" \
    --output "$manifest"
}

cmd_vault_ensure() {
  ps_log_info "Ensuring vault exists..."

  local owner vault_id
  owner="$(resolve_owner)"
  vault_id="${1:-0}"
  [[ -z "$owner" ]] && ps_fatal "No owner account found in state or manifest"

  wait_clock_synced

  # Idempotent: a funded vault needs no re-init/deposit. Re-running deposit on a
  # live ledger churns the wallet nonce and risks wallet/chain desync.
  if [[ "${FORCE_DEPOSIT:-0}" != "1" ]] && vault_is_funded "$vault_id"; then
    ps_log_info "Vault $vault_id already funded (skip prefund; FORCE_DEPOSIT=1 to override)"
    return 0
  fi

  # The deposit instruction chains into authenticated_transfer to debit the
  # owner, so the owner must hold at least the deposit amount. Top up from
  # the pinata faucet when the balance is short (localnet only; testnet
  # assumes the operator pre-funds the owner).
  if ! ps_is_testnet; then
    fund_owner_account "$owner" "$SEED_DEPOSIT_AMOUNT"
  fi

  local prefund_extra=()
  if [[ "${FORCE_DEPOSIT:-0}" == "1" ]]; then
    prefund_extra+=(--force)
  fi

  local ensure_attempts=0 max_ensure_attempts="${VAULT_ENSURE_MAX_RETRIES:-3}"
  while true; do
    ensure_attempts=$((ensure_attempts + 1))
    # After the first attempt, force the deposit even if the vault config
    # account exists: the init may have landed but the deposit confirm raced.
    local attempt_extra=()
    (( ensure_attempts > 1 )) && attempt_extra=(--force)
    cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
      --bin seed_localnet_fixture -- \
      prefund-onchain \
      --program-bin "${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}" \
      --owner "$owner" \
      --vault-id "$vault_id" \
      --deposit-amount "$SEED_DEPOSIT_AMOUNT" \
      "${prefund_extra[@]}" "${attempt_extra[@]}" && break
    if vault_is_funded "$vault_id"; then
      ps_log_info "prefund-onchain returned error but vault $vault_id is funded on chain (confirm race)"
      break
    fi
    if (( ensure_attempts >= max_ensure_attempts )); then
      ps_log_error "prefund-onchain failed after $max_ensure_attempts attempts and vault $vault_id is not funded"
      return 1
    fi
    ps_log_info "prefund-onchain attempt $ensure_attempts failed; waiting for chain settle before retry"
    wait_chain_settle
  done

  wait_chain_settle
  ps_log_info "Vault $vault_id ensured with $SEED_DEPOSIT_AMOUNT"
}

# ============================================================================
# Stream operations
# ============================================================================

cmd_stream_create() {
  ps_manifest_validate_exists
  
  ps_log_info "Creating stream..."
  
  local owner provider manifest vault_id next_stream_id
  manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
  owner="$(ps_json_get "$manifest" owner_account_id)"
  provider="$(ps_json_get "$manifest" provider_account_id)"
  vault_id="${1:-0}"
  
  wait_clock_synced
  wait_chain_settle
  
  # Stream id contract: the orchestrator passes the chain-derived id in
  # STREAM_ID; NEXT_STREAM_ID is the legacy name; otherwise fall back to
  # manifest stream_id + 1. The manifest baseline has no stream_id, so an
  # explicit id from the caller is required for correct per-run creation.
  next_stream_id="${STREAM_ID:-${NEXT_STREAM_ID:-$(python3 -c "
import json
with open('$manifest') as f:
    data = json.load(f)
    existing = int(data.get('stream_id', -1))
    print(existing + 1)
")}}"

  # CREATE_FORCE bypasses skip-if-initialized so a per-run create proceeds even
  # when a same-id PDA lingers from a prior leg.
  local force_args=()
  [[ "${CREATE_FORCE:-0}" == "1" ]] && force_args+=(--force)

  ps_log_info "Creating stream $next_stream_id (vault $vault_id, rate=$SEED_STREAM_RATE, allocation=$SEED_ALLOCATION)"
  
  cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- \
    create-stream-onchain \
    --program-bin "${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}" \
    --owner "$owner" \
    --provider "$provider" \
    --vault-id "$vault_id" \
    --stream-id "$next_stream_id" \
    --stream-rate "$SEED_STREAM_RATE" \
    --allocation "$SEED_ALLOCATION" \
    --sequencer-url "$(ps_seq_url)" \
    --write-manifest "$manifest" \
    ${force_args[@]+"${force_args[@]}"}
  
  wait_chain_settle
  ps_log_info "Stream $next_stream_id created"
  
  # Output for callers that parse the chosen id.
  echo "STREAM_ID=$next_stream_id"
  echo "NEXT_STREAM_ID=$next_stream_id"
}

cmd_stream_close() {
  ps_manifest_validate_exists
  
  local vault_id="${1:-}"
  local stream_id="${2:-}"
  
  [[ -z "$vault_id" ]] && ps_fatal "Usage: stream close <vault-id> <stream-id>"
  [[ -z "$stream_id" ]] && ps_fatal "Usage: stream close <vault-id> <stream-id>"
  
  local owner provider manifest
  manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
  owner="$(ps_json_get "$manifest" owner_account_id)"
  provider="$(ps_json_get "$manifest" provider_account_id)"
  
  ps_log_info "Closing stream $stream_id (vault $vault_id, provider signs)"
  
  wait_chain_settle
  
  cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- \
    close-stream-onchain \
    --program-bin "${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}" \
    --owner "$owner" \
    --provider "$provider" \
    --vault-id "$vault_id" \
    --stream-id "$stream_id"
  
  wait_chain_settle
  ps_log_info "Stream $stream_id closed"
}

cmd_stream_claim() {
  ps_manifest_validate_exists
  
  local vault_id="${1:-}"
  local stream_id="${2:-}"
  
  [[ -z "$vault_id" ]] && ps_fatal "Usage: stream claim <vault-id> <stream-id>"
  [[ -z "$stream_id" ]] && ps_fatal "Usage: stream claim <vault-id> <stream-id>"
  
  local provider manifest
  manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
  provider="$(ps_json_get "$manifest" provider_account_id)"
  
  ps_log_info "Claiming from stream $stream_id (vault $vault_id, provider claims)"
  
  wait_chain_settle
  
  cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- \
    claim-onchain \
    --program-bin "${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}" \
    --provider "$provider" \
    --vault-id "$vault_id" \
    --stream-id "$stream_id"
  
  ps_log_info "Claim submitted for stream $stream_id"
}

# ============================================================================
# Main dispatch
# ============================================================================

usage() {
  cat << EOF
Usage: $0 <command> [args]

Commands:
  prefund                      — Fund owner and provider accounts
  account fund-owner [owner] [amount]
                               — Pinata-fund owner (harness; default owner from manifest)
  vault ensure [vault-id]      — Initialize vault and deposit if under-funded (default: 0)
  vault is-funded [vault-id]   — Exit 0 if the vault is initialized and funded
  vault config-is-empty [id]   — Exit 0 if vault config account is missing or empty
  vault resolve-id             — Print Store run vault id (env VAULT_ID or scan)
  vault manifest [vault-id]    — Write vault-baseline fixture manifest from markers
  stream create [vault-id]     — Create stream (default vault: 0)
  stream close <vault> <id>    — Close specific stream
  stream claim <vault> <id>    — Claim from specific stream

Environment:
  FIXTURE_MANIFEST    — Path to fixture JSON (default: fixtures/localnet.json)
  SEED_DEPOSIT_AMOUNT — Deposit for vault (default: 1000)
  SEED_ALLOCATION       — CreateStream allocation lo (default: 200; legacy env: SEED_STREAM_ALLOCATION)
  SEED_STREAM_RATE    — Stream rate (default: 1)
  PAYMENT_STREAMS_GUEST_BIN — Path to guest binary
EOF
}

main() {
  [[ $# -lt 1 ]] && { usage; exit 1; }
  
  local cmd="$1"
  shift
  
  case "$cmd" in
    prefund)              cmd_prefund "$@" ;;
    account)
      [[ $# -lt 1 ]] && { usage; exit 1; }
      local subcmd="$1"; shift
      case "$subcmd" in
        fund-owner)       cmd_account_fund_owner "$@" ;;
        *)                usage; exit 1 ;;
      esac
      ;;
    vault)                
      [[ $# -lt 1 ]] && { usage; exit 1; }
      local subcmd="$1"; shift
      case "$subcmd" in
        ensure)           cmd_vault_ensure "$@" ;;
        is-funded)        cmd_vault_is_funded "$@" ;;
        config-is-empty)  cmd_vault_config_is_empty "$@" ;;
        resolve-id)       cmd_vault_resolve_id "$@" ;;
        manifest)         cmd_manifest_write "$@" ;;
        *)                usage; exit 1 ;;
      esac
      ;;
    stream)               
      [[ $# -lt 1 ]] && { usage; exit 1; }
      local subcmd="$1"; shift
      case "$subcmd" in
        create)           cmd_stream_create "$@" ;;
        close)            cmd_stream_close "$@" ;;
        claim)            cmd_stream_claim "$@" ;;
        *)                usage; exit 1 ;;
      esac
      ;;
    help|--help|-h)       usage ;;
    *)
      ps_log_error "Unknown command: $cmd"
      usage
      exit 1
      ;;
  esac
}

main "$@"
