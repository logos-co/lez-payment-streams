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

# Flow selector for the 2x2 verification matrix:
#   MODE=store  (default) — dual-host Store integration (Flow B, Python orchestrator)
#   MODE=module           — single-host payment-streams happy path (Flow A)
MODE="${MODE:-store}"

ps_is_module_mode() { [[ "$MODE" == "module" ]]; }

# ============================================================================
# Build command
# ============================================================================

cmd_build() {
  ps_log_info "Building modules..."
  
  local modules_user="${MODULES_USER:-$(ps_e2e_user_modules_dir)}"
  local modules_provider="${MODULES_PROVIDER:-$(ps_e2e_provider_modules_dir)}"
  
  mkdir -p "$modules_user" "$modules_provider"
  
  # Build payment_streams_module
  ps_log_info "Building payment_streams_module..."
  local ps_out
  ps_out="$(ps_nix_build "$REPO_ROOT/logos-payment-streams-module#lgx-portable")"
  ps_install_lgx "$ps_out"/*.lgx "$modules_user"
  ps_install_lgx "$ps_out"/*.lgx "$modules_provider"
  
  # Build wallet module (patched)
  ps_log_info "Building logos_execution_zone (wallet)..."
  local wallet_out="$REPO_ROOT/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out"
  # compgen -G expands the glob (unlike [[ -f ... ]], where it stays literal);
  # otherwise the wallet lgx would be re-bundled on every run.
  if ! compgen -G "$wallet_out/*.lgx" >/dev/null; then
    "$REPO_ROOT/scripts/archive/build-wallet-lgx.sh"
  fi
  local wallet_lgx
  wallet_lgx="$(readlink -f "$wallet_out/"*.lgx)"
  ps_install_lgx "$wallet_lgx" "$modules_user"
  ps_install_lgx "$wallet_lgx" "$modules_provider"

  # Flow A (module only) needs neither delivery_module nor the liblogosdelivery
  # overlay: it exercises payment_streams_module chainAction directly.
  if ps_is_module_mode; then
    ps_log_info "MODE=module — skipping delivery_module + liblogosdelivery overlay"
    ps_log_info "Build complete"
    return 0
  fi

  # Build delivery_module
  ps_log_info "Building delivery_module..."
  local dm_root="${DELIVERY_MODULE_ROOT:-$REPO_ROOT/../logos-delivery-module}"
  if [[ ! -d "$dm_root" ]]; then
    ps_fatal "DELIVERY_MODULE_ROOT not found: $dm_root"
  fi
  local dm_out
  dm_out="$(ps_nix_build "$dm_root#lgx-portable")"
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

  # Route seed-CLI wallet/sequencer to match CHAIN (testnet vs localnet). Set
  # unconditionally so a stale localnet value in the environment cannot redirect
  # testnet chain ops to the local sequencer.
  export LEE_WALLET_HOME_DIR="$(ps_chain_wallet_home)"

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

  if [[ "${E2E_PREPARE_DRY_RUN:-0}" == "1" ]]; then
    ps_log_info "E2E_PREPARE_DRY_RUN=1 — no prepare side effects"
    if ps_is_module_mode; then
      ps_log_info "module mode: would ensure localnet running only"
    else
      ps_log_info "store default: orchestrator ensures fresh vault per run (no vault ensure 0 here)"
      ps_log_info "E2E_REUSE_BASELINE_VAULT=1: would restore snapshot and vault ensure 0"
    fi
    return 0
  fi

  # Flow A drives its own vault lifecycle through the module (initializeVault,
  # deposit, createStream); it only needs localnet up. The Store-flow vault
  # snapshot/prefund baseline below does not apply.
  if ps_is_module_mode; then
    if [[ "$($REPO_ROOT/scripts/lifecycle.sh localnet status)" != "running" ]]; then
      "$REPO_ROOT/scripts/lifecycle.sh" localnet start
    fi
    ps_log_info "Local prepare complete (module mode)"
    return 0
  fi

  local snapshot_name="${SNAPSHOT_NAME:-funded}"

  # Continuation legs (back-to-back) must NOT reset the ledger: they continue on
  # the chain left by the previous leg with monotonic stream ids.
  if [[ "${SKIP_SEED:-0}" == "1" || "${RESTORE_LOCALNET:-1}" == "0" ]]; then
    ps_log_info "Continuation run — reusing live ledger (no restore/reseed)"
    if [[ "$($REPO_ROOT/scripts/lifecycle.sh localnet status)" != "running" ]]; then
      "$REPO_ROOT/scripts/lifecycle.sh" localnet start
    fi
    if [[ "${E2E_REUSE_BASELINE_VAULT:-0}" == "1" ]]; then
      "$REPO_ROOT/scripts/fixture.sh" vault ensure 0
      "$REPO_ROOT/scripts/fixture.sh" vault manifest 0
    else
      "$REPO_ROOT/scripts/fixture.sh" vault manifest 0
    fi
    ps_log_info "Local prepare complete (continuation)"
    return 0
  fi

  # Decide how to reach a consistent funded baseline. Prefer restoring a valid
  # snapshot (deterministic ledger + matching wallet nonces); otherwise reuse an
  # already-funded live ledger; only re-seed when nothing usable exists.
  if [[ "${FULL_RESET:-0}" == "1" ]]; then
    ps_log_info "FULL_RESET=1 — rebuilding funded baseline from scratch"
    "$REPO_ROOT/scripts/fixture.sh" prefund
    "$REPO_ROOT/scripts/lifecycle.sh" snapshot save "$snapshot_name"
  elif "$REPO_ROOT/scripts/lifecycle.sh" snapshot validate "$snapshot_name" 2>/dev/null; then
    ps_log_info "Valid snapshot found — restoring: $snapshot_name"
    "$REPO_ROOT/scripts/lifecycle.sh" snapshot restore "$snapshot_name"
  else
    ps_log_info "No valid snapshot for '$snapshot_name'"
    if [[ "$($REPO_ROOT/scripts/lifecycle.sh localnet status)" != "running" ]]; then
      "$REPO_ROOT/scripts/lifecycle.sh" localnet start
    fi
    if "$REPO_ROOT/scripts/fixture.sh" vault is-funded 0; then
      ps_log_info "Reusing existing funded vault on live ledger"
    else
      ps_log_info "Live ledger not funded — running prefund baseline"
      "$REPO_ROOT/scripts/fixture.sh" prefund
      "$REPO_ROOT/scripts/lifecycle.sh" snapshot save "$snapshot_name"
    fi
  fi

  # Localnet must be up before vault checks/seeding.
  if [[ "$($REPO_ROOT/scripts/lifecycle.sh localnet status)" != "running" ]]; then
    "$REPO_ROOT/scripts/lifecycle.sh" localnet start
  fi
  if [[ "$($REPO_ROOT/scripts/lifecycle.sh localnet status)" != "running" ]]; then
    ps_fatal "Localnet not running after prepare (refusing to continue)"
  fi

  if [[ "${E2E_REUSE_BASELINE_VAULT:-0}" == "1" ]]; then
    "$REPO_ROOT/scripts/fixture.sh" vault ensure 0
    "$REPO_ROOT/scripts/fixture.sh" vault manifest 0
  else
    # Identity + policy baseline; orchestrator ensures a fresh vault and rewrites PDAs per run.
    "$REPO_ROOT/scripts/fixture.sh" vault manifest 0
  fi

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

  # Bootstrap fixture if needed (module vs store paths)
  local fixture expected_bootstrap
  if ps_is_module_mode; then
    fixture="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet-module.json}"
    expected_bootstrap="make bootstrap-testnet-module"
  else
    fixture="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/testnet.json}"
    expected_bootstrap="make bootstrap-testnet"
  fi

  if [[ ! -f "$fixture" ]]; then
    ps_log_info "Testnet fixture not found: $fixture"
    ps_log_info "Run bootstrap first: $expected_bootstrap"
    ps_fatal "Missing fixture: $fixture"
  fi

  ps_log_info "Testnet prepare complete"
}

# ============================================================================
# Run command — Python orchestrator
# ============================================================================

cmd_run() {
  ps_log_info "Starting E2E run..."

  # Flow A (module only): single-host happy path, no Store / dual-host / N8.
  if ps_is_module_mode; then
    export LEE_WALLET_HOME_DIR="$(ps_chain_wallet_home)"
    export FIXTURE_MANIFEST="$(ps_default_fixture_manifest)"
    if ps_is_testnet && [[ -f "$REPO_ROOT/fixtures/testnet-module.json" ]]; then
      export FIXTURE_MANIFEST="$REPO_ROOT/fixtures/testnet-module.json"
    fi
    export WALLET_CONFIG="$(ps_default_wallet_config)"
    export WALLET_STORAGE="$(ps_default_wallet_storage)"
    export MODULES_USER="${MODULES_USER:-$(ps_e2e_user_modules_dir)}"
    export PAYMENT_STREAMS_GUEST_BIN="${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
    export ARTIFACT="${ARTIFACT:-$(ps_e2e_artifacts_dir)/module-e2e-$(date +%Y%m%dT%H%M%S).log}"
    mkdir -p "$(dirname "$ARTIFACT")"
    ps_log_info "Launching module happy path (Flow A)..."
    ps_normalize_privacy_flags
    if ps_is_any_privacy_e2e; then
      # Private submit proves in-process; stub receipts keep the module IPC
      # path inside the extended Timeout used by submitGenericPrivateViaFfi.
      export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
      ps_log_info "Privacy profile ($(ps_privacy_profile_label)) — RISC0_DEV_MODE=$RISC0_DEV_MODE; see PRIVACY_ENHANCED_JOURNEY.md"
    fi
    MODULES="$MODULES_USER" ARTIFACT="$ARTIFACT" \
      OWNER_PRIVACY="$OWNER_PRIVACY" PROVIDER_PRIVACY="$PROVIDER_PRIVACY" PRIVACY="$PRIVACY" \
      "$REPO_ROOT/scripts/module-e2e.sh" ${E2E_VERBOSITY:+--verbosity "$E2E_VERBOSITY"} || {
        ps_log_error "Module E2E run failed"
        return 1
      }
    ps_log_info "Module E2E complete. Artifact: $ARTIFACT"
    return 0
  fi

  # Ensure N8 wire hex
  if [[ -z "${N8_WIRE_HEX:-}" ]]; then
    ps_log_info "Computing N8 wire..."
    export N8_WIRE_HEX
    N8_WIRE_HEX="$(cargo run -q -p lez-payment-streams-core --bin n8_canonical_wire_hex)"
  fi
  
  # Set up environment for Python
  export CHAIN="${CHAIN:-local}"
  # Seed CLI + orchestrator wallet home must follow CHAIN; set unconditionally
  # so an inherited localnet value cannot redirect testnet ops to 127.0.0.1.
  export LEE_WALLET_HOME_DIR="$(ps_chain_wallet_home)"
  export FIXTURE_MANIFEST="$(ps_default_fixture_manifest)"
  if ps_is_testnet && ! ps_is_module_mode; then
    export SEED_ALLOCATION="${SEED_ALLOCATION:-400}"
    export SEED_DEPOSIT_AMOUNT="${SEED_DEPOSIT_AMOUNT:-500}"
    export E2E_CREATE_VIA="${E2E_CREATE_VIA:-chainaction}"
    export E2E_CLAIM_OPTIONAL="${E2E_CLAIM_OPTIONAL:-1}"
    ps_log_info "Store testnet sizing: SEED_ALLOCATION=$SEED_ALLOCATION SEED_DEPOSIT_AMOUNT=$SEED_DEPOSIT_AMOUNT (override via env; optional VAULT_ID)"
  fi
  export WALLET_CONFIG="$(ps_default_wallet_config)"
  export WALLET_STORAGE="$(ps_default_wallet_storage)"
  export MODULES_USER="${MODULES_USER:-$(ps_e2e_user_modules_dir)}"
  export MODULES_PROVIDER="${MODULES_PROVIDER:-$(ps_e2e_provider_modules_dir)}"
  export LOGOSCORE_CONFIG_USER="${LOGOSCORE_CONFIG_USER:-$(ps_e2e_user_logoscore_dir)}"
  export LOGOSCORE_CONFIG_PROVIDER="${LOGOSCORE_CONFIG_PROVIDER:-$(ps_e2e_provider_logoscore_dir)}"
  export PERSIST_USER="${PERSIST_USER:-$(ps_e2e_user_persist_dir)}"
  export PERSIST_PROVIDER="${PERSIST_PROVIDER:-$(ps_e2e_provider_persist_dir)}"
  export E2E_PROVIDER_AD="${E2E_PROVIDER_AD:-$(ps_e2e_provider_ad_path)}"
  export PAYMENT_STREAMS_GUEST_BIN="${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
  export ARTIFACT="${ARTIFACT:-$(ps_e2e_artifacts_dir)/e2e-$(date +%Y%m%dT%H%M%S).log}"
  export E2E_PHASE="${E2E_PHASE:-all}"
  export SKIP_BUILD="${SKIP_BUILD:-1}"  # Already built in prepare
  export SKIP_SEED="${SKIP_SEED:-0}"
  export RESTORE_LOCALNET="${RESTORE_LOCALNET:-1}"
  
  mkdir -p "$(dirname "$ARTIFACT")"

  ps_normalize_privacy_flags
  if ps_is_any_privacy_e2e; then
    # Private submit / NSK vault proof prove in-process; stub receipts keep IPC
    # inside module timeouts (same as module privacy E2E).
    export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
    ps_log_info "Privacy profile ($(ps_privacy_profile_label)) — RISC0_DEV_MODE=$RISC0_DEV_MODE"
  fi
  
  # Run Python orchestrator
  ps_log_info "Launching Python orchestrator..."
  OWNER_PRIVACY="${OWNER_PRIVACY:-0}" PROVIDER_PRIVACY="${PROVIDER_PRIVACY:-0}" PRIVACY="${PRIVACY:-0}" \
  RISC0_DEV_MODE="${RISC0_DEV_MODE:-}" \
  python3 "$REPO_ROOT/scripts/e2e/run_local_e2e.py" \
    --repo "$REPO_ROOT" \
    --phase "$E2E_PHASE" \
    --artifact "$ARTIFACT" \
    ${E2E_VERBOSITY:+--verbosity "$E2E_VERBOSITY"} || {
      ps_log_error "E2E run failed"
      return 1
    }
  
  ps_log_info "E2E complete. Artifact: $ARTIFACT"
}

# ============================================================================
# Teardown command
# ============================================================================

cmd_teardown() {
  ps_log_info "Running teardown..."
  
  # Stop logoscore daemons if running
  ps_log_info "Stopping logoscore daemons..."
  local user_cfg="${LOGOSCORE_CONFIG_USER:-$(ps_e2e_user_logoscore_dir)}"
  local provider_cfg="${LOGOSCORE_CONFIG_PROVIDER:-$(ps_e2e_provider_logoscore_dir)}"
  
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
  MODE               — store (dual-host Store, Flow B) or module
                       (single-host payment-streams happy path, Flow A);
                       default: store.
  SKIP_BUILD         — Skip module build (default: 0)
  SKIP_TEARDOWN      — Skip cleanup (default: 0)
  E2E_PHASE          — core, claim, or all (default: all)
  E2E_REUSE_BASELINE_VAULT — Store: reuse vault 0 snapshot path (lifecycle)
  VAULT_ID           — Store: optional fixed vault id (else scan for empty config)
  E2E_PREPARE_DRY_RUN — prepare prints intent only (no side effects)

Flags:
  --verbosity quiet|normal|verbose
                     — Console output level (default: verbose on TTY,
                       quiet when piped). quiet: JSON-lines only;
                       normal: phase headers + values; verbose: full
                       narrative with concept explanations.

Verification matrix (mode x chain):
  MODE=module CHAIN=local  $0 local run   # module verification, localnet
  MODE=module CHAIN=local OWNER_PRIVACY=1 $0 local run   # Step 36 PseudonymousFunder (PRIVACY=1 alias OK)
  MODE=module CHAIN=local PROVIDER_PRIVACY=1 $0 local run  # Step 37 private provider claim
  MODE=store  CHAIN=local  $0 local run   # Store integration, localnet
  MODE=store  CHAIN=testnet $0 testnet run # Store integration, testnet
  MODE=module CHAIN=testnet                # module verification, testnet

Examples:
  $0 local run                    # Full local E2E (Store)
  $0 local prepare                # Just setup
  MODE=module $0 local run        # Module-only happy path
  CHAIN=testnet $0 testnet run    # Testnet E2E
  $0 build                        # Build modules only
EOF
}

main() {
  [[ $# -lt 1 ]] && { usage; exit 1; }

  # Parse --verbosity flag anywhere in args
  local verbosity=""
  local filtered_args=()
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --verbosity) verbosity="$2"; shift 2 ;;
      --verbosity=*) verbosity="${1#*=}"; shift ;;
      *) filtered_args+=("$1"); shift ;;
    esac
  done
  set -- "${filtered_args[@]}"

  if [[ -n "$verbosity" ]]; then
    case "$verbosity" in
      quiet|normal|verbose) export E2E_VERBOSITY="$verbosity" ;;
      *) ps_fatal "invalid --verbosity: $verbosity (use quiet|normal|verbose)" ;;
    esac
  fi

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
