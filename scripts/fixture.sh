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

# Wait for clock sync (blocks advance to near wall time)
wait_clock_synced() {
  ps_log_info "Waiting for clock sync..."
  local max_wait=60 waited=0
  while [[ $waited -lt $max_wait ]]; do
    # Simple check: try a transaction or poll clock
    if cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
        --bin seed_localnet_fixture -- wait-clock-synced 2>/dev/null; then
      ps_log_info "Clock synced after ${waited}s"
      return 0
    fi
    sleep 5
    ((waited += 5))
  done
  ps_fatal "Clock sync timeout after ${max_wait}s"
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

cmd_vault_ensure() {
  ps_manifest_validate_exists
  
  ps_log_info "Ensuring vault exists..."
  
  local owner manifest vault_id
  manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
  owner="$(ps_json_get "$manifest" owner_account_id)"
  vault_id="${1:-0}"
  
  wait_clock_synced
  
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
  
  # Determine next stream id
  next_stream_id="${NEXT_STREAM_ID:-$(python3 -c "
import json
with open('$manifest') as f:
    data = json.load(f)
    existing = data.get('stream_id', -1)
    print(existing + 1)
")}"
  
  ps_log_info "Creating stream $next_stream_id (vault $vault_id, rate=$SEED_STREAM_RATE, allocation=$SEED_STREAM_ALLOCATION)"
  
  cargo run -q --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- \
    create-stream-onchain \
    --program-bin "${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}" \
    --owner "$owner" \
    --provider "$provider" \
    --vault-id "$vault_id" \
    --stream-id "$next_stream_id" \
    --rate "$SEED_STREAM_RATE" \
    --allocation "$SEED_STREAM_ALLOCATION"
  
  wait_chain_settle
  ps_log_info "Stream $next_stream_id created"
  
  # Output for manifest update
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
  vault ensure [vault-id]      — Initialize vault and deposit (default: 0)
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
