#!/usr/bin/env bash
# Shared Step 18 testnet operator helpers (operational LEZ pin v0.2.0).
set -euo pipefail

if [[ -z "${REPO_ROOT:-}" ]]; then
  REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fi

export LEZ_OP_REV="${LEZ_OP_REV:-a58fbce2ff48c58b7bb5001b1a27e64b9596ee3a}"
export TESTNET_SEQUENCER="${TESTNET_SEQUENCER:-https://testnet.lez.logos.co/}"
export TESTNET_WALLET_DIR="${TESTNET_WALLET_DIR:-$REPO_ROOT/.scaffold/e2e/testnet-wallet}"
export TESTNET_WALLET_PASSWORD="${TESTNET_WALLET_PASSWORD:-testnet-dev}"
export PROGRAM_BIN="${PROGRAM_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"

export WALLET_CONFIG="${WALLET_CONFIG:-$TESTNET_WALLET_DIR/wallet_config.json}"
export WALLET_STORAGE="${WALLET_STORAGE:-$TESTNET_WALLET_DIR/storage.json}"
export NSSA_WALLET_HOME_DIR="${NSSA_WALLET_HOME_DIR:-$TESTNET_WALLET_DIR}"
export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$TESTNET_WALLET_DIR}"

lez_scaffold_cache_dir() {
  echo "${HOME}/.cache/logos-scaffold/repos/lez/${LEZ_OP_REV}"
}

lez_wallet_bin() {
  if [[ -n "${LEZ_WALLET:-}" && -x "${LEZ_WALLET}" ]]; then
    echo "$LEZ_WALLET"
    return 0
  fi
  local built
  built="$(lez_scaffold_cache_dir)/target/release/wallet"
  if [[ -x "$built" ]]; then
    echo "$built"
    return 0
  fi
  local checkout="$HOME/.cargo/git/checkouts/logos-execution-zone-"*/"${LEZ_OP_REV:0:7}/target/release/wallet"
  # shellcheck disable=SC2086
  if compgen -G "$checkout" >/dev/null 2>&1; then
    # shellcheck disable=SC2086
    echo "$(readlink -f $checkout | head -1)"
    return 0
  fi
  echo "ERROR: v0.2.0 wallet not found. Run lgs setup from repo root or:" >&2
  echo "  cd \"\$(ls -d \$HOME/.cargo/git/checkouts/logos-execution-zone-*/${LEZ_OP_REV:0:7} | head -1)\"" >&2
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
  'seq_poll_timeout': '60s',
  'seq_tx_poll_max_blocks': 30,
  'seq_poll_max_retries': 10,
  'seq_block_poll_max_amount': 100,
  'basic_auth': None,
}
json.dump(cfg, open(path, 'w'), indent=2)
"
}

ensure_testnet_wallet() {
  write_testnet_wallet_config
  export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
  export LEE_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
  local wallet_bin
  wallet_bin="$(lez_wallet_bin)"
  if [[ -f "$TESTNET_WALLET_DIR/storage.json" ]]; then
    if TESTNET_WALLET_DIR="$TESTNET_WALLET_DIR" testnet_wallet_public_id >/dev/null 2>&1; then
      return 0
    fi
  fi
  echo "Creating testnet wallet at $TESTNET_WALLET_DIR …"
  printf '%s\n' "$TESTNET_WALLET_PASSWORD" | "$wallet_bin" account new public >/dev/null
  if [[ ! -f "$TESTNET_WALLET_DIR/storage.json" ]]; then
    echo "ERROR: wallet storage not created" >&2
    exit 1
  fi
}

testnet_wallet_public_id() {
  python3 -c "
import json, os
path = os.environ['TESTNET_WALLET_DIR'] + '/storage.json'
data = json.load(open(path))
accounts = data.get('accounts') or data.get('key_chain', {}).get('accounts') or []
for entry in accounts:
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

testnet_owner_balance() {
  local owner
  owner="$(testnet_wallet_public_id)"
  python3 "$REPO_ROOT/scripts/testnet_rpc.py" account-balance "$owner" 2>/dev/null || echo "0"
}

ensure_testnet_owner_funded() {
  local owner rounds balance
  owner="$(testnet_wallet_public_id)"
  if [[ -z "$owner" ]]; then
    echo "ERROR: no public account in testnet wallet" >&2
    exit 1
  fi
  if [[ "${TESTNET_SKIP_PINATA:-0}" == "1" ]]; then
    balance="$(testnet_owner_balance)"
    if [[ -z "$balance" || "$balance" == "0" ]]; then
      echo "ERROR: TESTNET_SKIP_PINATA=1 but owner Public/$owner has zero balance on testnet." >&2
      echo "Run bootstrap once with Piñata (unset TESTNET_SKIP_PINATA) or fund the owner manually." >&2
      exit 1
    fi
    echo "Skipping pinata (TESTNET_SKIP_PINATA=1); owner Public/$owner balance=$balance" >&2
    echo "$owner"
    return 0
  fi
  export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
  export LEE_WALLET_HOME_DIR="$TESTNET_WALLET_DIR"
  local wallet_bin
  wallet_bin="$(lez_wallet_bin)"
  rounds="${TESTNET_PINATA_ROUNDS:-3}"
  echo "Initializing auth-transfer for Public/$owner …" >&2
  "$wallet_bin" auth-transfer init --account-id "Public/$owner" >&2 || true
  echo "Funding owner Public/$owner (pinata x$rounds)…" >&2
  for ((i = 1; i <= rounds; i++)); do
    "$wallet_bin" pinata claim --to "Public/$owner" >&2 || true
  done
  echo "$owner"
}

testnet_auth_transfer_elf_path() {
  if [[ -n "${TESTNET_AUTH_TRANSFER_ELF_PATH:-}" && -f "${TESTNET_AUTH_TRANSFER_ELF_PATH}" ]]; then
    echo "${TESTNET_AUTH_TRANSFER_ELF_PATH}"
    return 0
  fi
  local path
  path="$(lez_scaffold_cache_dir)/artifacts/program_methods/authenticated_transfer.bin"
  if [[ -f "$path" ]]; then
    echo "$path"
    return 0
  fi
  echo "ERROR: authenticated_transfer.bin not found under $(lez_scaffold_cache_dir); run lgs setup" >&2
  return 1
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
