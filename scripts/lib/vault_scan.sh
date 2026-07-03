#!/usr/bin/env bash
# vault_scan.sh — Shared vault config probe and Store E2E vault id resolution.
# Sourced by fixture.sh and ensure-testnet-vault.sh (do not execute directly).

# True when the vault config account is missing or has zero bytes (not initialized).
vault_config_is_empty() {
  local owner="$1"
  local vault_id="${2:-0}"
  [[ -n "$owner" ]] || return 1
  local next
  next="$(ps_vault_next_stream_id "$owner" "$vault_id" 2>/dev/null)" || return 0
  [[ -z "$next" ]] && return 0
  return 1
}

# Print the vault id to use for a Store run (stdout). Honors VAULT_ID when set.
resolve_store_vault_id() {
  local owner
  owner="$(resolve_owner)"
  [[ -n "$owner" ]] || ps_fatal "resolve_store_vault_id: no owner in state/manifest"

  if [[ -n "${VAULT_ID:-}" ]]; then
    echo "$VAULT_ID"
    return 0
  fi

  local id=0
  while ! vault_config_is_empty "$owner" "$id"; do
    id=$((id + 1))
  done
  echo "$id"
}
