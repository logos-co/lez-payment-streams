#!/usr/bin/env bash
# Chain inclusion polling for payment-streams E2E scripts.
# Usage: source after common.sh

set -euo pipefail

[[ -n "${PS_CHAIN_POLL_SOURCED:-}" ]] && return 0
PS_CHAIN_POLL_SOURCED=1

# Defaults match module-e2e.sh (local 20×5s, testnet 45×2s when CHAIN is set).
ps_inclusion_defaults() {
  if ps_is_testnet; then
    INCLUSION_ATTEMPTS="${INCLUSION_ATTEMPTS:-45}"
    INCLUSION_SLEEP="${INCLUSION_SLEEP:-2}"
  else
    INCLUSION_ATTEMPTS="${INCLUSION_ATTEMPTS:-20}"
    INCLUSION_SLEEP="${INCLUSION_SLEEP:-5}"
  fi
  export INCLUSION_ATTEMPTS INCLUSION_SLEEP
}

# seq_tx_included <tx_hash> -> 0 if getTransaction returns a non-null result.
seq_tx_included() {
  local hash="$1" res
  [[ -n "$hash" ]] || return 1
  res="$(curl -sf -X POST "$(ps_seq_url)" -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getTransaction\",\"params\":[\"$hash\"]}" 2>/dev/null || true)"
  python3 -c '
import json,sys
try:
    d=json.loads(sys.argv[1])
    r=d.get("result")
    ok = r is not None and r != "" and r != []
except Exception:
    ok=False
sys.exit(0 if ok else 1)
' "$res" 2>/dev/null
}

# await_inclusion <tx_hash> -> poll until included or budget exhausted.
await_inclusion() {
  ps_inclusion_defaults
  local hash="$1" attempt
  for attempt in $(seq 1 "${INCLUSION_ATTEMPTS}"); do
    if seq_tx_included "$hash"; then
      return 0
    fi
    sleep "${INCLUSION_SLEEP}"
  done
  return 1
}
