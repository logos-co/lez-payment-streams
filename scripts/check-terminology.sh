#!/usr/bin/env bash
# Step 47 (D47.11): mechanical role-terminology gate.
# Fail on living payer/payee, module JSON "signer"/"authority", owner-id signer*
# helpers, StreamAuthority*, requesterPeerId — with path and meta allowlists.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

failures=0
note() { printf '%s\n' "$*"; }
fail() { note "FAIL: $*"; failures=$((failures + 1)); }
ok() { note "OK: $*"; }

# Paths excluded from scans (historical / self-referential).
is_allowlisted_path() {
  local f="$1"
  case "$f" in
    docs/plan/completed/*|docs/plan/wontfix/*|docs/archive/*) return 0 ;;
    scripts/check-terminology.sh) return 0 ;;
    .scaffold/*|target/*|vendor/*|nimbledeps/*) return 0 ;;
    *) return 1 ;;
  esac
}

# Step 46: living docs must not use User Journey / Developer Journey as product labels.
is_journey_name_allowlisted() {
  local f="$1"
  case "$f" in
    docs/plan/*|docs/archive/*) return 0 ;;
    docs/presentation.md|docs/handoff-*) return 0 ;;
    docs/reference/integration-decisions.md) return 0 ;;
    scripts/check-terminology.sh) return 0 ;;
    .scaffold/*|target/*|vendor/*|nimbledeps/*) return 0 ;;
    *) return 1 ;;
  esac
}

# rg over living trees; print matches that are not path-allowlisted.
scan_pattern() {
  local pattern="$1"
  local label="$2"
  shift 2
  local paths=("$@")
  local tmp
  tmp="$(mktemp)"
  # shellcheck disable=SC2068
  if ! rg -n --no-heading -e "$pattern" ${paths[@]} \
      --glob '!target/**' --glob '!.scaffold/**' --glob '!vendor/**' \
      --glob '!**/Cargo.lock' \
      >"$tmp" 2>/dev/null; then
    rm -f "$tmp"
    ok "$label (0 matches)"
    return 0
  fi
  local hit=0
  while IFS= read -r line; do
    local file="${line%%:*}"
    if is_allowlisted_path "$file"; then
      continue
    fi
    # Meta-usage allowlist: naming-conventions formerly-pointer / Informal row.
    if [[ "$file" == "docs/reference/naming-conventions.md" ]]; then
      if [[ "$line" == *"Formerly"* || "$line" == *"Informal"* || "$line" == *'`payer`'* || "$line" == *'`payee`'* ]]; then
        continue
      fi
    fi
    # LEZ must-sign IDL / guest attrs: "signer": true/false boolean fields.
    if [[ "$file" == "lez-payment-streams-idl.json" || "$file" == methods/guest/* ]]; then
      if [[ "$line" == *'"signer":'* || "$line" == *'#[account(signer)]'* || "$line" == *'signer)'* ]]; then
        continue
      fi
    fi
    # Guest account param name authority_account stays until Step 44.
    if [[ "$file" == methods/guest/* && "$line" == *authority_account* ]]; then
      continue
    fi
    # SIGNER_ID fixture key (inventory only).
    if [[ "$line" == *SIGNER_ID* ]]; then
      continue
    fi
    # Historical step filenames in links.
    if [[ "$line" == *step-36-payer* || "$line" == *step-37-payee* || "$line" == *step-44-payer* ]]; then
      continue
    fi
    # Legacy Makefile aliases until Step 50.
    if [[ "$line" == *payee-close* ]]; then
      continue
    fi
    note "  $line"
    hit=1
  done <"$tmp"
  rm -f "$tmp"
  if [[ "$hit" -eq 1 ]]; then
    fail "$label"
  else
    ok "$label (allowlisted only)"
  fi
}

note "=== Step 47 terminology gate (D47.11) ==="

# 1) Quoted JSON keys in module src after flip.
mod_signer="$(rg -n --no-heading -e '"signer"' logos-payment-streams-module/src 2>/dev/null || true)"
mod_auth="$(rg -n --no-heading -e '"authority"' logos-payment-streams-module/src 2>/dev/null || true)"
if [[ -n "$mod_signer" ]]; then
  note "$mod_signer"
  fail 'quoted "signer" in logos-payment-streams-module/src'
else
  ok 'no quoted "signer" in module src'
fi
if [[ -n "$mod_auth" ]]; then
  note "$mod_auth"
  fail 'quoted "authority" in logos-payment-streams-module/src'
else
  ok 'no quoted "authority" in module src'
fi

# 2) Owner-id helpers named signer* / *Signer* in module-owned code.
scan_pattern \
  'signerAccountId|ownerBytesFromSigner|enforceDepositSigner|depositSignerMismatch|SignerField|signerHexLower|signerBase58|signerAccountIdHex' \
  'module owner-id signer* helpers' \
  logos-payment-streams-module/

# 3) StreamAuthority* layout names.
scan_pattern \
  'StreamAuthority|stream_authority_instruction|plan_stream_authority' \
  'StreamAuthority* layout symbols' \
  lez-payment-streams-core/src lez-payment-streams-ffi/ logos-payment-streams-module/

# 4) requesterPeerId / requester_peer_id on living payment-streams surfaces.
scan_pattern \
  'requesterPeerId|requester_peer_id' \
  'requesterPeerId living names' \
  logos-payment-streams-module/ docs/reference/ docs/reproduce/ docs/integrate/ docs/external/ docs/presentation.md docs/payment-streams-module/

# 5) Journey env PAYER/PAYEE.
scan_pattern \
  '\bPAYER\b|\bPAYEE\b' \
  'PAYER/PAYEE env tokens' \
  docs/reproduce/ scripts/ docs/reference/ docs/payment-streams-module/ README.md AGENTS.md

# 6) Living informal payer/payee (word tokens); allow meta + historical paths.
scan_pattern \
  '\bpayer\b|\bpayee\b|\bPayer\b|\bPayee\b' \
  'living payer/payee prose' \
  logos-payment-streams-module/ \
  lez-payment-streams-core/src/policy/ \
  lez-payment-streams-core/src/stream_provider_policy.rs \
  lez-payment-streams-core/src/instruction_accounts.rs \
  lez-payment-streams-ffi/ \
  scripts/ \
  docs/reproduce/ \
  docs/integrate/ \
  docs/external/ \
  docs/reference/ \
  docs/on-chain/ \
  docs/payment-streams-module/ \
  docs/plan/upcoming/ \
  README.md \
  AGENTS.md

# 7) Policy *payee* symbols.
scan_pattern \
  'payee_binding|expected_payee|proposal_payee|_payee_' \
  'policy *payee* symbols' \
  lez-payment-streams-core/src lez-payment-streams-ffi/

# 8) Living journey-name scan (Step 46 D46.12).
scan_journey_names() {
  local tmp
  tmp="$(mktemp)"
  local paths=(
    README.md
    AGENTS.md
    docs/README.md
    docs/reproduce/
    docs/integrate/
    docs/external/
    docs/on-chain/
    docs/payment-streams-module/
    docs/reference/
    scripts/README.md
    scripts/e2e.sh
    scripts/module-e2e.sh
    scripts/module-e2e-local.sh
    scripts/module-e2e-privacy.sh
    scripts/e2e/run_local_e2e.py
    scripts/user-journey-reset.sh
    scripts/user-journey-shell.sh
    scripts/lib/user-journey-env.sh
    scripts/lib/common.sh
  )
  if ! rg -n --no-heading -e 'User Journey' -e 'Developer Journey' -e 'E2E\.md' \
      "${paths[@]}" \
      --glob '!target/**' --glob '!.scaffold/**' \
      >"$tmp" 2>/dev/null; then
    rm -f "$tmp"
    ok "living journey-name scan (0 matches)"
    return 0
  fi
  local hit=0
  while IFS= read -r line; do
    local file="${line%%:*}"
    if is_journey_name_allowlisted "$file"; then
      continue
    fi
    if [[ "$file" == "docs/reference/naming-conventions.md" && "$line" == *"Formerly"* ]]; then
      continue
    fi
    note "  $line"
    hit=1
  done <"$tmp"
  rm -f "$tmp"
  if [[ "$hit" -eq 1 ]]; then
    fail "living User Journey / Developer Journey / E2E.md labels"
  else
    ok "living journey-name scan (allowlisted only)"
  fi
}
scan_journey_names

# 9) Retired path grep (Step 46 D46.16).
scan_retired_paths() {
  local tmp
  tmp="$(mktemp)"
  local paths=(
    README.md
    AGENTS.md
    Makefile
    docs/README.md
    docs/reproduce/
    docs/integrate/
    docs/external/
    docs/on-chain/
    docs/payment-streams-module/
    docs/reference/
    docs/context-manifest.json
    scripts/
  )
  if ! rg -n --no-heading -e 'docs/journeys' -e 'docs/store-integration' \
      "${paths[@]}" \
      --glob '!target/**' --glob '!.scaffold/**' --glob '!scripts/archive/**' \
      --glob '!scripts/check-terminology.sh' \
      >"$tmp" 2>/dev/null; then
    rm -f "$tmp"
    ok "retired path grep (0 matches)"
    return 0
  fi
  local hit=0
  while IFS= read -r line; do
    local file="${line%%:*}"
    case "$file" in
      docs/plan/*|docs/archive/*|docs/presentation.md|docs/handoff-*|scripts/check-terminology.sh)
        continue
        ;;
    esac
    note "  $line"
    hit=1
  done <"$tmp"
  rm -f "$tmp"
  if [[ "$hit" -eq 1 ]]; then
    fail "retired docs/journeys or docs/store-integration paths"
  else
    ok "retired path grep (allowlisted only)"
  fi
}
scan_retired_paths

if [[ "$failures" -ne 0 ]]; then
  note "=== $failures check(s) failed ==="
  exit 1
fi
note "=== terminology gate passed ==="
exit 0
