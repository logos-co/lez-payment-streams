#!/usr/bin/env bash
# Store E2E: ensure vault init + deposit on public testnet (identity manifest + vault PDAs).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"
# shellcheck source=scripts/archive/testnet-common.sh
source "$REPO_ROOT/scripts/archive/testnet-common.sh"

MANIFEST=""
VAULT_ID_ARG=""
DEPOSIT_AMOUNT=""
WALLET_CONFIG_PATH=""
WALLET_STORAGE_PATH=""
SEQUENCER_URL=""
PROGRAM_ID_HEX=""
PROGRAM_BIN=""
SUBMIT_HELPER=""
DRY_RUN=0
VERIFY_ONLY=0

usage() {
  cat <<'EOF'
Usage: ensure-testnet-vault.sh --manifest PATH --vault-id ID --deposit-amount LO \
  --wallet-config PATH --wallet-storage PATH --sequencer-url URL \
  --program-id-hex HEX --program-bin PATH [--submit-helper PATH] [--dry-run] [--verify-only]
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest) MANIFEST="$2"; shift 2 ;;
    --vault-id) VAULT_ID_ARG="$2"; shift 2 ;;
    --deposit-amount) DEPOSIT_AMOUNT="$2"; shift 2 ;;
    --wallet-config) WALLET_CONFIG_PATH="$2"; shift 2 ;;
    --wallet-storage) WALLET_STORAGE_PATH="$2"; shift 2 ;;
    --sequencer-url) SEQUENCER_URL="$2"; shift 2 ;;
    --program-id-hex) PROGRAM_ID_HEX="$2"; shift 2 ;;
    --program-bin) PROGRAM_BIN="$2"; shift 2 ;;
    --submit-helper) SUBMIT_HELPER="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --verify-only) VERIFY_ONLY=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$MANIFEST" && -n "$VAULT_ID_ARG" && -n "$DEPOSIT_AMOUNT" ]] || {
  usage
  exit 1
}
[[ -f "$MANIFEST" ]] || ps_fatal "manifest not found: $MANIFEST"

require_testnet_rpc
ensure_testnet_wallet

PROGRAM_BIN="${PROGRAM_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
PROGRAM_ID_HEX="${PROGRAM_ID_HEX:-16b95d3701d256eecd41d5a55e4f570331994d787abf0cba44eec209e24f8a44}"
WALLET_CONFIG_PATH="${WALLET_CONFIG_PATH:-$TESTNET_WALLET_DIR/wallet_config.json}"
WALLET_STORAGE_PATH="${WALLET_STORAGE_PATH:-$TESTNET_WALLET_DIR/storage.json}"
SEQUENCER_URL="${SEQUENCER_URL:-$(python3 -c "import json; print(json.load(open('$MANIFEST')).get('sequencer_url','').strip() or 'https://testnet.lez.logos.co/')")}"
SUBMIT_BIN="${SUBMIT_HELPER:-$(lez_testnet_submit_bin)}"
export LEZ_TESTNET_SUBMIT="$SUBMIT_BIN"
export PATH="$(dirname "$SUBMIT_BIN"):$PATH"

OWNER="$(python3 -c "import json; print(json.load(open('$MANIFEST')).get('owner_account_id','').strip())")"
PROVIDER="$(python3 -c "import json; print(json.load(open('$MANIFEST')).get('provider_account_id','').strip())")"
STREAM_RATE="$(python3 -c "import json; m=json.load(open('$MANIFEST')); print(int(m.get('stream_rate',1)))")"
ALLOCATION="$(python3 -c "import json; m=json.load(open('$MANIFEST')); print(int(m.get('allocation', m.get('stream_allocation',400))))")"

[[ -n "$OWNER" && -n "$PROVIDER" ]] || ps_fatal "manifest missing owner_account_id or provider_account_id"

if [[ "$DRY_RUN" == "1" || "$VERIFY_ONLY" == "1" ]]; then
  python3 - "$MANIFEST" "$VAULT_ID_ARG" "$DEPOSIT_AMOUNT" "$OWNER" <<'PY'
import json, sys
manifest, vid, dep, owner = sys.argv[1:5]
m = json.load(open(manifest))
m["vault_id"] = int(vid)
m["demo_deposit_amount"] = int(dep)
print(json.dumps({"dry_run": True, "vault_id": int(vid), "deposit_amount": int(dep), "owner": owner}, indent=2))
PY
  [[ "$VERIFY_ONLY" == "1" ]] && exit 0
  [[ "$DRY_RUN" == "1" ]] && exit 0
fi

[[ -f "$PROGRAM_BIN" ]] || ps_fatal "missing program bin: $PROGRAM_BIN"

echo "=== ensure-testnet-vault vault=$VAULT_ID_ARG deposit=$DEPOSIT_AMOUNT owner=$OWNER ==="

# Top up deposit when holding exists but is under target (idempotent ensure).
FORCE_FLAG=()
if [[ "${FORCE_DEPOSIT:-0}" == "1" ]]; then
  FORCE_FLAG=(--force)
fi

cargo run --quiet --manifest-path "$REPO_ROOT/examples/Cargo.toml" --bin bootstrap_testnet_fixture -- \
  --program-bin "$PROGRAM_BIN" \
  --owner "$OWNER" \
  --provider "$PROVIDER" \
  --program-id-hex "$PROGRAM_ID_HEX" \
  --rc3-wallet-config "$WALLET_CONFIG_PATH" \
  --rc3-wallet-storage "$WALLET_STORAGE_PATH" \
  --submit-helper "$SUBMIT_BIN" \
  --sequencer-url "$SEQUENCER_URL" \
  --vault-id "$VAULT_ID_ARG" \
  --stream-id 0 \
  --deposit-amount "$DEPOSIT_AMOUNT" \
  --stream-rate "$STREAM_RATE" \
  --allocation "$ALLOCATION" \
  --write-manifest "$MANIFEST" \
  --skip-if-initialized \
  "${FORCE_FLAG[@]}"

echo "=== ensure-testnet-vault done: $MANIFEST ==="
