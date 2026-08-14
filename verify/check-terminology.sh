#!/usr/bin/env bash
# Mechanical role-terminology gate.
# Fail on living payer/payee, module JSON "signer"/"authority", owner-id signer*
# helpers, StreamAuthority*, requesterPeerId — with path and meta allowlists.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

failures=0
note() { printf '%s\n' "$*"; }
fail() { note "FAIL: $*"; failures=$((failures + 1)); }
ok() { note "OK: $*"; }

require_paths() {
  local p
  for p in "$@"; do
    if [[ ! -e "$p" ]]; then
      fail "missing scan path: $p"
      return 1
    fi
  done
  return 0
}

# Paths excluded from scans (historical / self-referential).
is_allowlisted_path() {
  local f="$1"
  case "$f" in
    docs/plan/completed/*|docs/plan/wontfix/*|docs/archive/*|docs/external/*|verify/archive/*) return 0 ;;
    verify/check-terminology.sh) return 0 ;;
    .scaffold/*|target/*|vendor/*|nimbledeps/*) return 0 ;;
    *) return 1 ;;
  esac
}

is_journey_name_allowlisted() {
  local f="$1"
  case "$f" in
    docs/plan/*|docs/archive/*|docs/external/*) return 0 ;;
    docs/presentation.md|docs/handoff-*) return 0 ;;
    docs/reference/decisions.md) return 0 ;;
    verify/check-terminology.sh) return 0 ;;
    .scaffold/*|target/*|vendor/*|nimbledeps/*) return 0 ;;
    *) return 1 ;;
  esac
}

run_rg() {
  local out="$1"
  local err="$2"
  shift 2
  set +e
  rg -n --no-heading "$@" >"$out" 2>"$err"
  local rc=$?
  set -e
  if [[ "$rc" -ge 2 ]]; then
    note "$(cat "$err")"
    return 2
  fi
  return "$rc"
}

# rg over living trees; print matches that are not path-allowlisted.
scan_pattern() {
  local pattern="$1"
  local label="$2"
  shift 2
  local paths=("$@")
  if ! require_paths "${paths[@]}"; then
    return 0
  fi
  local tmp err
  tmp="$(mktemp)"
  err="$(mktemp)"
  local rc=0
  run_rg "$tmp" "$err" -e "$pattern" "${paths[@]}" \
      --glob '!target/**' --glob '!.scaffold/**' --glob '!vendor/**' \
      --glob '!**/Cargo.lock' --glob '!verify/archive/**' || rc=$?
  if [[ "$rc" -ge 2 ]]; then
    rm -f "$tmp" "$err"
    fail "$label (rg error)"
    return 0
  fi
  if [[ "$rc" -eq 1 ]]; then
    rm -f "$tmp" "$err"
    ok "$label (0 matches)"
    return 0
  fi
  local hit=0
  while IFS= read -r line; do
    local file="${line%%:*}"
    if is_allowlisted_path "$file"; then
      continue
    fi
    if [[ "$file" == "docs/reference/names.md" ]]; then
      if [[ "$line" == *"Formerly"* || "$line" == *"Informal"* || "$line" == *'`payer`'* || "$line" == *'`payee`'* ]]; then
        continue
      fi
    fi
    if [[ "$file" == "program/lez-payment-streams-idl.json" || "$file" == program/methods/guest/* ]]; then
      if [[ "$line" == *'"signer":'* || "$line" == *'#[account(signer)]'* || "$line" == *'signer)'* ]]; then
        continue
      fi
    fi
    if [[ "$line" == *SIGNER_ID* ]]; then
      continue
    fi
    if [[ "$line" == *step-36-payer* || "$line" == *step-37-payee* || "$line" == *step-44-payer* ]]; then
      continue
    fi
    note "  $line"
    hit=1
  done <"$tmp"
  rm -f "$tmp" "$err"
  if [[ "$hit" -eq 1 ]]; then
    fail "$label"
  else
    ok "$label (allowlisted only)"
  fi
}

note "=== terminology gate ==="

if ! require_paths module/src; then
  :
else
  mod_err="$(mktemp)"
  mod_signer_out="$(mktemp)"
  mod_auth_out="$(mktemp)"
  signer_rc=0
  auth_rc=0
  run_rg "$mod_signer_out" "$mod_err" -e '"signer"' module/src || signer_rc=$?
  if [[ "$signer_rc" -ge 2 ]]; then
    note "$(cat "$mod_err")"
    fail 'quoted "signer" in module/src (rg error)'
  elif [[ "$signer_rc" -eq 0 ]]; then
    note "$(cat "$mod_signer_out")"
    fail 'quoted "signer" in module/src'
  else
    ok 'no quoted "signer" in module src'
  fi
  run_rg "$mod_auth_out" "$mod_err" -e '"authority"' module/src || auth_rc=$?
  if [[ "$auth_rc" -ge 2 ]]; then
    note "$(cat "$mod_err")"
    fail 'quoted "authority" in module/src (rg error)'
  elif [[ "$auth_rc" -eq 0 ]]; then
    note "$(cat "$mod_auth_out")"
    fail 'quoted "authority" in module/src'
  else
    ok 'no quoted "authority" in module src'
  fi
  rm -f "$mod_err" "$mod_signer_out" "$mod_auth_out"
fi

scan_pattern \
  'signerAccountId|ownerBytesFromSigner|enforceDepositSigner|depositSignerMismatch|SignerField|signerHexLower|signerBase58|signerAccountIdHex' \
  'module owner-id signer* helpers' \
  module/

scan_pattern \
  'StreamAuthority|stream_authority_instruction|plan_stream_authority' \
  'StreamAuthority* layout symbols' \
  program/core/src module/ffi/ module/

scan_pattern \
  'requesterPeerId|requester_peer_id' \
  'requesterPeerId living names' \
  module/ docs/reference/ docs/reproduce/ docs/integrate.md docs/external/

scan_pattern \
  '\bPAYER\b|\bPAYEE\b' \
  'PAYER/PAYEE env tokens' \
  docs/reproduce/ verify/ docs/reference/ module/ README.md AGENTS.md

scan_pattern \
  '\bpayer\b|\bpayee\b|\bPayer\b|\bPayee\b' \
  'living payer/payee prose' \
  module/ \
  program/core/src/policy/ \
  program/core/src/stream_provider_policy.rs \
  program/core/src/instruction_accounts.rs \
  module/ffi/ \
  verify/ \
  docs/reproduce/ \
  docs/integrate.md \
  docs/external/ \
  docs/reference/ \
  program/README.md \
  module/README.md \
  docs/plan/upcoming/ \
  README.md \
  AGENTS.md

scan_pattern \
  'payee_binding|expected_payee|proposal_payee|_payee_' \
  'policy *payee* symbols' \
  program/core/src module/ffi/

scan_journey_names() {
  local paths=(
    README.md
    AGENTS.md
    docs/README.md
    docs/reproduce/
    docs/integrate.md
    docs/external/
    program/README.md
    module/README.md
    docs/reference/
    verify/README.md
    verify/e2e.sh
    verify/module-e2e.sh
    verify/store/run_e2e.py
    verify/repro-reset.sh
    verify/repro-shell.sh
    verify/lib/repro-env.sh
    verify/lib/common.sh
  )
  if ! require_paths "${paths[@]}"; then
    return 0
  fi
  local tmp err
  tmp="$(mktemp)"
  err="$(mktemp)"
  local rc=0
  run_rg "$tmp" "$err" -e 'User Journey' -e 'Developer Journey' -e 'E2E\.md' \
      "${paths[@]}" \
      --glob '!target/**' --glob '!.scaffold/**' --glob '!verify/archive/**' || rc=$?
  if [[ "$rc" -ge 2 ]]; then
    note "$(cat "$err")"
    rm -f "$tmp" "$err"
    fail "living journey-name scan (rg error)"
    return 0
  fi
  if [[ "$rc" -eq 1 ]]; then
    rm -f "$tmp" "$err"
    ok "living journey-name scan (0 matches)"
    return 0
  fi
  local hit=0
  while IFS= read -r line; do
    local file="${line%%:*}"
    if is_journey_name_allowlisted "$file"; then
      continue
    fi
    if [[ "$file" == "docs/reference/names.md" && "$line" == *"Formerly"* ]]; then
      continue
    fi
    note "  $line"
    hit=1
  done <"$tmp"
  rm -f "$tmp" "$err"
  if [[ "$hit" -eq 1 ]]; then
    fail "living User Journey / Developer Journey / E2E.md labels"
  else
    ok "living journey-name scan (allowlisted only)"
  fi
}
scan_journey_names

scan_retired_paths() {
  local paths=(
    README.md
    AGENTS.md
    Makefile
    docs/README.md
    docs/reproduce/
    docs/integrate.md
    docs/external/
    program/README.md
    module/README.md
    docs/reference/
    docs/plan/context-manifest.json
    verify/
  )
  if ! require_paths "${paths[@]}"; then
    return 0
  fi
  local tmp err
  tmp="$(mktemp)"
  err="$(mktemp)"
  local rc=0
  run_rg "$tmp" "$err" -e 'docs/journeys' -e 'docs/store-integration' \
      "${paths[@]}" \
      --glob '!target/**' --glob '!.scaffold/**' --glob '!verify/archive/**' \
      --glob '!verify/check-terminology.sh' || rc=$?
  if [[ "$rc" -ge 2 ]]; then
    note "$(cat "$err")"
    rm -f "$tmp" "$err"
    fail "retired path grep (rg error)"
    return 0
  fi
  if [[ "$rc" -eq 1 ]]; then
    rm -f "$tmp" "$err"
    ok "retired path grep (0 matches)"
    return 0
  fi
  local hit=0
  while IFS= read -r line; do
    local file="${line%%:*}"
    case "$file" in
      docs/plan/*|docs/archive/*|docs/presentation.md|docs/handoff-*|docs/external/*|verify/check-terminology.sh)
        continue
        ;;
    esac
    note "  $line"
    hit=1
  done <"$tmp"
  rm -f "$tmp" "$err"
  if [[ "$hit" -eq 1 ]]; then
    fail "retired docs/journeys or docs/store-integration paths"
  else
    ok "retired path grep (allowlisted only)"
  fi
}
scan_retired_paths

scan_product_door_process() {
  local paths=(
    README.md
    docs/README.md
    verify/README.md
    program/README.md
    module/README.md
    Makefile
    docs/integrate.md
    docs/reference/matrix.md
  )
  if ! require_paths "${paths[@]}"; then
    return 0
  fi
  scan_pattern \
    'verify-step[0-9]' \
    'product-door leftover verify-step aliases' \
    "${paths[@]}"
  scan_pattern \
    'user-journey' \
    'product-door leftover user-journey names' \
    "${paths[@]}"
  scan_pattern \
    'docs/plan/|step-[0-9]+-' \
    'product-door plan-packet path citations' \
    "${paths[@]}"
}
scan_product_door_process

if [[ "$failures" -ne 0 ]]; then
  note "=== $failures check(s) failed ==="
  exit 1
fi
note "=== terminology gate passed ==="
exit 0
