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

# Default configuration
export SEED_DEPOSIT_AMOUNT="${SEED_DEPOSIT_AMOUNT:-1000}"
export SEED_STREAM_ALLOCATION="${SEED_STREAM_ALLOCATION:-200}"
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
  if [[ -f "$REPO_ROOT/.lez_payment_streams-state" ]]; then
    owner="$(grep '^SIGNER_ID=' "$REPO_ROOT/.lez_payment_streams-state" 2>/dev/null |
      cut -d= -f2 | tr -d '"'\''')"
  fi
  if [[ -z "$owner" ]]; then
    local manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
    [[ -f "$manifest" ]] && owner="$(ps_json_get "$manifest" owner_account_id)"
  fi
  echo "$owner"
}

# A vault is "funded" when its config account is initialized (next_stream_id is
# readable) and, when the holding account id is known, it carries at least one
# stream allocation of unallocated-ish balance.
vault_is_funded() {
  local vault_id="${1:-0}"
  local owner next holding bal min_balance manifest
  owner="$(resolve_owner)"
  [[ -z "$owner" ]] && return 1

  next="$(ps_vault_next_stream_id "$owner" "$vault_id")" || return 1
  [[ -n "$next" ]] || return 1

  manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
  holding=""
  [[ -f "$manifest" ]] && holding="$(ps_json_get "$manifest" vault_holding_account_id)"
  # Initialized but no manifest holding id (e.g. fresh restore): trust the
  # snapshot/seed that funded it.
  [[ -z "$holding" ]] && return 0

  bal="$(ps_account_balance "$holding")"
  min_balance="${SEED_STREAM_ALLOCATION:-200}"
  [[ "${bal:-0}" -ge "$min_balance" ]]
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

  ps_log_info "Writing vault baseline manifest: $manifest"
  LEE_WALLET_HOME_DIR="$wallet_home" cargo run -q \
    --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- write-vault-manifest \
    --program-bin "$guest" \
    --owner "$owner" \
    --provider "$provider" \
    --deposit-amount "$SEED_DEPOSIT_AMOUNT" \
    --stream-rate "$SEED_STREAM_RATE" \
    --allocation "$SEED_STREAM_ALLOCATION" \
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
    ps_log_info "Vault $vault_id already funded (skip initialize/deposit; FORCE_DEPOSIT=1 to override)"
    return 0
  fi

  # Initialize vault if needed
  cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- \
    initialize-vault-onchain \
    --program-bin "${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}" \
    --owner "$owner" \
    --vault-id "$vault_id" 2>/dev/null || ps_log_info "Vault may already exist, continuing..."
  
  # Deposit
  cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- \
    deposit-onchain \
    --program-bin "${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}" \
    --owner "$owner" \
    --vault-id "$vault_id" \
    --deposit-amount "$SEED_DEPOSIT_AMOUNT"
  
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

  ps_log_info "Creating stream $next_stream_id (vault $vault_id, rate=$SEED_STREAM_RATE, allocation=$SEED_STREAM_ALLOCATION)"
  
  cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- \
    create-stream-onchain \
    --program-bin "${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}" \
    --owner "$owner" \
    --provider "$provider" \
    --vault-id "$vault_id" \
    --stream-id "$next_stream_id" \
    --stream-rate "$SEED_STREAM_RATE" \
    --allocation "$SEED_STREAM_ALLOCATION" \
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
  # Note: May not confirm on testnet, that's a known issue
}

# ============================================================================
# Main dispatch
# ============================================================================

usage() {
  cat << EOF
Usage: $0 <command> [args]

Commands:
  prefund                      — Fund owner and provider accounts
  vault ensure [vault-id]      — Initialize vault and deposit if under-funded (default: 0)
  vault is-funded [vault-id]   — Exit 0 if the vault is initialized and funded
  vault manifest               — Write vault-baseline fixture manifest from markers
  stream create [vault-id]     — Create stream (default vault: 0)
  stream close <vault> <id>    — Close specific stream
  stream claim <vault> <id>    — Claim from specific stream

Environment:
  FIXTURE_MANIFEST    — Path to fixture JSON (default: fixtures/localnet.json)
  SEED_DEPOSIT_AMOUNT — Deposit for vault (default: 1000)
  SEED_STREAM_ALLOCATION — Stream allocation (default: 200)
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
    vault)                
      [[ $# -lt 1 ]] && { usage; exit 1; }
      local subcmd="$1"; shift
      case "$subcmd" in
        ensure)           cmd_vault_ensure "$@" ;;
        is-funded)        cmd_vault_is_funded "$@" ;;
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
