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
  local manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/verify/fixtures/localnet.json}"
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
  local out
  if ! out="$(nix build "$flake_ref" -L --no-link --print-out-paths "$@" | tail -1)"; then
    ps_fatal "nix build failed: $flake_ref"
  fi
  if [[ -z "$out" || ! -e "$out" ]]; then
    ps_fatal "nix build produced no output path: $flake_ref"
  fi
  printf '%s\n' "$out"
}

ps_first_lgx() {
  local dir="$1"
  local match
  match="$(compgen -G "$dir"/*.lgx | head -1 || true)"
  if [[ -z "$match" || ! -f "$match" ]]; then
    ps_fatal "no .lgx package in $dir"
  fi
  printf '%s\n' "$match"
}

# Module installation
ps_install_lgx() {
  local lgx_path="$1"
  local dest_dir="$2"
  if [[ ! -f "$lgx_path" ]]; then
    ps_fatal "lgx package missing: $lgx_path"
  fi
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

ps_default_guest_bin() {
  echo "$REPO_ROOT/program/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
}

# spel inspect ImageID (hex bytes) of a guest ELF; empty if inspect fails.
ps_guest_bin_image_id_hex() {
  local bin="${1:-}"
  [[ -n "$bin" && -f "$bin" ]] || return 1
  command -v spel >/dev/null 2>&1 || return 1
  spel inspect "$bin" 2>/dev/null | grep 'ImageID (hex bytes)' | awk '{print $NF}'
}

ps_program_bins_dir() {
  echo "$REPO_ROOT/.scaffold/program-bins"
}

ps_pinned_guest_bin_path() {
  local id_hex="${1:-}"
  local id8="${id_hex:0:8}"
  echo "$(ps_program_bins_dir)/lez_payment_streams-${id8}.bin"
}

ps_pin_guest_bin() {
  local src="${1:-}"
  local id_hex="${2:-}"
  local dest
  [[ -f "$src" && -n "$id_hex" ]] || return 1
  dest="$(ps_pinned_guest_bin_path "$id_hex")"
  mkdir -p "$(dirname "$dest")"
  cp -f "$src" "$dest"
  printf '%s\n' "$dest"
}

# Copy src ELF to .scaffold/program-bins/ after confirming spel inspect matches expect_id.
ps_pin_deployed_guest_or_die() {
  local src="${1:-}"
  local expect_id="${2:-}"
  local actual dest
  [[ -f "$src" ]] || ps_fatal "guest ELF missing: $src"
  actual="$(ps_guest_bin_image_id_hex "$src")" || ps_fatal "spel inspect failed: $src"
  if [[ -n "$expect_id" && "$actual" != "$expect_id" ]]; then
    ps_fatal "ELF ImageID $actual != expected $expect_id (rebuild-vs-deploy). Use the ELF that matches the deployed program, then re-run bootstrap/deploy to pin it under .scaffold/program-bins/."
  fi
  dest="$(ps_pin_guest_bin "$src" "$actual")"
  ps_log_info "Pinned guest ELF ${actual:0:8}… -> $dest"
}

# Honor FIXTURE_MANIFEST only when the basename is testnet*.json (module-e2e.sh rule).
ps_resolve_testnet_fixture() {
  if [[ -n "${FIXTURE_MANIFEST:-}" && "${FIXTURE_MANIFEST##*/}" == testnet*.json && -f "$FIXTURE_MANIFEST" ]]; then
    printf '%s\n' "$FIXTURE_MANIFEST"
    return 0
  fi
  if [[ "${MODE:-store}" == "module" && -f "$REPO_ROOT/verify/fixtures/testnet-module.json" ]]; then
    echo "$REPO_ROOT/verify/fixtures/testnet-module.json"
    return 0
  fi
  echo "$REPO_ROOT/verify/fixtures/testnet.json"
}

# PAYMENT_STREAMS_PROGRAM_ID_HEX and PAYMENT_STREAMS_GUEST_BIN must not leak
# across local/testnet cells.
ps_reset_chain_scoped_env() {
  if ps_is_testnet; then
    if [[ -n "${FIXTURE_MANIFEST:-}" && "${FIXTURE_MANIFEST##*/}" != testnet*.json ]]; then
      ps_log_info "Ignoring FIXTURE_MANIFEST=$FIXTURE_MANIFEST (basename is not testnet*.json)"
      unset FIXTURE_MANIFEST
    fi
  else
    unset PAYMENT_STREAMS_PROGRAM_ID_HEX
    if [[ -n "${FIXTURE_MANIFEST:-}" && "${FIXTURE_MANIFEST##*/}" == testnet*.json ]]; then
      ps_log_info "Ignoring FIXTURE_MANIFEST=$FIXTURE_MANIFEST on local (testnet fixture)"
      unset FIXTURE_MANIFEST
    fi
  fi
}

# Testnet: fixture ImageID always; pinned ELF when present (overrides inherited
# PAYMENT_STREAMS_GUEST_BIN). Privacy cells that prove with the ELF fail fast on
# ImageID skew. Public cells may run with a rebuilt ELF on disk as --program-bin
# while PDAs use PAYMENT_STREAMS_PROGRAM_ID_HEX.
# Local: live ELF, no PAYMENT_STREAMS_PROGRAM_ID_HEX.
ps_export_chain_guest_identity() {
  ps_normalize_privacy_flags
  if ps_is_testnet; then
    local fixture fixture_pid pinned live live_id guest_id
    fixture="$(ps_resolve_testnet_fixture)"
    [[ -f "$fixture" ]] || ps_fatal "Testnet fixture not found: $fixture"
    fixture_pid="$(ps_json_get "$fixture" program_id_hex)"
    [[ -n "$fixture_pid" ]] || ps_fatal "fixture missing program_id_hex: $fixture"
    export PAYMENT_STREAMS_PROGRAM_ID_HEX="$fixture_pid"
    live="$(ps_default_guest_bin)"
    pinned="$(ps_pinned_guest_bin_path "$fixture_pid")"
    if [[ ! -f "$pinned" && -f "$live" ]]; then
      live_id="$(ps_guest_bin_image_id_hex "$live" || true)"
      if [[ "$live_id" == "$fixture_pid" ]]; then
        pinned="$(ps_pin_guest_bin "$live" "$fixture_pid")"
      fi
    fi
    if [[ -f "$pinned" ]]; then
      export PAYMENT_STREAMS_GUEST_BIN="$pinned"
      ps_log_info "PAYMENT_STREAMS_GUEST_BIN=$PAYMENT_STREAMS_GUEST_BIN (pinned)"
    else
      # Drop inherited GUEST_BIN so a poisoned shell cannot rewrite identity.
      export PAYMENT_STREAMS_GUEST_BIN="$live"
      if ps_is_any_privacy_e2e; then
        ps_fatal "No ELF matching fixture program_id_hex=$fixture_pid. ImageID skew is a fail-fast (not a timeout). Copy the deployed binary to $pinned (make bootstrap-testnet pins it) or rebuild the guest that spel inspect reports as $fixture_pid."
      fi
      ps_log_info "No pinned ELF for ${fixture_pid:0:8}…; public cell using live GUEST_BIN for --program-bin, identity from PAYMENT_STREAMS_PROGRAM_ID_HEX"
    fi
    ps_log_info "PAYMENT_STREAMS_PROGRAM_ID_HEX=$PAYMENT_STREAMS_PROGRAM_ID_HEX (from $fixture)"
    if ps_is_any_privacy_e2e; then
      guest_id="$(ps_guest_bin_image_id_hex "$PAYMENT_STREAMS_GUEST_BIN" || true)"
      if [[ "$guest_id" != "$fixture_pid" ]]; then
        ps_fatal "GUEST_BIN ImageID ${guest_id:-missing} != fixture $fixture_pid. Privacy legs prove with the ELF; rebuild-vs-deploy skew is fatal in seconds. Pin the deployed ELF via make bootstrap-testnet."
      fi
    fi
  else
    unset PAYMENT_STREAMS_PROGRAM_ID_HEX
    export PAYMENT_STREAMS_GUEST_BIN="$(ps_default_guest_bin)"
  fi
}

# Sequencer getAccount.program_owner as ImageID hex (empty/nonzero-exit if missing).
ps_account_program_owner_hex() {
  local account_id="$1"
  local url="${2:-$(ps_seq_url)}"
  python3 -c '
import json, struct, sys, urllib.request
url, aid = sys.argv[1], sys.argv[2]
body = json.dumps({"jsonrpc":"2.0","id":1,"method":"getAccount","params":{"accountId":aid}}).encode()
req = urllib.request.Request(url, data=body, headers={"content-type":"application/json"})
acc = json.load(urllib.request.urlopen(req, timeout=15)).get("result") or {}
owner = acc.get("program_owner") or []
data = acc.get("data") or []
if not data or not owner or owner == [0] * 8:
    raise SystemExit(1)
print("".join(struct.pack("<I", int(x) & 0xFFFFFFFF).hex() for x in owner))
' "$url" "$account_id"
}

# Deployed payment-streams ImageID from the vault_config account on the live sequencer.
ps_deployed_program_id_hex() {
  local vault_cfg="${1:-}"
  local manifest
  if [[ -z "$vault_cfg" ]]; then
    manifest="${FIXTURE_MANIFEST:-$REPO_ROOT/verify/fixtures/localnet.json}"
    if [[ -f "$manifest" ]]; then
      vault_cfg="$(ps_json_get "$manifest" vault_config_account_id)"
    fi
  fi
  [[ -n "$vault_cfg" ]] || return 1
  ps_account_program_owner_hex "$vault_cfg"
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
  guest="${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/program/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
  wallet_home="${LEE_WALLET_HOME_DIR:-$(ps_chain_wallet_home)}"
  LEE_WALLET_HOME_DIR="$wallet_home" cargo run -q \
    --manifest-path "$REPO_ROOT/verify/seed/Cargo.toml" \
    --bin seed_localnet_fixture -- wait-clock-synced >&2
}

# CLOCK_50 base58 (LEZ genesis `/LEZ/ClockProgramAccount/0000050`).
PS_CLOCK50_ACCOUNT_ID="${PS_CLOCK50_ACCOUNT_ID:-4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWkX}"

# Wait until CLOCK_50 is early in its 50-block update window so a multi-minute
# real PPE prove is unlikely to cross a clock public_pre_state change (D39.25).
# Uses payment_streams_module readClockDecoded. Nudges via wallet sync only.
ps_wait_clock50_prove_window() {
  command -v logoscore >/dev/null 2>&1 || return 1
  local attempt=0 max_attempts="${PS_CLOCK50_WINDOW_ATTEMPTS:-90}" rem block_id line
  while (( attempt < max_attempts )); do
    attempt=$((attempt + 1))
    sync_wallet 2>/dev/null || true
    line="$(logoscore call payment_streams_module readClockDecoded "$PS_CLOCK50_ACCOUNT_ID" 2>/dev/null | tail -1)"
    block_id="$(python3 -c '
import json,sys
try:
  outer=json.loads(sys.argv[1])
  inner=outer.get("result","{}")
  if isinstance(inner,str):
    inner=json.loads(inner) if inner.strip().startswith("{") else {}
  dec=inner.get("decoded") if isinstance(inner.get("decoded"), dict) else inner
  for k in ("block_id","blockId"):
    if k in dec and str(dec[k]).replace("-","").isdigit():
      print(int(dec[k])); raise SystemExit
except Exception:
  pass
' "$line" 2>/dev/null || true)"
    if [[ -n "${block_id:-}" ]]; then
      rem=$((block_id % 50))
      # Early window: rem<=2 leaves ~48 blocks (~12 min at 15s) for PPE prove.
      if (( rem <= 2 )); then
        # Confirm once more after a fresh sync so we do not trust a stale read.
        sync_wallet 2>/dev/null || true
        sleep 1
        line="$(logoscore call payment_streams_module readClockDecoded "$PS_CLOCK50_ACCOUNT_ID" 2>/dev/null | tail -1)"
        block_id="$(python3 -c '
import json,sys
try:
  outer=json.loads(sys.argv[1])
  inner=outer.get("result","{}")
  if isinstance(inner,str):
    inner=json.loads(inner) if inner.strip().startswith("{") else {}
  dec=inner.get("decoded") if isinstance(inner.get("decoded"), dict) else inner
  print(int(dec.get("block_id") or dec.get("blockId") or 0))
except Exception:
  print(0)
' "$line" 2>/dev/null || echo 0)"
        rem=$((block_id % 50))
        if (( rem <= 2 )); then
          echo "CLOCK_50 prove window ok: block_id=$block_id rem=$rem" >&2
          return 0
        fi
      fi
      echo "CLOCK_50 window wait: block_id=$block_id rem=$rem (want <=2) attempt=$attempt" >&2
    else
      echo "CLOCK_50 window wait: could not parse block_id attempt=$attempt" >&2
    fi
    sleep 2
  done
  return 1
}

# Wait until CLOCK_50 crosses into a new 50-block epoch so stream accrual can
# fold under real prove (D39.25).
ps_wait_clock50_advance() {
  command -v logoscore >/dev/null 2>&1 || return 1
  local attempt=0 max_attempts="${PS_CLOCK50_ADVANCE_ATTEMPTS:-120}"
  local start_id=0 block_id=0 line start_epoch
  line="$(logoscore call payment_streams_module readClockDecoded "$PS_CLOCK50_ACCOUNT_ID" 2>/dev/null | tail -1)"
  start_id="$(python3 -c '
import json,sys
try:
  outer=json.loads(sys.argv[1])
  inner=outer.get("result","{}")
  if isinstance(inner,str):
    inner=json.loads(inner) if inner.strip().startswith("{") else {}
  dec=inner.get("decoded") if isinstance(inner.get("decoded"), dict) else inner
  print(int(dec.get("block_id") or dec.get("blockId") or 0))
except Exception:
  print(0)
' "$line" 2>/dev/null || echo 0)"
  start_epoch=$((start_id / 50))
  echo "CLOCK_50 advance wait: start block_id=$start_id epoch=$start_epoch" >&2
  while (( attempt < max_attempts )); do
    attempt=$((attempt + 1))
    sleep 5
    if declare -F sync_wallet >/dev/null 2>&1; then
      sync_wallet 2>/dev/null || true
    fi
    line="$(logoscore call payment_streams_module readClockDecoded "$PS_CLOCK50_ACCOUNT_ID" 2>/dev/null | tail -1)"
    block_id="$(python3 -c '
import json,sys
try:
  outer=json.loads(sys.argv[1])
  inner=outer.get("result","{}")
  if isinstance(inner,str):
    inner=json.loads(inner) if inner.strip().startswith("{") else {}
  dec=inner.get("decoded") if isinstance(inner.get("decoded"), dict) else inner
  print(int(dec.get("block_id") or dec.get("blockId") or 0))
except Exception:
  print(0)
' "$line" 2>/dev/null || echo 0)"
    if (( block_id / 50 > start_epoch )); then
      echo "CLOCK_50 advanced: $start_id -> $block_id" >&2
      return 0
    fi
    if (( attempt % 6 == 0 )); then
      local tip=""
      if declare -F ps_seq_url >/dev/null 2>&1; then
        tip="$(curl -sf -X POST "$(ps_seq_url)" -H 'Content-Type: application/json' \
          -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' 2>/dev/null |
          python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("result",""))' 2>/dev/null || true)"
      fi
      echo "CLOCK_50 advance wait: clock_block=$block_id start=$start_id tip=${tip:-?} next_tick=$(( (start_epoch + 1) * 50 )) attempt=$attempt" >&2
    fi
  done
  return 1
}

# Read the on-chain next_stream_id for a vault; non-zero exit if the vault
# config account has no data (vault not initialized).
ps_vault_next_stream_id() {
  local owner="$1" vault_id="${2:-0}"
  local guest wallet_home
  guest="${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/program/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
  wallet_home="${LEE_WALLET_HOME_DIR:-$(ps_chain_wallet_home)}"
  LEE_WALLET_HOME_DIR="$wallet_home" cargo run -q \
    --manifest-path "$REPO_ROOT/verify/seed/Cargo.toml" \
    --bin seed_localnet_fixture -- read-vault-next-stream-id \
    --program-bin "$guest" --owner "$owner" --vault-id "$vault_id" 2>/dev/null
}

# Print the vault's unallocated balance (holding - total_allocated) from chain.
ps_vault_unallocated_lo() {
  local owner="$1" vault_id="${2:-0}"
  local guest wallet_home
  guest="${PAYMENT_STREAMS_GUEST_BIN:-$REPO_ROOT/program/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
  wallet_home="${LEE_WALLET_HOME_DIR:-$(ps_chain_wallet_home)}"
  LEE_WALLET_HOME_DIR="$wallet_home" cargo run -q \
    --manifest-path "$REPO_ROOT/verify/seed/Cargo.toml" \
    --bin seed_localnet_fixture -- read-vault-unallocated \
    --program-bin "$guest" --owner "$owner" --vault-id "$vault_id" 2>/dev/null
}

# Prepend the pinned LEZ release dir (wallet, spel) to PATH if present.
ps_prepend_lez_wallet_path() {
  # Prefer scaffold operator LEZ release wallet (v0.2.4) over ~/.cargo/bin/wallet.
  local lez_release
  lez_release="$(ps_lez_cache)/target/release"
  if [[ -d "$lez_release" ]]; then
    export PATH="$lez_release:$PATH"
  fi
}

ps_pinned_lez_wallet_bin() {
  ps_prepend_lez_wallet_path
  local pinned bin
  pinned="$(ps_lez_cache)/target/release/wallet"
  if [[ -x "$pinned" ]]; then
    printf '%s\n' "$pinned"
    return 0
  fi
  bin="$(command -v wallet 2>/dev/null || true)"
  [[ -n "$bin" ]] || return 1
  printf '%s\n' "$bin"
}

ps_assert_pinned_wallet_bin() {
  local bin="$1"
  local cache
  cache="$(ps_lez_cache)"
  [[ -n "$bin" ]] || {
    echo "ERROR: wallet binary not found (pinned LEZ release under $cache/target/release)" >&2
    return 1
  }
  case "$bin" in
    "$cache"/*) return 0 ;;
  esac
  echo "ERROR: wallet resolves outside the pinned LEZ release: $bin" >&2
  echo "  expected under $cache/target/release (run lgs setup / build wallet)" >&2
  return 2
}

# Fail fast if logoscore still answers; a held storage.json futex-waits the CLI.
ps_assert_logoscore_down_for_wallet_cli() {
  command -v logoscore >/dev/null 2>&1 || return 0
  if timeout 2 logoscore call logos_execution_zone save >/dev/null 2>&1; then
    echo "ERROR: logoscore still holds storage.json; stop the daemon before wallet CLI" >&2
    return 1
  fi
  return 0
}

ps_wallet_cli() {
  local bin
  bin="$(ps_pinned_lez_wallet_bin)" || {
    echo "wallet binary not on PATH (pinned LEZ release)" >&2
    return 1
  }
  ps_assert_pinned_wallet_bin "$bin" || return $?
  ps_assert_logoscore_down_for_wallet_cli || return 1
  env -u LD_LIBRARY_PATH "$bin" "$@"
}

ps_wallet_cli_timeout() {
  local secs="$1"
  shift
  local bin
  bin="$(ps_pinned_lez_wallet_bin)" || {
    echo "wallet binary not on PATH (pinned LEZ release)" >&2
    return 1
  }
  ps_assert_pinned_wallet_bin "$bin" || return $?
  ps_assert_logoscore_down_for_wallet_cli || return 1
  timeout "$secs" env -u LD_LIBRARY_PATH "$bin" "$@"
}

# ImageID (hex bytes) of the authenticated_transfer program for on-chain verify.
# Override: PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX
# Under the Step 45 split, resolve from the operator LEZ cache (live sequencer AT),
# not from program-graph `authenticated_transfer().id()` (graph AT differs).
ps_authenticated_transfer_program_id_hex() {
  if [[ -n "${PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX:-}" ]]; then
    echo "${PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX}" | tr '[:upper:]' '[:lower:]'
    return 0
  fi
  local cache bin
  cache="$(ps_lez_cache)"
  for bin in \
    "$cache/artifacts/program_methods/authenticated_transfer.bin" \
    "$cache/artifacts/lez/programs/authenticated_transfer.bin"; do
    if [[ -f "$bin" ]] && command -v spel >/dev/null 2>&1; then
      local hex
      hex="$(spel inspect "$bin" 2>/dev/null | grep 'ImageID (hex bytes)' | awk '{print $NF}' || true)"
      if [[ -n "$hex" ]]; then
        echo "$hex" | tr '[:upper:]' '[:lower:]'
        return 0
      fi
    fi
  done
  ps_prepend_lez_wallet_path
  if command -v spel >/dev/null 2>&1; then
    for bin in \
      "$cache/artifacts/program_methods/authenticated_transfer.bin" \
      "$cache/artifacts/lez/programs/authenticated_transfer.bin"; do
      if [[ -f "$bin" ]]; then
        local hex
        hex="$(spel inspect "$bin" 2>/dev/null | grep 'ImageID (hex bytes)' | awk '{print $NF}' || true)"
        if [[ -n "$hex" ]]; then
          echo "$hex" | tr '[:upper:]' '[:lower:]'
          return 0
        fi
      fi
    done
  fi
  return 1
}

# Export live AT hex for seed / WalletCore paths that still default to graph AT.
ps_export_authenticated_transfer_program_id_hex() {
  local hex
  if ! hex="$(ps_authenticated_transfer_program_id_hex)"; then
    ps_log_error "Could not resolve PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX (operator AT bin / override)"
    return 1
  fi
  export PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX="$hex"
  ps_log_info "PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX=$hex"
}

# Fetch getAccount JSON result for acct; up to 3 attempts on curl/RPC failure.
# Prints result JSON object on stdout; non-zero exit on failure.
ps_get_account_result() {
  local acct="$1" attempt body
  for attempt in 1 2 3; do
    body="$(curl -sf --max-time 15 -X POST "$(ps_seq_url)" -H 'Content-Type: application/json' \
      -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$acct\"]}" 2>/dev/null || true)"
    if [[ -n "$body" ]] && python3 -c 'import json,sys
try:
    d=json.loads(sys.argv[1])
    sys.exit(0 if d.get("result") is not None else 1)
except Exception:
    sys.exit(1)
' "$body" 2>/dev/null; then
      python3 -c 'import json,sys; print(json.dumps(json.loads(sys.argv[1]).get("result") or {}))' "$body"
      return 0
    fi
    [[ "$attempt" -lt 3 ]] && sleep 2
  done
  return 1
}

# Normalize program_owner limb array to 64-char lowercase hex (empty if invalid).
ps_program_owner_limbs_to_hex() {
  python3 -c 'import json,struct,sys
try:
    po=json.loads(sys.argv[1])
    if not isinstance(po, list) or len(po) != 8:
        sys.exit(0)
    h="".join(struct.pack("<I", int(x) & 0xFFFFFFFF).hex() for x in po)
    print(h.lower())
except Exception:
    pass
' "$1" 2>/dev/null
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

# True when program_owner matches authenticated_transfer ImageID (strict).
# Sets PS_AT_VERIFY_MODE=image_id|nonzero_only|rpc_error on failure paths when exported by caller.
ps_account_is_at_initialized() {
  local acct="$1"
  local result po_hex at_hex
  if ! result="$(ps_get_account_result "$acct")"; then
    PS_AT_VERIFY_MODE=rpc_error
    export PS_AT_VERIFY_MODE
    return 1
  fi
  po_hex="$(python3 -c 'import json,sys; r=json.loads(sys.argv[1]); print(json.dumps(r.get("program_owner") or []))' "$result")"
  po_hex="$(ps_program_owner_limbs_to_hex "$po_hex")"
  [[ -n "$po_hex" ]] || return 1
  if at_hex="$(ps_authenticated_transfer_program_id_hex)"; then
    PS_AT_VERIFY_MODE=image_id
    export PS_AT_VERIFY_MODE
    [[ "$po_hex" == "$at_hex" ]]
    return $?
  fi
  if ps_is_testnet; then
    PS_AT_VERIFY_MODE=image_id_unresolved
    export PS_AT_VERIFY_MODE
    return 1
  fi
  PS_AT_VERIFY_MODE=nonzero_only
  export PS_AT_VERIFY_MODE
  python3 -c 'import json,sys
try:
    po=json.loads(sys.argv[1]).get("program_owner") or []
    sys.exit(0 if any(int(x)!=0 for x in po) else 1)
except Exception:
    sys.exit(1)
' "$result"
}

# E2E scaffold layout (SSOT for paths): docs/reference/names.md#scaffold-layout
ps_scaffold_e2e_dir() {
  echo "$REPO_ROOT/.scaffold/e2e"
}

ps_e2e_user_modules_dir() {
  echo "$(ps_scaffold_e2e_dir)/user/modules"
}

ps_e2e_provider_modules_dir() {
  echo "$(ps_scaffold_e2e_dir)/provider/modules"
}

ps_e2e_user_logoscore_dir() {
  echo "$(ps_scaffold_e2e_dir)/user/logoscore"
}

ps_e2e_provider_logoscore_dir() {
  echo "$(ps_scaffold_e2e_dir)/provider/logoscore"
}

ps_e2e_user_persist_dir() {
  echo "$(ps_scaffold_e2e_dir)/user/persist"
}

ps_e2e_provider_persist_dir() {
  echo "$(ps_scaffold_e2e_dir)/provider/persist"
}

ps_e2e_artifacts_dir() {
  echo "$(ps_scaffold_e2e_dir)/artifacts"
}

ps_e2e_testnet_wallet_dir() {
  echo "$(ps_scaffold_e2e_dir)/testnet-wallet"
}

# Module localnet: isolated wallet reset each module-e2e run (fresh owner/provider).
ps_e2e_user_wallet_local_dir() {
  echo "$(ps_scaffold_e2e_dir)/user/wallet-local"
}

# Store dual-host: independent wallet homes (never share storage.json across daemons).
ps_e2e_user_wallet_dir() {
  echo "$(ps_scaffold_e2e_dir)/user/wallet"
}

ps_e2e_provider_wallet_dir() {
  echo "$(ps_scaffold_e2e_dir)/provider/wallet"
}

ps_e2e_provider_ad_path() {
  echo "$(ps_scaffold_e2e_dir)/provider-advertisement.json"
}

ps_scaffold_localnet_wallet_dir() {
  echo "$REPO_ROOT/.scaffold/wallet"
}

# Default paths
ps_default_fixture_manifest() {
  if ps_is_testnet; then
    echo "$REPO_ROOT/verify/fixtures/testnet.json"
  else
    echo "$REPO_ROOT/verify/fixtures/localnet.json"
  fi
}

ps_default_wallet_config() {
  if ps_is_testnet; then
    echo "$(ps_e2e_testnet_wallet_dir)/wallet_config.json"
  else
    echo "$(ps_scaffold_localnet_wallet_dir)/wallet_config.json"
  fi
}

ps_default_wallet_storage() {
  if ps_is_testnet; then
    echo "$(ps_e2e_testnet_wallet_dir)/storage.json"
  else
    echo "$(ps_scaffold_localnet_wallet_dir)/storage.json"
  fi
}

# LEZ wallet open/create_new (v0.2.2+) take a statistics.json path beside storage.
ps_wallet_statistics_path() {
  local storage="${1:-${WALLET_STORAGE:-$(ps_default_wallet_storage)}}"
  if [[ -n "${WALLET_STATISTICS:-}" ]]; then
    echo "$WALLET_STATISTICS"
    return 0
  fi
  echo "$(dirname "$storage")/statistics.json"
}

ps_ensure_wallet_statistics() {
  local path
  path="$(ps_wallet_statistics_path "${1:-}")"
  mkdir -p "$(dirname "$path")"
  if [[ ! -f "$path" ]]; then
    printf '%s\n' '{}' >"$path"
  fi
  echo "$path"
}

# Step 45 split: operator wallet CLI (v0.2.4) requires sequencers[]; program-graph
# WalletCore (v0.2.0) still requires sequencer_addr. Keep both keys for local
# scaffold wallets so seed + pinata share one home.
ps_ensure_wallet_config_split_compatible() {
  local cfg="${1:-$(ps_default_wallet_config)}"
  local url="${2:-http://127.0.0.1:3040}"
  [[ -f "$cfg" ]] || return 0
  python3 - "$cfg" "$url" <<'PY'
import json, sys
path, url = sys.argv[1], sys.argv[2].rstrip("/")
cfg = json.loads(open(path).read())
changed = False
if "sequencer_addr" not in cfg:
    cfg["sequencer_addr"] = url
    changed = True
elif isinstance(cfg.get("sequencer_addr"), str):
    url = cfg["sequencer_addr"].rstrip("/")
if "sequencers" not in cfg:
    cfg["sequencers"] = [{"sequencer_addr": url, "basic_auth": None}]
    changed = True
cfg.setdefault(
    "multi_sequencer_client_config",
    {"distribution_limit": 1, "calibration_limit": 100},
)
if changed or "multi_sequencer_client_config" not in json.loads(open(path).read()):
    open(path, "w").write(json.dumps(cfg, indent=2) + "\n")
PY
}

# Wallet home directory (holds storage.json + wallet_config.json) the seed
# binary opens via LEE_WALLET_HOME_DIR. This selects which sequencer the seed
# CLI talks to, so it MUST follow CHAIN: testnet writes use the testnet wallet,
# not the localnet one.
ps_chain_wallet_home() {
  if ps_is_testnet; then
    ps_e2e_testnet_wallet_dir
  else
    ps_scaffold_localnet_wallet_dir
  fi
}

# Export wallet home for both env names (pinned LEZ uses LEE_*; some builds NSSA_*).
ps_export_wallet_home() {
  local home="${1:-$(ps_chain_wallet_home)}"
  export LEE_WALLET_HOME_DIR="$home"
  export NSSA_WALLET_HOME_DIR="$home"
}

# Export environment defaults
export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$(ps_default_fixture_manifest)}"
export WALLET_CONFIG="${WALLET_CONFIG:-$(ps_default_wallet_config)}"
export WALLET_STORAGE="${WALLET_STORAGE:-$(ps_default_wallet_storage)}"

# Privacy profile flags (independent owner vs provider choice).
# OWNER_PRIVACY=1 — PseudonymousFunding vault owner (Step 36).
# PROVIDER_PRIVACY=1 — private provider / shielded claim (Step 37).
# PRIVACY=1 remains an alias for OWNER_PRIVACY=1.
ps_normalize_privacy_flags() {
  if [[ -z "${OWNER_PRIVACY:-}" && "${PRIVACY:-0}" == "1" ]]; then
    OWNER_PRIVACY=1
  fi
  OWNER_PRIVACY="${OWNER_PRIVACY:-0}"
  PROVIDER_PRIVACY="${PROVIDER_PRIVACY:-0}"
  if [[ "$OWNER_PRIVACY" == "1" ]]; then
    PRIVACY=1
  else
    PRIVACY=0
  fi
  export OWNER_PRIVACY PROVIDER_PRIVACY PRIVACY
}

ps_is_owner_privacy_e2e() {
  [[ "${OWNER_PRIVACY:-0}" == "1" ]]
}

ps_is_provider_privacy_e2e() {
  [[ "${PROVIDER_PRIVACY:-0}" == "1" ]]
}

ps_is_any_privacy_e2e() {
  ps_is_owner_privacy_e2e || ps_is_provider_privacy_e2e
}

ps_privacy_profile_label() {
  local parts=()
  if ps_is_owner_privacy_e2e; then
    parts+=("OWNER_PRIVACY=1")
  fi
  if ps_is_provider_privacy_e2e; then
    parts+=("PROVIDER_PRIVACY=1")
  fi
  if ((${#parts[@]} == 0)); then
    echo "public"
    return 0
  fi
  local IFS=','
  echo "${parts[*]}"
}

ps_e2e_burned_ids_file() {
  echo "$REPO_ROOT/.scaffold/e2e/burned-private-ids.json"
}

ps_e2e_current_program_id() {
  local id="${PAYMENT_STREAMS_PROGRAM_ID_HEX:-}"
  if [[ -z "$id" ]]; then
    id="$(ps_program_id_hex || true)"
  fi
  printf '%s\n' "${id,,}"
}

# Harvest wallet privates into the burned-id file when ImageID changes.
ps_e2e_burned_apply_imageid_cut() {
  local storage="${1:-${WALLET_STORAGE:-}}"
  python3 - "$REPO_ROOT" "$(ps_e2e_current_program_id)" "$storage" <<'PY'
import json, sys
from pathlib import Path
repo, pid, storage = Path(sys.argv[1]), sys.argv[2].strip().lower(), Path(sys.argv[3]) if sys.argv[3] else Path()
path = repo / ".scaffold" / "e2e" / "burned-private-ids.json"
state = {"program_id_hex": "", "ids": []}
if path.is_file():
    try:
        loaded = json.loads(path.read_text())
        if isinstance(loaded, dict):
            state = {
                "program_id_hex": str(loaded.get("program_id_hex") or ""),
                "ids": [str(x) for x in (loaded.get("ids") or []) if str(x).strip()],
            }
    except Exception:
        pass
harvested = []
if storage.is_file():
    try:
        data = json.loads(storage.read_text())
        for row in data.get("key_chain", {}).get("accounts") or data.get("accounts") or []:
            if isinstance(row, dict) and isinstance(row.get("Private"), dict):
                aid = row["Private"].get("account_id")
                if aid:
                    harvested.append(str(aid))
    except Exception:
        pass
ids = list(state["ids"])
prev = str(state.get("program_id_hex") or "").strip().lower()
if prev and pid and prev != pid:
    ids.extend(harvested)
out, seen = [], set()
for item in ids:
    if item not in seen:
        seen.add(item)
        out.append(item)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps({"program_id_hex": pid, "ids": out}, indent=2) + "\n")
PY
}

ps_e2e_private_id_unusable() {
  local b58="$1"
  python3 - "$REPO_ROOT" "$b58" <<'PY'
import json, sys
from pathlib import Path
repo, b58 = Path(sys.argv[1]), sys.argv[2]
prefixes = ("DaV7bT45", "FqxTyJhY", "8vSpcfHE", "H7JEDimH", "2B8gB6jB", "42epagKp", "99AdxJCt")
ids = []
path = repo / ".scaffold" / "e2e" / "burned-private-ids.json"
if path.is_file():
    try:
        ids = json.loads(path.read_text()).get("ids") or []
    except Exception:
        ids = []
sys.exit(0 if (not b58 or b58 in ids or any(b58.startswith(p) for p in prefixes)) else 1)
PY
}

ps_e2e_record_burned_private() {
  local b58="$1"
  [[ -n "$b58" ]] || return 0
  python3 - "$REPO_ROOT" "$b58" "$(ps_e2e_current_program_id)" <<'PY'
import json, sys
from pathlib import Path
repo, b58, pid = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
path = repo / ".scaffold" / "e2e" / "burned-private-ids.json"
state = {"program_id_hex": pid, "ids": []}
if path.is_file():
    try:
        loaded = json.loads(path.read_text())
        if isinstance(loaded, dict):
            state["ids"] = [str(x) for x in (loaded.get("ids") or []) if str(x).strip()]
            if not pid:
                state["program_id_hex"] = str(loaded.get("program_id_hex") or "")
    except Exception:
        pass
if b58 not in state["ids"]:
    state["ids"].append(b58)
if pid:
    state["program_id_hex"] = pid
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(state, indent=2) + "\n")
PY
}

ps_log_info "Common library loaded (REPO_ROOT=$REPO_ROOT)"
