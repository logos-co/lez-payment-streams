#!/usr/bin/env bash
# Common functions for payment-streams E2E scripts
# Usage: source "$(dirname "$0")/lib/common.sh"

set -euo pipefail

# Guard against double-sourcing
[[ -n "${PS_COMMON_SOURCED:-}" ]] && return 0
PS_COMMON_SOURCED=1

# Determine repo root
ps_repo_root() {
  if [[ -n "${REPO_ROOT:-}" ]]; then
    echo "$REPO_ROOT"
  else
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
    echo "$script_dir"
  fi
}

# Export repo root for use by other scripts
export REPO_ROOT="${REPO_ROOT:-$(ps_repo_root)}"

# Logging
ps_log() {
  echo "[$(date +%Y-%m-%dT%H:%M:%S)] $*" >&2
}

ps_log_info() {
  ps_log "INFO: $*"
}

ps_log_error() {
  ps_log "ERROR: $*"
}

ps_log_phase() {
  local phase="$1" ok="$2"
  shift 2 || true
  local extra="${*:-{}}"
  echo "{\"phase\":\"$phase\",\"ok\":$ok,\"extra\":$extra}"
}

# JSON helpers
ps_json_get() {
  local file="$1" key="$2"
  python3 -c "import json; print(json.load(open('$file')).get('$key', ''))"
}

# Manifest helpers
ps_manifest_get() {
  local key="$1"
  local manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/fixtures/localnet.json}"
  if [[ -f "$manifest" ]]; then
    ps_json_get "$manifest" "$key"
  fi
}

ps_manifest_validate_exists() {
  local manifest="${1:-${FIXTURE_MANIFEST:-}}"
  if [[ -z "$manifest" ]] || [[ ! -f "$manifest" ]]; then
    ps_log_error "Fixture manifest not found: ${manifest:-(not set)}"
    return 1
  fi
  ps_log_info "Using manifest: $manifest"
}

# Environment detection
ps_is_testnet() {
  [[ "${CHAIN:-local}" == "testnet" ]]
}

ps_is_local() {
  [[ "${CHAIN:-local}" == "local" ]]
}

# Binary availability
ps_require_command() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    ps_log_error "Required command not found: $cmd"
    return 1
  fi
}

# Nix wrapper with common flags
ps_nix_build() {
  local flake_ref="$1"
  shift
  nix build "$flake_ref" -L --no-link --print-out-paths "$@" | tail -1
}

# Module installation
ps_install_lgx() {
  local lgx_path="$1"
  local dest_dir="$2"
  mkdir -p "$dest_dir"
  lgpm --modules-dir "$dest_dir" install --file "$lgx_path" --force
}

# Error handling
ps_fatal() {
  ps_log_error "$*"
  exit 1
}

ps_check_file() {
  local file="$1" msg="${2:-File not found}"
  if [[ ! -f "$file" ]]; then
    ps_fatal "$msg: $file"
  fi
}

# ============================================================================
# LEZ ledger / snapshot helpers (localnet)
# ============================================================================

# Pin of the LEZ checkout from scaffold.toml ([repos.lez] pin = "..."). The
# RocksDB ledger lives under the per-pin scaffold cache, not anywhere reported
# by `lgs localnet status`.
ps_lez_pin() {
  grep -A2 '\[repos.lez\]' "$REPO_ROOT/scaffold.toml" | grep '^pin' |
    sed 's/.*"\([^"]*\)".*/\1/'
}

ps_lez_cache() {
  echo "${HOME}/.cache/logos-scaffold/repos/lez/$(ps_lez_pin)"
}

ps_rocksdb_dir() {
  echo "$(ps_lez_cache)/rocksdb"
}

# ImageID hex of the currently built guest binary; empty if the build is missing.
ps_program_id_hex() {
  make -C "$REPO_ROOT" program-id 2>/dev/null |
    grep 'ImageID (hex bytes)' | awk '{print $NF}' || true
}

# LEZ 510+ nests the sequencer under lez/; older lgs builds expect it at the pin
# root. Link it so `lgs localnet start` finds the config after a restore.
ps_ensure_lez_layout() {
  local cache
  cache="$(ps_lez_cache)"
  if [[ ! -d "$cache" ]]; then
    ps_log_info "LEZ cache missing at $cache (run lgs setup)"
    return 0
  fi
  if [[ ! -e "$cache/sequencer" && -d "$cache/lez/sequencer" ]]; then
    ln -sfn lez/sequencer "$cache/sequencer"
    ps_log_info "linked $cache/sequencer -> lez/sequencer"
  fi
}

# Canonical public testnet sequencer endpoint (override with TESTNET_SEQUENCER_URL).
ps_testnet_seq_url() {
  echo "${TESTNET_SEQUENCER_URL:-https://testnet.lez.logos.co/}"
}

# Sequencer URL for the active CHAIN. This feeds the seed binary's
# --sequencer-url (which it stamps into any written manifest), so it MUST be the
# testnet endpoint on testnet — otherwise a manifest write clobbers sequencer_url
# with the localnet default and later reads hit 127.0.0.1.
ps_seq_url() {
  if [[ -n "${SEQUENCER_URL:-}" ]]; then
    echo "$SEQUENCER_URL"
  elif ps_is_testnet; then
    ps_testnet_seq_url
  else
    echo "http://127.0.0.1:3040"
  fi
}

ps_seq_reachable() {
  curl -sf -X POST "$(ps_seq_url)" -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' \
    >/dev/null 2>&1
}

# Block until the sequencer port stops answering (so RocksDB can be swapped).
ps_wait_port_free() {
  local i
  for i in $(seq 1 10); do
    ps_seq_reachable || return 0
    ps_log_info "waiting for sequencer port to free..."
    sleep 1
  done
  ps_log_error "sequencer still reachable; a foreign sequencer may be running"
  return 1
}

# Wait for Clock10 to track wall time before submitting transactions.
ps_wait_clock_synced() {
  local guest wallet_home
  guest="${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
  wallet_home="${LEE_WALLET_HOME_DIR:-$(ps_chain_wallet_home)}"
  LEE_WALLET_HOME_DIR="$wallet_home" cargo run -q \
    --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- wait-clock-synced >&2
}

# Read the on-chain next_stream_id for a vault; non-zero exit if the vault
# config account has no data (vault not initialized).
ps_vault_next_stream_id() {
  local owner="$1" vault_id="${2:-0}"
  local guest wallet_home
  guest="${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
  wallet_home="${LEE_WALLET_HOME_DIR:-$(ps_chain_wallet_home)}"
  LEE_WALLET_HOME_DIR="$wallet_home" cargo run -q \
    --manifest-path "$REPO_ROOT/examples/Cargo.toml" \
    --bin seed_localnet_fixture -- read-vault-next-stream-id \
    --program-bin "$guest" --owner "$owner" --vault-id "$vault_id" 2>/dev/null
}

# Balance of an account via the sequencer JSON-RPC (0 when absent).
ps_account_balance() {
  local acct="$1"
  curl -sf -X POST "$(ps_seq_url)" -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$acct\"]}" \
    2>/dev/null |
    python3 -c "import json,sys
try:
    print(int((json.load(sys.stdin).get('result') or {}).get('balance', 0) or 0))
except Exception:
    print(0)"
}

# True when the account is registered under authenticated_transfer (non-default owner).
ps_account_is_at_initialized() {
  local acct="$1"
  curl -sf -X POST "$(ps_seq_url)" -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$acct\"]}" \
    2>/dev/null |
    python3 -c "import json,sys
try:
    po=(json.load(sys.stdin).get('result') or {}).get('program_owner') or []
    sys.exit(0 if any(int(x)!=0 for x in po) else 1)
except Exception:
    sys.exit(1)"
}

# Default paths
ps_default_fixture_manifest() {
  if ps_is_testnet; then
    echo "$REPO_ROOT/fixtures/testnet.json"
  else
    echo "$REPO_ROOT/fixtures/localnet.json"
  fi
}

ps_default_wallet_config() {
  if ps_is_testnet; then
    echo "$REPO_ROOT/.scaffold/e2e/testnet-wallet/wallet_config.json"
  else
    echo "$REPO_ROOT/.scaffold/wallet/wallet_config.json"
  fi
}

ps_default_wallet_storage() {
  if ps_is_testnet; then
    echo "$REPO_ROOT/.scaffold/e2e/testnet-wallet/storage.json"
  else
    echo "$REPO_ROOT/.scaffold/wallet/storage.json"
  fi
}

# Wallet home directory (holds storage.json + wallet_config.json) the seed
# binary opens via LEE_WALLET_HOME_DIR. This selects which sequencer the seed
# CLI talks to, so it MUST follow CHAIN: testnet writes use the testnet wallet,
# not the localnet one.
ps_chain_wallet_home() {
  if ps_is_testnet; then
    echo "$REPO_ROOT/.scaffold/e2e/testnet-wallet"
  else
    echo "$REPO_ROOT/.scaffold/wallet"
  fi
}

# Export environment defaults
export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$(ps_default_fixture_manifest)}"
export WALLET_CONFIG="${WALLET_CONFIG:-$(ps_default_wallet_config)}"
export WALLET_STORAGE="${WALLET_STORAGE:-$(ps_default_wallet_storage)}"

ps_log_info "Common library loaded (REPO_ROOT=$REPO_ROOT)"
