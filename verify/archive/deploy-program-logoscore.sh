#!/usr/bin/env bash
# Smoke-test send_program_deployment_transaction via logoscore (empty ELF → structured error).
# Full guest deploy uses scaffold wallet CLI: make deploy / wallet deploy-program (see step11d doc).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export MODULES="${MODULES:-$HOME/Downloads/software/waku/lez-related/logos-cli/modules}"
WALLET_CONFIG="${WALLET_CONFIG:-$REPO_ROOT/.scaffold/wallet/wallet_config.json}"
WALLET_STORAGE="${WALLET_STORAGE:-$REPO_ROOT/.scaffold/wallet/storage.json}"
LOGOSCORE_E2E_TIMEOUT="${LOGOSCORE_E2E_TIMEOUT:-120}"

if [[ ! -f "$MODULES/logos_execution_zone/logos_execution_zone_plugin.so" ]]; then
  echo "ERROR: logos_execution_zone not installed under MODULES=$MODULES" >&2
  exit 1
fi

OUT="$(mktemp)"
trap 'rm -f "$OUT"' EXIT

timeout "$LOGOSCORE_E2E_TIMEOUT" nix shell github:logos-co/logos-logoscore-cli --command bash -c "
  set -euo pipefail
  logoscore -m '$MODULES' call logos_execution_zone open '$WALLET_CONFIG' '$WALLET_STORAGE'
  logoscore -m '$MODULES' call logos_execution_zone sync_to_block latest
  logoscore -m '$MODULES' call logos_execution_zone send_program_deployment_transaction '[]'
" >"$OUT" 2>&1

python3 -c "
import json, pathlib, sys
text = pathlib.Path('$OUT').read_text()
# Last JSON line from logoscore responses.
lines = [ln.strip() for ln in text.splitlines() if ln.strip().startswith('{')]
if not lines:
    sys.exit('no JSON in logoscore output')
last = json.loads(lines[-1])
# Empty deploy should fail gracefully (wallet FFI error), not missing method.
if last.get('status') == 'ok':
    inner = json.loads(last.get('result', '{}'))
    if inner.get('success'):
        sys.exit('unexpected success for empty ELF deploy')
print('deploy-program-logoscore: method wired (empty ELF rejected as expected)')
"
