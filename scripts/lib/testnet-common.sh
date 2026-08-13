#!/usr/bin/env bash
# Shared testnet operator helpers (operator LEZ pin v0.2.4 / Step 45).
# AT ensure for E2E: prefer scripts/auth-transfer-ensure.sh (Step 32); bootstrap
# one-liner migration deferred (Step 32 D6).
set -euo pipefail

if [[ -z "${REPO_ROOT:-}" ]]; then
  REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fi
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"

export LEZ_OP_REV="${LEZ_OP_REV:-47eba256479f6f785acbd138834340703cd03401}"
export TESTNET_SEQUENCER="${TESTNET_SEQUENCER:-https://testnet.lez.logos.co/}"
export TESTNET_WALLET_DIR="${TESTNET_WALLET_DIR:-$(ps_e2e_testnet_wallet_dir)}"
export TESTNET_WALLET_PASSWORD="${TESTNET_WALLET_PASSWORD:-testnet-dev}"
export PROGRAM_BIN="${PROGRAM_BIN:-$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"

# TESTNET_PROGRAM_ID_HEX override, else derive from the built guest via make -s program-id.
# Does not auto-build (D50.10).
ps_testnet_program_id_hex() {
  if [[ -n "${TESTNET_PROGRAM_ID_HEX:-}" ]]; then
    printf '%s\n' "$TESTNET_PROGRAM_ID_HEX"
    return 0
  fi
  if [[ ! -f "$PROGRAM_BIN" ]]; then
    echo "ERROR: guest binary missing at $PROGRAM_BIN (run make build); cannot derive program id" >&2
    return 1
  fi
  local id
  id="$(make -s -C "$REPO_ROOT" program-id | sed -n 's/.*ImageID (hex bytes): //p' | tr -d ' ')"
  if [[ -z "$id" || "${#id}" -ne 64 ]]; then
    echo "ERROR: make -s program-id did not print ImageID (hex bytes)" >&2
    return 1
  fi
  printf '%s\n' "$id"
}

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
  # Prefer scaffold operator LEZ release wallet (v0.2.4).
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
  echo "ERROR: wallet CLI not found for operator LEZ ${LEZ_OP_REV}." >&2
  echo "  Run: lgs setup (scaffold.toml [repos.lez] pin) then build wallet," >&2
  echo "  or set LEZ_WALLET to that target/release/wallet." >&2
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
  # logos_execution_zone module tracks live LEZ (v0.2.2+): sequencers[] shape.
  python3 -c "
import json, os
path = os.environ['TESTNET_WALLET_DIR'] + '/wallet_config.json'
url = os.environ['TESTNET_SEQUENCER'].rstrip('/')
cfg = {
  'sequencers': [{
    'sequencer_addr': url,
    'basic_auth': None,
  }],
  'seq_poll_timeout': '60s',
  'seq_tx_poll_max_blocks': 120,
  'seq_poll_max_retries': 20,
  'seq_block_poll_max_amount': 100,
  'multi_sequencer_client_config': {
    'distribution_limit': 1,
    'calibration_limit': 100,
  },
}
json.dump(cfg, open(path, 'w'), indent=2)
"
}

# Wallet CLI home for LEZ tags that match public getProgramIds (v0.2.2+).
# Kept separate from TESTNET_WALLET_DIR so the v0.2.0 module config/storage
# stay openable while auth-transfer/pinata/deploy use the matching CLI.
write_testnet_wallet_cli_config() {
  local cli_dir="${TESTNET_WALLET_CLI_DIR:-${TESTNET_WALLET_DIR}-cli-v024}"
  mkdir -p "$cli_dir"
  if [[ -f "${TESTNET_WALLET_DIR}/storage.json" && ! -f "${cli_dir}/storage.json" ]]; then
    cp -a "${TESTNET_WALLET_DIR}/storage.json" "${cli_dir}/storage.json"
  fi
  python3 -c "
import json, os
path = os.environ['TESTNET_WALLET_CLI_DIR'] + '/wallet_config.json'
url = os.environ['TESTNET_SEQUENCER'].rstrip('/')
cfg = {
  'sequencers': [{
    'sequencer_addr': url,
    'basic_auth': None,
  }],
  'seq_poll_timeout': '60s',
  'seq_tx_poll_max_blocks': 120,
  'seq_poll_max_retries': 20,
  'seq_block_poll_max_amount': 100,
  'multi_sequencer_client_config': {
    'distribution_limit': 1,
    'calibration_limit': 100,
  },
}
json.dump(cfg, open(path, 'w'), indent=2)
print('Wrote', path)
"
  export TESTNET_WALLET_CLI_DIR="$cli_dir"
}

ensure_testnet_wallet() {
  write_testnet_wallet_config
  export TESTNET_WALLET_CLI_DIR="${TESTNET_WALLET_CLI_DIR:-${TESTNET_WALLET_DIR}-cli-v024}"
  export TESTNET_WALLET_CLI_DIR
  write_testnet_wallet_cli_config
  # Module/logoscore use TESTNET_WALLET_DIR. Wallet CLI may use a separate
  # TESTNET_WALLET_CLI_DIR when CLI storage is kept apart from the module home.
  export NSSA_WALLET_HOME_DIR="$TESTNET_WALLET_CLI_DIR"
  export LEE_WALLET_HOME_DIR="$TESTNET_WALLET_CLI_DIR"
  local wallet_bin
  wallet_bin="$(lez_wallet_bin)"
  if [[ -f "$TESTNET_WALLET_DIR/storage.json" ]]; then
    if TESTNET_WALLET_DIR="$TESTNET_WALLET_DIR" testnet_wallet_public_id >/dev/null 2>&1; then
      return 0
    fi
  fi
  echo "Creating testnet wallet at $TESTNET_WALLET_DIR …"
  # Create via CLI home, then copy storage into the module home.
  printf '%s\n' "$TESTNET_WALLET_PASSWORD" | "$wallet_bin" account new public >/dev/null
  if [[ ! -f "$TESTNET_WALLET_CLI_DIR/storage.json" ]]; then
    echo "ERROR: wallet storage not created" >&2
    exit 1
  fi
  cp -a "$TESTNET_WALLET_CLI_DIR/storage.json" "$TESTNET_WALLET_DIR/storage.json"
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
