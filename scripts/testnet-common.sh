#!/usr/bin/env bash
# Shared Step 18 testnet operator helpers (rc3 pin cf3639d8).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

export LEZ_RC3_REV="${LEZ_RC3_REV:-cf3639d8252040d13b3d4e933feb19b42c76e14a}"
export TESTNET_SEQUENCER="${TESTNET_SEQUENCER:-https://testnet.lez.logos.co/}"
export TESTNET_WALLET_DIR="${TESTNET_WALLET_DIR:-$REPO_ROOT/.scaffold/e2e/testnet-wallet}"
export TESTNET_WALLET_PASSWORD="${TESTNET_WALLET_PASSWORD:-testnet-dev}"
export PROGRAM_BIN="${PROGRAM_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"

lez_rc3_wallet_bin() {
  if [[ -n "${LEZ_RC3_WALLET:-}" && -x "${LEZ_RC3_WALLET}" ]]; then
    echo "$LEZ_RC3_WALLET"
    return 0
  fi
  local checkout="$HOME/.cargo/git/checkouts/logos-execution-zone-"*/"${LEZ_RC3_REV:0:7}/target/release/wallet"
  # shellcheck disable=SC2086
  if compgen -G "$checkout" >/dev/null 2>&1; then
    # shellcheck disable=SC2086
    echo "$(readlink -f $checkout | head -1)"
    return 0
  fi
  echo "ERROR: rc3 wallet not found. Build with:" >&2
  echo "  cd \"\$(ls -d \$HOME/.cargo/git/checkouts/logos-execution-zone-*/${LEZ_RC3_REV:0:7} | head -1)\"" >&2
  echo "  cargo build --release -p wallet" >&2
  return 1
}

lez_testnet_submit_bin() {
  if [[ -n "${LEZ_TESTNET_SUBMIT:-}" && -x "${LEZ_TESTNET_SUBMIT}" ]]; then
    echo "$LEZ_TESTNET_SUBMIT"
    return 0
  fi
  local built="$REPO_ROOT/tools/lez-testnet-submit/target/release/lez-testnet-submit"
  if [[ -x "$built" ]]; then
    echo "$built"
    return 0
  fi
  echo "ERROR: build lez-testnet-submit (cd tools/lez-testnet-submit && cargo build --release)" >&2
  return 1
}

write_testnet_wallet_config() {
  mkdir -p "$TESTNET_WALLET_DIR"
  python3 -c "
import json, os
path = os.environ['TESTNET_WALLET_DIR'] + '/wallet_config.json'
url = os.environ['TESTNET_SEQUENCER']
cfg = {
  'sequencer_addr': url,
  'seq_poll_timeout': '30s',
  'seq_tx_poll_max_blocks': 15,
  'seq_poll_max_retries': 10,
  'seq_block_poll_max_amount': 100,
  'basic_auth': None,
}
json.dump(cfg, open(path, 'w'), indent=2)
"
}

ensure_testnet_rc3_wallet() {
  write_testnet_wallet_config
  export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
  local wallet_bin
  wallet_bin="$(lez_rc3_wallet_bin)"
  if [[ -f "$TESTNET_WALLET_DIR/storage.json" ]]; then
    return 0
  fi
  echo "Creating rc3 testnet wallet at $TESTNET_WALLET_DIR …"
  printf '%s\n' "$TESTNET_WALLET_PASSWORD" | "$wallet_bin" account new public >/dev/null
  if [[ ! -f "$TESTNET_WALLET_DIR/storage.json" ]]; then
    echo "ERROR: rc3 wallet storage not created" >&2
    exit 1
  fi
}

testnet_wallet_public_id() {
  python3 -c "
import json, os
path = os.environ['TESTNET_WALLET_DIR'] + '/storage.json'
for entry in json.load(open(path)).get('accounts', []):
    pub = entry.get('Public')
    if not pub:
        continue
    cid = pub.get('account_id', '')
    if not cid:
        continue
    chain = pub.get('chain_index')
    if chain == [0] or chain == []:
        print(cid)
        break
else:
    raise SystemExit('no public account in testnet wallet storage')
"
}

ensure_testnet_owner_funded() {
  local owner rounds
  owner="$(testnet_wallet_public_id)"
  if [[ -z "$owner" ]]; then
    echo "ERROR: no public account in testnet rc3 wallet" >&2
    exit 1
  fi
  if [[ "${TESTNET_SKIP_PINATA:-0}" == "1" ]]; then
    echo "Skipping pinata (TESTNET_SKIP_PINATA=1); owner Public/$owner" >&2
    echo "$owner"
    return 0
  fi
  export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
  local wallet_bin
  wallet_bin="$(lez_rc3_wallet_bin)"
  rounds="${TESTNET_PINATA_ROUNDS:-3}"
  echo "Initializing auth-transfer for Public/$owner …"
  "$wallet_bin" auth-transfer init --account-id "Public/$owner" || true
  echo "Funding owner Public/$owner (pinata x$rounds)…"
  for ((i = 1; i <= rounds; i++)); do
    "$wallet_bin" pinata claim --to "Public/$owner" || true
  done
  echo "$owner"
}

sync_testnet_owner_to_510_wallet() {
  local owner_pk owner_id
  owner_id="$(testnet_wallet_public_id)"
  export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
  local rc3_wallet
  rc3_wallet="$(lez_rc3_wallet_bin)"
  owner_pk="$(TESTNET_WALLET_DIR="$TESTNET_WALLET_DIR" owner_id="$owner_id" python3 -c "
import json, os
wid = os.environ['owner_id']
for entry in json.load(open(os.environ['TESTNET_WALLET_DIR'] + '/storage.json')).get('accounts', []):
    pub = entry.get('Public') or {}
    if pub.get('account_id') == wid:
        print(pub.get('data', {}).get('csk', ''))
        break
")"
  if [[ -z "$owner_pk" ]]; then
    echo "WARN: could not read owner pk from rc3 wallet list; 510 sign may fail" >&2
    return 0
  fi
  export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$REPO_ROOT/.scaffold/wallet}"
  local lez510_wallet="$HOME/.cache/logos-scaffold/repos/lez/62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60/target/release/wallet"
  if [[ ! -x "$lez510_wallet" ]]; then
    echo "WARN: 510 wallet CLI missing; skip import public key" >&2
    return 0
  fi
  if "$lez510_wallet" account list 2>&1 | grep -q "$owner_id"; then
    echo "Owner $owner_id already in 510 wallet"
    return 0
  fi
  echo "Importing testnet owner signing key into 510 wallet storage…"
  "$lez510_wallet" account import public --private-key "$owner_pk" || {
    echo "WARN: import public failed (510 open may still work if key present)" >&2
  }
}

patch_510_wallet_config_for_testnet() {
  local src="${1:-$REPO_ROOT/.scaffold/wallet/wallet_config.json}"
  local dst="${2:-$REPO_ROOT/.scaffold/e2e/testnet-wallet/wallet_config_510.json}"
  python3 -c "
import json, sys
src, dst, url = sys.argv[1], sys.argv[2], sys.argv[3]
c = json.load(open(src))
c['sequencer_addr'] = url
json.dump(c, open(dst, 'w'), indent=2)
" "$src" "$dst" "$TESTNET_SEQUENCER"
  echo "$dst"
}

testnet_rpc_last_block() {
  python3 "$REPO_ROOT/scripts/testnet_rpc.py" block-height
}

require_testnet_rpc() {
  if ! testnet_rpc_last_block >/dev/null 2>&1; then
    echo "ERROR: testnet sequencer unreachable at $TESTNET_SEQUENCER (expected getLastBlockId JSON-RPC)" >&2
    exit 1
  fi
}
