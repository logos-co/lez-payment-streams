#!/usr/bin/env bash
# Step 18 Part B — one-time guest deploy to public testnet (rc5 wallet deploy-program).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=verify/testnet/testnet-common.sh
source "$REPO_ROOT/verify/testnet/testnet-common.sh"

require_testnet_rpc
ensure_testnet_wallet

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Building guest…"
  make build
fi

EXPECTED_ID="${TESTNET_PROGRAM_ID_HEX:-$(make -s program-id | sed -n 's/.*ImageID (hex bytes): //p' | tr -d ' ')}"
WALLET_BIN="$(lez_wallet_bin)"
export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
export LEE_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"

echo "=== deploy-testnet (expected program-id ${EXPECTED_ID:-unknown}, ELF $(stat -c%s "$PROGRAM_BIN") bytes) ==="
set +e
"$WALLET_BIN" deploy-program "$PROGRAM_BIN"
DEPLOY_RC=$?
set -e

if [[ "$DEPLOY_RC" -ne 0 ]]; then
  echo "deploy-program exit $DEPLOY_RC (program may already be on chain; verify with make program-id)"
else
  echo "deploy-program exit 0"
fi

ACTUAL_ID="$(ps_guest_bin_image_id_hex "$PROGRAM_BIN" || true)"
if [[ -z "$ACTUAL_ID" ]]; then
  ACTUAL_ID="$(make -s program-id | sed -n 's/.*ImageID (hex bytes): //p' | tr -d ' ')"
fi
if [[ -n "$EXPECTED_ID" && "$ACTUAL_ID" != "$EXPECTED_ID" ]]; then
  echo "WARN: guest ImageID ($ACTUAL_ID) != expected ($EXPECTED_ID)" >&2
fi
echo "program_id_hex=$ACTUAL_ID"
if [[ -n "$ACTUAL_ID" ]]; then
  ps_pin_deployed_guest_or_die "$PROGRAM_BIN" "$ACTUAL_ID"
  python3 - "$ACTUAL_ID" "$REPO_ROOT/verify/fixtures/testnet.json" "$REPO_ROOT/verify/fixtures/testnet-module.json" <<'PY'
import json, sys
pid = sys.argv[1]
for path in sys.argv[2:]:
    try:
        with open(path, encoding="utf-8") as f:
            data = json.load(f)
    except FileNotFoundError:
        continue
    data["program_id_hex"] = pid
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
        f.write("\n")
    print("updated", path, "program_id_hex", pid)
PY
fi
echo "=== deploy-testnet done ==="
