#!/usr/bin/env bash
# Soft smoke: Store dual-host split wallets + user stop/restart remounts delivery
# while private keys remain resolvable. No wallet CLI prove / CLOCK_50.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$REPO_ROOT/scripts/lib/common.sh"

export PATH="${PATH:-}"
if [[ -d "$REPO_ROOT/../logos-logoscore-cli/result/bin" ]]; then
  export PATH="$REPO_ROOT/../logos-logoscore-cli/result/bin:$PATH"
fi

command -v logoscore >/dev/null || ps_fatal "logoscore not on PATH"
command -v python3 >/dev/null || ps_fatal "python3 required"
export RISC0_DEV_MODE=1
CHAIN="${CHAIN:-testnet}"
export CHAIN

SEED_HOME="$(ps_chain_wallet_home)"
[[ -f "$SEED_HOME/wallet_config.json" && -f "$SEED_HOME/storage.json" ]] || ps_fatal "missing seed wallet"

WORK="${TMPDIR:-/tmp}/ps-store-host-restart-$$"
export REPO="$REPO_ROOT"
export LOGOSCORE_CONFIG_USER="$WORK/user/logoscore"
export LOGOSCORE_CONFIG_PROVIDER="$WORK/provider/logoscore"
export PERSIST_USER="$WORK/user/persist"
export PERSIST_PROVIDER="$WORK/provider/persist"
export MODULES_USER="${MODULES_USER:-$(ps_e2e_user_modules_dir)}"
export MODULES_PROVIDER="${MODULES_PROVIDER:-$(ps_e2e_provider_modules_dir)}"
export WALLET_HOME_USER="$WORK/user/wallet"
export WALLET_HOME_PROVIDER="$WORK/provider/wallet"
mkdir -p "$LOGOSCORE_CONFIG_USER" "$LOGOSCORE_CONFIG_PROVIDER" "$PERSIST_USER" "$PERSIST_PROVIDER"

cleanup() {
  logoscore --config-dir "$LOGOSCORE_CONFIG_USER" stop >/dev/null 2>&1 || true
  logoscore --config-dir "$LOGOSCORE_CONFIG_PROVIDER" stop >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

python3 - <<'PY'
import json
import os
import sys
import time
from pathlib import Path

repo = Path(os.environ["REPO"])
sys.path.insert(0, str(repo / "scripts" / "e2e"))
import run_local_e2e as e2e

seed_cfg = Path(os.environ.get("WALLET_CONFIG", repo / ".scaffold" / "e2e" / "testnet-wallet" / "wallet_config.json"))
if not seed_cfg.is_file():
    seed_cfg = Path(e2e.__file__).resolve().parents[2] / ".scaffold" / "e2e" / "testnet-wallet" / "wallet_config.json"
# Prefer chain wallet home from env LEE if present.
lee = Path(os.environ.get("LEE_WALLET_HOME_DIR", ""))
if (lee / "wallet_config.json").is_file():
    seed_cfg = lee / "wallet_config.json"
seed_ws = seed_cfg.parent / "storage.json"

cfg_u = Path(os.environ["LOGOSCORE_CONFIG_USER"])
cfg_p = Path(os.environ["LOGOSCORE_CONFIG_PROVIDER"])
mod_u = Path(os.environ["MODULES_USER"])
mod_p = Path(os.environ["MODULES_PROVIDER"])
pers_u = Path(os.environ["PERSIST_USER"])
pers_p = Path(os.environ["PERSIST_PROVIDER"])

wc_u, ws_u, wc_p, ws_p = e2e.prepare_split_store_wallets(
    repo, cfg_u, cfg_p, seed_cfg, seed_ws
)

e2e.start_daemon(cfg_p, mod_p, pers_p)
e2e.load_modules(cfg_p)
e2e.open_wallet(cfg_p, wc_p, ws_p)
prov_create = {
    "mode": "Core",
    **e2e.local_waku_json_base(),
    "portsShift": 100,
    "relay": True,
    "store": True,
    "storeMessageRetentionPolicy": "capacity:1000",
    "storeMessageDbUrl": f"sqlite://{(pers_p / 'store.sqlite3').as_posix()}",
}
e2e.delivery_create_start(cfg_p, prov_create, persist=pers_p, label="provider")
e2e.register_store_host_runtime(
    cfg_p,
    modules=mod_p,
    persist=pers_p,
    delivery_create=prov_create,
    label="provider",
)
peer_id = e2e.get_node_info(cfg_p, "MyPeerId")
addr = e2e.loopback_multiaddr(peer_id, 60100)

e2e.start_daemon(cfg_u, mod_u, pers_u)
e2e.load_modules(cfg_u)
e2e.open_wallet(cfg_u, wc_u, ws_u)
user_create = {
    "mode": "Core",
    **e2e.local_waku_json_base(),
    "portsShift": 0,
    "relay": True,
    "store": False,
    "staticnodes": [addr],
    "storenode": addr,
}
e2e.delivery_create_start(cfg_u, user_create, persist=pers_u, label="user")
e2e.register_store_host_runtime(
    cfg_u,
    modules=mod_u,
    persist=pers_u,
    delivery_create=user_create,
    label="user",
)

priv = e2e.create_account(cfg_u, private=True)
e2e.logoscore_cmd(cfg_u, "call", "logos_execution_zone", "save", timeout=60)
hex_id = e2e.account_id_to_hex(cfg_u, priv)
b58_before = e2e.account_id_to_base58(cfg_u, hex_id)

# Exclusive handoff: stop user only; provider stays up.
e2e.stop_store_host_for_wallet_cli(cfg_u)
time.sleep(1)
# Simulate wallet CLI touching user storage (touch mtime; keys unchanged).
ws_u.touch()
e2e.restart_store_host_after_wallet_cli(cfg_u, "https://testnet.lez.logos.co/")
b58_after = e2e.account_id_to_base58(cfg_u, hex_id)
if b58_after != b58_before:
    raise SystemExit(f"private resolve mismatch {b58_before} vs {b58_after}")
# Provider still serves peer id.
peer2 = e2e.get_node_info(cfg_p, "MyPeerId")
if peer2 != peer_id:
    raise SystemExit(f"provider peer changed {peer_id} -> {peer2}")
user_peer = e2e.get_node_info(cfg_u, "MyPeerId")
if not user_peer:
    raise SystemExit("user delivery remount failed (no peer id)")
print(f"OK: user stop/restart remount; private {b58_after[:12]}…; provider peer unchanged")
PY

ps_log_info "verify-store-host-restart: PASS"
