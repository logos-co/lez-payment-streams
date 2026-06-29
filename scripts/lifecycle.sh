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

cmd_snapshot_save() {
  local name="${1:-funded}"
  local snap_dir="$SNAPSHOT_DIR/$name"
  
  ps_log_info "Creating snapshot: $name"
  mkdir -p "$snap_dir"
  
  # Copy RocksDB
  local lez_dir
  lez_dir="$(lgs localnet status 2>/dev/null | grep -oE '/[^ ]+rocksdb[^ ]*' | head -1 || true)"
  if [[ -d "$lez_dir" ]]; then
    cp -r "$lez_dir" "$snap_dir/rocksdb"
  fi
  
  # Copy wallet and state
  [[ -d "$REPO_ROOT/.scaffold/wallet" ]] && cp -r "$REPO_ROOT/.scaffold/wallet" "$snap_dir/"
  [[ -d "$REPO_ROOT/.scaffold/state" ]] && cp -r "$REPO_ROOT/.scaffold/state" "$snap_dir/"
  
  # Save metadata
  cat > "$snap_dir/snapshot.json" << EOF
{
  "name": "$name",
  "created": "$(date -Iseconds)",
  "lez_pin": "$(cat "$REPO_ROOT/scaffold.toml" | grep -A1 '\[repos.lez\]' | grep rev | cut -d'"' -f2)"
}
EOF
  
  ps_log_info "Snapshot saved: $snap_dir"
}

cmd_snapshot_restore() {
  local name="${1:-funded}"
  local snap_dir="$SNAPSHOT_DIR/$name"
  
  if [[ ! -d "$snap_dir" ]]; then
    ps_fatal "Snapshot not found: $snap_dir"
  fi
  
  ps_log_info "Restoring snapshot: $name"
  
  # Restore RocksDB
  local lez_dir
  lez_dir="$(lgs localnet status 2>/dev/null | grep -oE '/[^ ]+rocksdb[^ ]*' | head -1 || true)"
  if [[ -d "$lez_dir" && -d "$snap_dir/rocksdb" ]]; then
    rm -rf "$lez_dir"
    cp -r "$snap_dir/rocksdb" "$lez_dir"
  fi
  
  # Restore wallet and state
  [[ -d "$snap_dir/wallet" ]] && cp -r "$snap_dir/wallet" "$REPO_ROOT/.scaffold/"
  [[ -d "$snap_dir/state" ]] && cp -r "$snap_dir/state" "$REPO_ROOT/.scaffold/"
  
  ps_log_info "Snapshot restored"
}

cmd_snapshot_validate() {
  local name="${1:-funded}"
  local snap_dir="$SNAPSHOT_DIR/$name"
  
  if [[ ! -f "$snap_dir/snapshot.json" ]]; then
    ps_log_error "No metadata in snapshot: $name"
    return 1
  fi
  
  local snap_pin current_pin
  snap_pin="$(ps_json_get "$snap_dir/snapshot.json" lez_pin 2>/dev/null || echo unknown)"
  current_pin="$(grep -A1 '\[repos.lez\]' "$REPO_ROOT/scaffold.toml" | grep rev | cut -d'"' -f2 || echo unknown)"
  
  if [[ "$snap_pin" != "$current_pin" ]]; then
    ps_log_error "Snapshot LEZ pin mismatch: snapshot=$snap_pin, current=$current_pin"
    return 1
  fi
  
  ps_log_info "Snapshot valid: $name"
}

# ============================================================================
# Testnet wallet commands
# ============================================================================

TESTNET_WALLET_DIR="$REPO_ROOT/.scaffold/e2e/testnet-wallet"

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
  
  local wallet_home="${TESTNET_WALLET_DIR:-$REPO_ROOT/.scaffold/e2e/testnet-wallet}"
  
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
