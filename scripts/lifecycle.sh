#!/usr/bin/env bash
# lifecycle.sh — Environment lifecycle management (localnet, testnet wallet, snapshots)
# Usage: ./scripts/lifecycle.sh <localnet|testnet|snapshot> <command> [args]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"

# ============================================================================
# Localnet commands
# ============================================================================

cmd_localnet_start() {
  ps_log_info "Starting localnet..."
  ps_require_command lgs
  lgs localnet start
  ps_log_info "Localnet started"
}

cmd_localnet_stop() {
  ps_log_info "Stopping localnet..."
  ps_require_command lgs
  lgs localnet stop || true
  ps_log_info "Localnet stopped"
}

cmd_localnet_status() {
  ps_require_command lgs
  if lgs localnet status 2>/dev/null | grep -q running; then
    echo "running"
  else
    echo "stopped"
  fi
}

# ============================================================================
# Snapshot commands (localnet only)
# ============================================================================

SNAPSHOT_DIR="$REPO_ROOT/.scaffold/snapshots"

# The RocksDB ledger lives under the per-pin scaffold cache. A consistent copy
# requires the sequencer to be stopped and its RocksDB LOCK released.
snapshot_write_metadata() {
  local snap_dir="$1"
  local name pin prog owner provider
  name="$(basename "$snap_dir")"
  pin="$(ps_lez_pin)"
  prog="$(ps_program_id_hex)"
  owner=""
  if [[ -f "$REPO_ROOT/.lez_payment_streams-state" ]]; then
    owner="$(grep '^SIGNER_ID=' "$REPO_ROOT/.lez_payment_streams-state" | cut -d= -f2 | tr -d '"'\''')"
  fi
  provider=""
  [[ -f "$REPO_ROOT/.lez_payment_streams-fixture-provider" ]] &&
    provider="$(cat "$REPO_ROOT/.lez_payment_streams-fixture-provider")"
  cat > "$snap_dir/snapshot.json" << EOF
{
  "schema_version": 1,
  "name": "$name",
  "created": "$(date -Iseconds)",
  "lez_pin": "$pin",
  "program_id_hex": "$prog",
  "owner_account_id": "$owner",
  "provider_account_id": "$provider",
  "vault_id": 0,
  "deposit_amount": ${SEED_DEPOSIT_AMOUNT:-1000}
}
EOF
}

cmd_snapshot_save() {
  local name="${1:-funded}"
  local snap_dir="$SNAPSHOT_DIR/$name"
  local rocksdb restart i
  rocksdb="$(ps_rocksdb_dir)"
  restart="${SNAPSHOT_RESTART:-1}"

  ps_log_info "Creating snapshot: $name (rocksdb=$rocksdb)"

  if [[ ! -d "$rocksdb" ]]; then
    ps_fatal "No ledger at $rocksdb (run prefund/seed first)"
  fi

  # Sequencer must be stopped to copy a consistent RocksDB.
  if [[ "$(cmd_localnet_status)" == "running" ]]; then
    ps_log_info "Stopping localnet before copying RocksDB..."
    cmd_localnet_stop
  fi
  ps_wait_port_free || true
  # The port is free, so the sequencer has released its flock; a leftover LOCK
  # file is stale. Give it a brief grace period then proceed.
  for i in $(seq 1 5); do
    [[ -f "$rocksdb/LOCK" ]] || break
    sleep 1
  done

  rm -rf "$snap_dir"
  mkdir -p "$snap_dir"
  cp -a "$rocksdb" "$snap_dir/rocksdb"
  # Do not carry a stale RocksDB LOCK into the snapshot.
  rm -f "$snap_dir/rocksdb/LOCK"
  [[ -d "$REPO_ROOT/.scaffold/wallet" ]] && cp -a "$REPO_ROOT/.scaffold/wallet" "$snap_dir/wallet"
  [[ -d "$REPO_ROOT/.scaffold/state" ]] && cp -a "$REPO_ROOT/.scaffold/state" "$snap_dir/state"
  local f
  for f in .lez_payment_streams-state .lez_payment_streams-fixture-provider; do
    [[ -f "$REPO_ROOT/$f" ]] && cp -a "$REPO_ROOT/$f" "$snap_dir/"
  done

  snapshot_write_metadata "$snap_dir"
  ps_log_info "Snapshot saved: $snap_dir"

  if [[ "$restart" != "0" ]]; then
    ps_ensure_lez_layout
    cmd_localnet_start
  fi
}

cmd_snapshot_restore() {
  local name="${1:-funded}"
  local snap_dir="$SNAPSHOT_DIR/$name"
  local rocksdb
  rocksdb="$(ps_rocksdb_dir)"

  [[ -d "$snap_dir" ]] || ps_fatal "Snapshot not found: $snap_dir"
  [[ -d "$snap_dir/rocksdb" ]] || ps_fatal "Snapshot missing rocksdb/: $snap_dir"

  ps_log_info "Restoring snapshot: $name (rocksdb=$rocksdb)"

  # Swap RocksDB while the sequencer is stopped, then restart on the restored
  # ledger. The wallet is restored from the same snapshot so nonces match.
  if [[ "$(cmd_localnet_status)" == "running" ]]; then
    ps_log_info "Stopping localnet before restore..."
    cmd_localnet_stop
  fi
  ps_wait_port_free || ps_fatal "sequencer port still busy; refusing to swap ledger"

  rm -rf "$rocksdb"
  mkdir -p "$(dirname "$rocksdb")"
  cp -a "$snap_dir/rocksdb" "$rocksdb"

  if [[ -d "$snap_dir/wallet" ]]; then
    rm -rf "$REPO_ROOT/.scaffold/wallet"
    cp -a "$snap_dir/wallet" "$REPO_ROOT/.scaffold/wallet"
  fi
  if [[ -d "$snap_dir/state" ]]; then
    rm -rf "$REPO_ROOT/.scaffold/state"
    cp -a "$snap_dir/state" "$REPO_ROOT/.scaffold/state"
  fi
  local f
  for f in .lez_payment_streams-state .lez_payment_streams-fixture-provider; do
    [[ -f "$snap_dir/$f" ]] && cp -a "$snap_dir/$f" "$REPO_ROOT/$f"
  done

  # Per-run stream fixture is regenerated by the orchestrator from chain
  # next_stream_id; drop any stale manifest from a prior run.
  rm -f "$REPO_ROOT/fixtures/localnet.json"

  ps_ensure_lez_layout
  cmd_localnet_start
  ps_wait_clock_synced || ps_log_info "clock sync wait returned non-zero (continuing)"
  ps_log_info "Snapshot restored"
}

cmd_snapshot_validate() {
  local name="${1:-funded}"
  local snap_dir="$SNAPSHOT_DIR/$name"
  local meta="$snap_dir/snapshot.json"

  if [[ ! -f "$meta" ]]; then
    ps_log_error "No metadata in snapshot: $name"
    return 1
  fi
  if [[ ! -d "$snap_dir/rocksdb" ]]; then
    ps_log_error "Snapshot missing rocksdb/: $name"
    return 1
  fi

  local snap_pin snap_prog cur_pin cur_prog
  snap_pin="$(ps_json_get "$meta" lez_pin)"
  snap_prog="$(ps_json_get "$meta" program_id_hex)"
  cur_pin="$(ps_lez_pin)"
  cur_prog="$(ps_program_id_hex)"

  if [[ -z "$snap_pin" || "$snap_pin" != "$cur_pin" ]]; then
    ps_log_error "Snapshot LEZ pin mismatch (snapshot='$snap_pin' current='$cur_pin')"
    return 1
  fi
  if [[ -z "$cur_prog" ]]; then
    ps_log_error "Could not read current program id (make build && make program-id)"
    return 1
  fi
  if [[ -n "$snap_prog" && "$snap_prog" != "$cur_prog" ]]; then
    ps_log_error "Snapshot program_id mismatch (snapshot='$snap_prog' current='$cur_prog')"
    return 1
  fi

  ps_log_info "Snapshot valid: $name"
}

# ============================================================================
# Testnet wallet commands
# ============================================================================

TESTNET_WALLET_DIR="$(ps_e2e_testnet_wallet_dir)"

cmd_testnet_wallet_ensure() {
  ps_log_info "Ensuring testnet wallet..."
  
  mkdir -p "$TESTNET_WALLET_DIR"
  
  if [[ ! -f "$TESTNET_WALLET_DIR/wallet_config.json" ]]; then
    cat > "$TESTNET_WALLET_DIR/wallet_config.json" << 'EOF'
{
  "sequencer_addr": "https://testnet.lez.logos.co/"
}
EOF
    ps_log_info "Created testnet wallet config"
  fi
  
  if [[ ! -f "$TESTNET_WALLET_DIR/storage.json" ]]; then
    ps_log_info "WARNING: No wallet storage found. Run wallet setup first."
  fi
  
  export TESTNET_WALLET_DIR
  ps_log_info "Testnet wallet ready at: $TESTNET_WALLET_DIR"
}

cmd_testnet_read_smoke() {
  ps_log_info "Running testnet read smoke..."
  
  local wallet_home="${TESTNET_WALLET_DIR:-$(ps_e2e_testnet_wallet_dir)}"
  
  # Check sequencer accessible
  if ! curl -sf "https://testnet.lez.logos.co/" -X POST \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"getLastBlockId","params":[],"id":1}' \
    >/dev/null 2>&1; then
    ps_fatal "Testnet sequencer not accessible"
  fi
  
  ps_log_info "Testnet read smoke passed"
}

# ============================================================================
# Scaffold verification
# ============================================================================

cmd_scaffold_check() {
  ps_log_info "Checking scaffold layout..."
  
  if [[ ! -f "$REPO_ROOT/scaffold.toml" ]]; then
    ps_fatal "scaffold.toml not found. Run 'lgs setup' first."
  fi
  
  if [[ ! -d "$REPO_ROOT/.scaffold" ]]; then
    ps_fatal ".scaffold directory not found. Run 'lgs init' first."
  fi
  
  ps_log_info "Scaffold layout OK"
}

# ============================================================================
# Main dispatch
# ============================================================================

usage() {
  cat << EOF
Usage: $0 <category> <command> [args]

Categories:
  localnet <start|stop|status>
  snapshot <save|restore|validate> [name]
  testnet <wallet-ensure|read-smoke>
  scaffold <check>

Examples:
  $0 localnet start
  $0 snapshot save funded
  $0 snapshot restore funded
  $0 testnet wallet-ensure
  $0 scaffold check
EOF
}

main() {
  [[ $# -lt 2 ]] && { usage; exit 1; }
  
  local category="$1" command="$2"
  shift 2
  
  case "$category:$command" in
    localnet:start)      cmd_localnet_start "$@" ;;
    localnet:stop)       cmd_localnet_stop "$@" ;;
    localnet:status)     cmd_localnet_status "$@" ;;
    snapshot:save)       cmd_snapshot_save "$@" ;;
    snapshot:restore)    cmd_snapshot_restore "$@" ;;
    snapshot:validate)   cmd_snapshot_validate "$@" ;;
    testnet:wallet-ensure)  cmd_testnet_wallet_ensure "$@" ;;
    testnet:read-smoke)     cmd_testnet_read_smoke "$@" ;;
    scaffold:check)      cmd_scaffold_check "$@" ;;
    *)
      ps_log_error "Unknown command: $category $command"
      usage
      exit 1
      ;;
  esac
}

main "$@"
