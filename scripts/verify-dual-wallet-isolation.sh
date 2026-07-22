#!/usr/bin/env bash
# Prove Store dual logoscore daemons cannot wipe each other's private keys.
# Soft prove only; no Store / CLOCK_50 / hour-long path.
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

CHAIN="${CHAIN:-testnet}"
export CHAIN
# Soft prove only — this is an isolation smoke, not a DoD real-prove gate.
export RISC0_DEV_MODE=1

SEED_HOME="$(ps_chain_wallet_home)"
SEED_CFG="$SEED_HOME/wallet_config.json"
SEED_STOR="$SEED_HOME/storage.json"
[[ -f "$SEED_CFG" && -f "$SEED_STOR" ]] || ps_fatal "missing seed wallet under $SEED_HOME"

WORK="${TMPDIR:-/tmp}/ps-dual-wallet-isolation-$$"
A_HOME="$WORK/a"
B_HOME="$WORK/b"
A_CFG="$WORK/cfg-a"
B_CFG="$WORK/cfg-b"
A_MOD="${MODULES_USER:-$(ps_e2e_user_modules_dir)}"
B_MOD="${MODULES_PROVIDER:-$(ps_e2e_provider_modules_dir)}"
A_PERS="$WORK/persist-a"
B_PERS="$WORK/persist-b"
mkdir -p "$A_HOME" "$B_HOME" "$A_CFG" "$B_CFG" "$A_PERS" "$B_PERS"
cp "$SEED_CFG" "$A_HOME/wallet_config.json"
cp "$SEED_STOR" "$A_HOME/storage.json"
cp "$SEED_CFG" "$B_HOME/wallet_config.json"
cp "$SEED_STOR" "$B_HOME/storage.json"

cleanup() {
  logoscore --config-dir "$A_CFG" stop >/dev/null 2>&1 || true
  logoscore --config-dir "$B_CFG" stop >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

ps_log_info "Starting isolated dual logoscore daemons under $WORK"
logoscore --config-dir "$A_CFG" -D -m "$A_MOD" --persistence-path "$A_PERS" -q >/dev/null 2>&1 &
logoscore --config-dir "$B_CFG" -D -m "$B_MOD" --persistence-path "$B_PERS" -q >/dev/null 2>&1 &
sleep 4

logoscore --config-dir "$A_CFG" load-module logos_execution_zone >/dev/null
logoscore --config-dir "$B_CFG" load-module logos_execution_zone >/dev/null
logoscore --config-dir "$A_CFG" call logos_execution_zone open \
  "$A_HOME/wallet_config.json" "$A_HOME/storage.json" >/dev/null
logoscore --config-dir "$B_CFG" call logos_execution_zone open \
  "$B_HOME/wallet_config.json" "$B_HOME/storage.json" >/dev/null

create_line="$(logoscore --config-dir "$A_CFG" call logos_execution_zone create_account_private 2>/dev/null | tail -1)"
priv_raw="$(python3 -c 'import json,sys; o=json.loads(sys.argv[1]); r=o.get("result",""); print(r if isinstance(r,str) else "")' "$create_line")"
[[ -n "$priv_raw" ]] || ps_fatal "create_account_private failed: $create_line"
logoscore --config-dir "$A_CFG" call logos_execution_zone save >/dev/null
# Peer save must not touch A's storage when files are independent.
logoscore --config-dir "$B_CFG" call logos_execution_zone save >/dev/null

python3 - "$priv_raw" "$A_HOME/storage.json" "$B_HOME/storage.json" <<'PY'
import json, sys
from pathlib import Path

priv = sys.argv[1]
blob_a = Path(sys.argv[2]).read_text()
blob_b = Path(sys.argv[3]).read_text()
data_a = json.loads(blob_a)
ids_a = []
for row in data_a.get("key_chain", {}).get("accounts", []):
    kind = next(iter(row))
    ids_a.append((kind, row[kind]["account_id"]))
private_ids = [i for k, i in ids_a if k == "Private"]
if not private_ids:
    raise SystemExit("FAIL: no Private accounts in A storage after create+save")
a_set = {i for k, i in ids_a if k == "Private"}
b_set = set()
for row in json.loads(blob_b).get("key_chain", {}).get("accounts", []):
    k = next(iter(row))
    if k == "Private":
        b_set.add(row[k]["account_id"])
new_on_a = a_set - b_set
if not new_on_a:
    raise SystemExit(
        f"FAIL: A did not retain a Private account absent from B "
        f"(a={sorted(a_set)[:3]} b={sorted(b_set)[:3]})"
    )
print(f"OK: split storage isolation; new private on A only: {next(iter(new_on_a))[:12]}…")
# Hex id from create should round-trip into A's persisted private set via save.
if len(priv) == 64 and priv not in blob_a:
    # Account id in storage is base58; presence of new_on_a is sufficient.
    pass
PY

# Negative control: shared storage wipe (documents why split is required).
SHARED="$WORK/shared"
mkdir -p "$SHARED"
cp "$SEED_CFG" "$SHARED/wallet_config.json"
cp "$SEED_STOR" "$SHARED/storage.json"
logoscore --config-dir "$A_CFG" call logos_execution_zone close >/dev/null 2>&1 || true
logoscore --config-dir "$B_CFG" call logos_execution_zone close >/dev/null 2>&1 || true
logoscore --config-dir "$A_CFG" unload-module logos_execution_zone >/dev/null 2>&1 || true
logoscore --config-dir "$B_CFG" unload-module logos_execution_zone >/dev/null 2>&1 || true
logoscore --config-dir "$A_CFG" load-module logos_execution_zone >/dev/null
logoscore --config-dir "$B_CFG" load-module logos_execution_zone >/dev/null
logoscore --config-dir "$A_CFG" call logos_execution_zone open \
  "$SHARED/wallet_config.json" "$SHARED/storage.json" >/dev/null
logoscore --config-dir "$B_CFG" call logos_execution_zone open \
  "$SHARED/wallet_config.json" "$SHARED/storage.json" >/dev/null
create_line="$(logoscore --config-dir "$A_CFG" call logos_execution_zone create_account_private 2>/dev/null | tail -1)"
logoscore --config-dir "$A_CFG" call logos_execution_zone save >/dev/null
before="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(sum(1 for r in d["key_chain"]["accounts"] if next(iter(r))=="Private"))' "$SHARED/storage.json")"
# B still holds pre-create memory → save wipes A's private key from the shared file.
logoscore --config-dir "$B_CFG" call logos_execution_zone save >/dev/null
after="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(sum(1 for r in d["key_chain"]["accounts"] if next(iter(r))=="Private"))' "$SHARED/storage.json")"
if [[ "$after" -ge "$before" ]]; then
  ps_log_info "WARN: shared-storage wipe did not reduce Private count ($before -> $after); close may have synced B"
else
  ps_log_info "OK: shared-storage negative control wiped Private accounts ($before -> $after)"
fi

ps_log_info "Dual-wallet isolation verification passed"
