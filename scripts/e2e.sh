#!/usr/bin/env bash
# e2e.sh — Main entry point for E2E operations
# Usage: ./scripts/e2e.sh <local|testnet> <run|prepare|teardown|build>
#
# Commands:
#   e2e.sh local run         — Full local E2E (prepare + run + teardown)
#   e2e.sh local prepare     — Setup environment only
#   e2e.sh local teardown    — Cleanup only
#   e2e.sh testnet run       — Full testnet E2E
#   e2e.sh build             — Build all modules

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"

# ============================================================================
# Build command
# ============================================================================

cmd_build() {
  ps_log_info "Building modules..."
  
  local modules_user="${MODULES_USER:-$REPO_ROOT/.scaffold/e2e/user/modules}"
  local modules_provider="${MODULES_PROVIDER:-$REPO_ROOT/.scaffold/e2e/provider/modules}"
  
  mkdir -p "$modules_user" "$modules_provider"
  
  # Build payment_streams_module
  ps_log_info "Building payment_streams_module..."
  local ps_out
  ps_out="$(ps_nix_build "$REPO_ROOT/logos-payment-streams-module#lgx")"
  ps_install_lgx "$ps_out"/*.lgx "$modules_user"
  ps_install_lgx "$ps_out"/*.lgx "$modules_provider"
  
  # Build wallet module (patched)
  ps_log_info "Building logos_execution_zone (wallet)..."
  if [[ ! -f "$REPO_ROOT/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/"*.lgx ]]; then
    "$REPO_ROOT/scripts/build-wallet-lgx.sh"
  fi
  local wallet_lgx
  wallet_lgx="$(readlink -f "$REPO_ROOT/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/"*.lgx)"
  ps_install_lgx "$wallet_lgx" "$modules_user"
  ps_install_lgx "$wallet_lgx" "$modules_provider"
  
  # Build delivery_module
  ps_log_info "Building delivery_module..."
  local dm_root="${DELIVERY_MODULE_ROOT:-$REPO_ROOT/../logos-delivery-module}"
  if [[ ! -d "$dm_root" ]]; then
    ps_fatal "DELIVERY_MODULE_ROOT not found: $dm_root"
  fi
  local dm_out
  dm_out="$(ps_nix_build "$dm_root#lgx")"
  ps_install_lgx "$dm_out"/*.lgx "$modules_user"
  ps_install_lgx "$dm_out"/*.lgx "$modules_provider"
  
  # Optional: overlay liblogosdelivery
  if [[ "${SKIP_LIBLOGOSDELIVERY_OVERLAY:-0}" != "1" ]]; then
    local ld_root="${LOGOS_DELIVERY_ROOT:-$REPO_ROOT/../logos-delivery}"
    if [[ -d "$ld_root" && -f "$ld_root/Makefile" ]]; then
      ps_log_info "Overlaying liblogosdelivery..."
      (cd "$ld_root" && make liblogosdelivery)
      cp -f "$ld_root/build/liblogosdelivery.so" "$modules_user/delivery_module/"
      cp -f "$ld_root/build/liblogosdelivery.so" "$modules_provider/delivery_module/"
    fi
  fi
  
  ps_log_info "Build complete"
}

# ============================================================================
# Prepare command
# ============================================================================

cmd_prepare() {
  ps_log_info "Preparing environment..."
  
  # Validate scaffold
  "$REPO_ROOT/scripts/lifecycle.sh" scaffold check
  
  # Build if needed
  if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
    cmd_build
  else
    ps_log_info "SKIP_BUILD=1 — skipping build"
  fi
  
  # Prepare based on chain
  if ps_is_testnet; then
    cmd_prepare_testnet
  else
    cmd_prepare_local
  fi
}

cmd_prepare_local() {
  ps_log_info "Preparing localnet..."
  
  # Start localnet if needed
  if [[ "$($REPO_ROOT/scripts/lifecycle.sh localnet status)" != "running" ]]; then
    "$REPO_ROOT/scripts/lifecycle.sh" localnet start
  fi
  
  # Use funded snapshot or create it
  local snapshot_name="${SNAPSHOT_NAME:-funded}"
  if "$REPO_ROOT/scripts/lifecycle.sh" snapshot validate "$snapshot_name" 2>/dev/null; then
    ps_log_info "Restoring snapshot: $snapshot_name"
    "$REPO_ROOT/scripts/lifecycle.sh" snapshot restore "$snapshot_name"
  else
    ps_log_info "No valid snapshot, creating..."
    # Prefund
    "$REPO_ROOT/scripts/fixture.sh" prefund
    # Create snapshot
    "$REPO_ROOT/scripts/lifecycle.sh" snapshot save "$snapshot_name"
  fi
  
  # Ensure vault (creates baseline if needed)
  "$REPO_ROOT/scripts/fixture.sh" vault ensure 0
  
  ps_log_info "Local prepare complete"
}

cmd_prepare_testnet() {
  ps_log_info "Preparing testnet..."
  
  # Ensure testnet wallet
  "$REPO_ROOT/scripts/lifecycle.sh" testnet wallet-ensure
  
  # Read smoke check
  "$REPO_ROOT/scripts/lifecycle.sh" testnet read-smoke
  
  # Program deploy (one-time, usually done)
  if [[ "${TESTNET_DEPLOY:-0}" == "1" ]]; then
    ps_log_info "Deploying program to testnet..."
    # Would call deploy script here
  fi
  
  # Bootstrap fixture if needed
  if [[ ! -f "$REPO_ROOT/fixtures/testnet.json" ]]; then
    ps_log_info "Testnet fixture not found. Run bootstrap first:"
    ps_log_info "  make bootstrap-testnet"
  fi
  
  ps_log_info "Testnet prepare complete"
}

# ============================================================================
# Run command — Python orchestrator
# ============================================================================

cmd_run() {
  ps_log_info "Starting E2E run..."
  
  # Ensure N8 wire hex
  if [[ -z "${N8_WIRE_HEX:-}" ]]; then
    ps_log_info "Computing N8 wire..."
    export N8_WIRE_HEX
    N8_WIRE_HEX="$(cargo run -q -p lez-payment-streams-core --bin n8_canonical_wire_hex)"
  fi
  
  # Set up environment for Python
  export CHAIN="${CHAIN:-local}"
  export FIXTURE_MANIFEST
  export WALLET_CONFIG
  export WALLET_STORAGE
  export MODULES_USER="${MODULES_USER:-$REPO_ROOT/.scaffold/e2e/user/modules}"
  export MODULES_PROVIDER="${MODULES_PROVIDER:-$REPO_ROOT/.scaffold/e2e/provider/modules}"
  export LOGOSCORE_CONFIG_USER="${LOGOSCORE_CONFIG_USER:-$REPO_ROOT/.scaffold/e2e/user/logoscore}"
  export LOGOSCORE_CONFIG_PROVIDER="${LOGOSCORE_CONFIG_PROVIDER:-$REPO_ROOT/.scaffold/e2e/provider/logoscore}"
  export PERSIST_USER="${PERSIST_USER:-$REPO_ROOT/.scaffold/e2e/user/persist}"
  export PERSIST_PROVIDER="${PERSIST_PROVIDER:-$REPO_ROOT/.scaffold/e2e/provider/persist}"
  export E2E_PROVIDER_AD="${E2E_PROVIDER_AD:-$REPO_ROOT/.scaffold/e2e/provider-advertisement.json}"
  export PAYMENT_STREAMS_GUEST_BIN="${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
  export ARTIFACT="${ARTIFACT:-$REPO_ROOT/.scaffold/e2e/artifacts/e2e-$(date +%Y%m%dT%H%M%S).log}"
  export E2E_PHASE="${E2E_PHASE:-all}"
  export SKIP_BUILD="${SKIP_BUILD:-1}"  # Already built in prepare
  export SKIP_SEED="${SKIP_SEED:-0}"
  export RESTORE_LOCALNET="${RESTORE_LOCALNET:-1}"
  
  mkdir -p "$(dirname "$ARTIFACT")"
  
  # Run Python orchestrator
  ps_log_info "Launching Python orchestrator..."
  python3 "$REPO_ROOT/scripts/e2e/run_local_e2e.py" \
    --repo "$REPO_ROOT" \
    --phase "$E2E_PHASE" \
    --artifact "$ARTIFACT" || {
      ps_log_error "E2E run failed"
      return 1
    }
  
  ps_log_info "E2E complete. Artifact: $ARTIFACT"
  cat "$ARTIFACT"
}

# ============================================================================
# Teardown command
# ============================================================================

cmd_teardown() {
  ps_log_info "Running teardown..."
  
  # Stop logoscore daemons if running
  ps_log_info "Stopping logoscore daemons..."
  local user_cfg="${LOGOSCORE_CONFIG_USER:-$REPO_ROOT/.scaffold/e2e/user/logoscore}"
  local provider_cfg="${LOGOSCORE_CONFIG_PROVIDER:-$REPO_ROOT/.scaffold/e2e/provider/logoscore}"
  
  logoscore --config-dir "$user_cfg" stop 2>/dev/null || true
  logoscore --config-dir "$provider_cfg" stop 2>/dev/null || true
  
  # Stop localnet if this was the last consumer
  if ps_is_local && [[ "${STOP_LOCALNET:-0}" == "1" ]]; then
    ps_log_info "Stopping localnet..."
    "$REPO_ROOT/scripts/lifecycle.sh" localnet stop
  fi
  
  ps_log_info "Teardown complete"
}

# ============================================================================
# Full run command (prepare + run + teardown)
# ============================================================================

cmd_full_run() {
  local exit_code=0
  
  # Prepare
  cmd_prepare || { ps_log_error "Prepare failed"; return 1; }
  
  # Run
  cmd_run || { ps_log_error "Run failed"; exit_code=1; }
  
  # Teardown (always, even on failure)
  if [[ "${SKIP_TEARDOWN:-0}" != "1" ]]; then
    cmd_teardown || ps_log_error "Teardown had issues"
  fi
  
  return $exit_code
}

# ============================================================================
# Main dispatch
# ============================================================================

usage() {
  cat << EOF
Usage: $0 <local|testnet|build> <command> [args]

Categories:
  local run          — Full local E2E (prepare + run + teardown)
  local prepare      — Setup environment only
  local teardown     — Cleanup only
  testnet run        — Full testnet E2E
  testnet prepare    — Setup testnet environment
  build              — Build all modules

Environment:
  CHAIN              — local or testnet (default: local)
  SKIP_BUILD         — Skip module build (default: 0)
  SKIP_TEARDOWN      — Skip cleanup (default: 0)
  E2E_PHASE          — core, claim, or all (default: all)

Examples:
  $0 local run                    # Full local E2E
  $0 local prepare                # Just setup
  CHAIN=testnet $0 testnet run    # Testnet E2E
  $0 build                        # Build modules only
EOF
}

main() {
  [[ $# -lt 1 ]] && { usage; exit 1; }
  
  local category="$1"
  shift
  
  case "$category" in
    build)
      cmd_build "$@"
      ;;
    local|testnet)
      export CHAIN="$category"
      [[ $# -lt 1 ]] && { usage; exit 1; }
      local command="$1"
      shift
      
      case "$command" in
        run)       cmd_full_run "$@" ;;
        prepare)   cmd_prepare "$@" ;;
        teardown)  cmd_teardown "$@" ;;
        *)
          ps_log_error "Unknown command: $category $command"
          usage
          exit 1
          ;;
      esac
      ;;
    help|--help|-h)
      usage
      ;;
    *)
      ps_log_error "Unknown category: $category"
      usage
      exit 1
      ;;
  esac
}

main "$@"
