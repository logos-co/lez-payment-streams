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

# Export environment defaults
export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$(ps_default_fixture_manifest)}"
export WALLET_CONFIG="${WALLET_CONFIG:-$(ps_default_wallet_config)}"
export WALLET_STORAGE="${WALLET_STORAGE:-$(ps_default_wallet_storage)}"

ps_log_info "Common library loaded (REPO_ROOT=$REPO_ROOT)"
