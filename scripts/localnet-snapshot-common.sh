#!/usr/bin/env bash
# Shared paths and snapshot metadata for Step 17b (source, do not execute).
set -euo pipefail

localnet_snapshot_repo_root() (
  cd "$(dirname "${BASH_SOURCE[1]}")/.." && pwd
)

localnet_snapshot_lez_pin() {
  grep -A2 '\[repos.lez\]' "$1/scaffold.toml" | grep '^pin' | sed 's/.*"\([^"]*\)".*/\1/'
}

localnet_snapshot_lez_cache() {
  echo "${HOME}/.cache/logos-scaffold/repos/lez/$(localnet_snapshot_lez_pin "$1")"
}

localnet_snapshot_rocksdb_dir() {
  echo "$(localnet_snapshot_lez_cache "$1")/rocksdb"
}

localnet_snapshot_default_name() {
  echo "funded"
}

localnet_snapshot_dir() {
  echo "$1/.scaffold/snapshots/$(localnet_snapshot_default_name)"
}

localnet_snapshot_program_id_hex() {
  make -C "$1" program-id 2>/dev/null | grep 'ImageID (hex bytes)' | awk '{print $NF}' || true
}

localnet_snapshot_write_metadata() {
  local repo="$1"
  local snap_dir="$2"
  local manifest="${FIXTURE_MANIFEST:-$repo/fixtures/localnet.json}"
  local deposit="${SEED_DEPOSIT_AMOUNT:-2000}"
  local rate="${SEED_STREAM_RATE:-1}"
  local alloc="${SEED_STREAM_ALLOCATION:-1800}"
  local owner="" provider=""
  if [[ -f "$repo/.lez_payment_streams-state" ]]; then
    # shellcheck disable=SC1090
    source "$repo/.lez_payment_streams-state"
    owner="${SIGNER_ID:-}"
  fi
  if [[ -f "$repo/.lez_payment_streams-fixture-provider" ]]; then
    provider="$(cat "$repo/.lez_payment_streams-fixture-provider")"
  fi
  local pin prog
  pin="$(localnet_snapshot_lez_pin "$repo")"
  prog="$(localnet_snapshot_program_id_hex "$repo")"
  python3 - "$snap_dir/snapshot.json" "$pin" "$prog" "$owner" "$provider" "$deposit" "$rate" "$alloc" <<'PY'
import json, sys, datetime
path, pin, prog, owner, provider, deposit, rate, alloc = sys.argv[1:9]
doc = {
    "schema_version": 1,
    "lez_pin": pin,
    "program_id_hex": prog,
    "owner_account_id": owner,
    "provider_account_id": provider,
    "vault_id": 0,
    "deposit_amount": int(deposit),
    "stream_rate": int(rate),
    "stream_allocation": int(alloc),
    "sequencer_url": "http://127.0.0.1:3040",
    "created_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
}
with open(path, "w", encoding="utf-8") as f:
    json.dump(doc, f, indent=2)
    f.write("\n")
PY
}

localnet_snapshot_validate_metadata() {
  local repo="$1"
  local snap_dir="$2"
  local meta="$snap_dir/snapshot.json"
  if [[ ! -f "$meta" ]]; then
    echo "ERROR: missing $meta" >&2
    return 1
  fi
  local pin prog want_pin want_prog
  pin="$(python3 -c "import json; print(json.load(open('$meta'))['lez_pin'])")"
  prog="$(python3 -c "import json; print(json.load(open('$meta'))['program_id_hex'])")"
  want_pin="$(localnet_snapshot_lez_pin "$repo")"
  want_prog="$(localnet_snapshot_program_id_hex "$repo")"
  if [[ "$pin" != "$want_pin" ]]; then
    echo "ERROR: snapshot lez_pin=$pin does not match scaffold.toml pin=$want_pin" >&2
    echo "Remedy: FULL_RESET=1 make verify-step17 (or ./scripts/demo-localnet-prepare.sh with FULL_RESET=1)" >&2
    return 1
  fi
  if [[ -z "$want_prog" ]]; then
    echo "ERROR: could not read current program id (make build && make program-id)" >&2
    return 1
  fi
  if [[ "$prog" != "$want_prog" ]]; then
    echo "ERROR: snapshot program_id_hex mismatch (snapshot=$prog current=$want_prog)" >&2
    echo "Remedy: FULL_RESET=1 after guest rebuild" >&2
    return 1
  fi
  if [[ ! -d "$snap_dir/rocksdb" ]]; then
    echo "ERROR: snapshot missing rocksdb/ under $snap_dir" >&2
    return 1
  fi
  return 0
}
