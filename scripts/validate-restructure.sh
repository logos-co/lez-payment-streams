#!/usr/bin/env bash
# Validation levels for script restructuring
# Usage: ./scripts/validate-restructure.sh [level]
#   level 1: Static validation (syntax, imports)
#   level 2: Interface validation (dependencies, structure)
#   level 3: Quick smoke (external binaries)
#   level 4: Prepare phase validation

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

LEVEL="${1:-1}"
ERRORS=0

log() {
  echo "[validate:$LEVEL] $*"
}

error() {
  echo "[validate:ERROR] $*" >&2
  ((ERRORS++)) || true
}

# Level 1: Static Validation
run_level_1() {
  log "=== Level 1: Static Validation ==="

  log "Checking Python syntax..."
  for py in scripts/e2e/*.py; do
    if python3 -m py_compile "$py" 2>/dev/null; then
      log "  OK: $py"
    else
      error "Python syntax error: $py"
    fi
  done

  log "Checking Bash syntax..."
  for sh in scripts/*.sh scripts/e2e/*.sh; do
    if [[ -f "$sh" ]]; then
      if bash -n "$sh" 2>/dev/null; then
        log "  OK: $sh"
      else
        error "Bash syntax error: $sh"
      fi
    fi
  done

  log "Level 1 complete. Errors: $ERRORS"
  return $ERRORS
}

# Level 2: Interface Validation
run_level_2() {
  log "=== Level 2: Interface Validation ==="

  log "Checking sourced script dependencies..."
  grep -h "source.*scripts/" scripts/*.sh 2>/dev/null | \
    sed 's/.*source "\([^"]*\)".*/\1/' | \
    sed 's|^\$REPO/||' | \
    sort -u | while read -r sourced; do
    # Expand variables like $REPO_ROOT
    resolved="${sourced/\$REPO_ROOT/$REPO_ROOT}"
    resolved="${resolved/\$REPO/\$REPO_ROOT}"
    if [[ -f "$resolved" ]] || [[ -f "$REPO_ROOT/$sourced" ]]; then
      : # ok
    else
      # Check if it's meant to be resolved at runtime
      if [[ "$sourced" == *"REPO"* ]] || [[ "$sourced" == *"TESTNET_WALLET_DIR"* ]]; then
        : # Runtime-resolved, skip
      else
        error "Missing sourced file: $sourced"
      fi
    fi
  done

  log "Checking Python can import required functions..."
  if python3 -c "
import sys
sys.path.insert(0, 'scripts/e2e')
from run_local_e2e import min_unaccrued_lo_for_proof, stream_fundable_wait_s
print('Python interfaces OK')
" 2>/dev/null; then
    log "  OK: Python interfaces"
  else
    error "Python import failed"
  fi

  log "Checking JSON fixture examples..."
  for json in fixtures/*.json.example; do
    if [[ -f "$json" ]]; then
      if python3 -c "import json; json.load(open('$json'))" 2>/dev/null; then
        log "  OK: $json"
      else
        error "Invalid JSON: $json"
      fi
    fi
  done

  log "Level 2 complete. Errors: $ERRORS"
  return $ERRORS
}

# Level 3: Quick Smoke
run_level_3() {
  log "=== Level 3: Quick Smoke ==="
  log "Note: Requires localnet/tools to be available"

  if command -v logoscore >/dev/null 2>&1; then
    if logoscore --version >/dev/null 2>&1; then
      log "  OK: logoscore binary"
    else
      error "logoscore --version failed"
    fi
  else
    log "  SKIP: logoscore not in PATH"
  fi

  if command -v lgs >/dev/null 2>&1; then
    log "  OK: lgs binary"
  else
    log "  SKIP: lgs not in PATH"
  fi

  log "Level 3 complete. Errors: $ERRORS"
  return $ERRORS
}

# Level 4: Prepare Phase
run_level_4() {
  log "=== Level 4: Prepare Phase Validation ==="
  log "Note: This will interact with localnet state"

  if [[ -f "scripts/demo-localnet-prepare.sh" ]]; then
    log "Checking prepare script exists: OK"
  else
    error "Missing: scripts/demo-localnet-prepare.sh"
  fi

  log "Level 4 complete. Errors: $ERRORS"
  return $ERRORS
}

# Main
case "$LEVEL" in
  1) run_level_1 ;;
  2) run_level_1 && run_level_2 ;;
  3) run_level_1 && run_level_2 && run_level_3 ;;
  4) run_level_1 && run_level_2 && run_level_3 && run_level_4 ;;
  *) echo "Usage: $0 [1|2|3|4]" >&2; exit 1 ;;
esac

if [[ $ERRORS -eq 0 ]]; then
  log "All validation passed!"
  exit 0
else
  log "Validation failed with $ERRORS error(s)"
  exit 1
fi
