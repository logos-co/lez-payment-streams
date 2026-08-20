#!/usr/bin/env bash
# Build, prepare accounts, and launch Basecamp with payment_streams_ui env (Step 21).
# NETWORK=testnet (default) or localnet. WALLET_HOME and FIXTURE_MANIFEST override defaults.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NETWORK="${NETWORK:-testnet}"
USER_DIR="${USER_DIR:-$REPO_ROOT/.scaffold/basecamp-ui}"
BASECAMP="${BASECAMP:-$REPO_ROOT/../logos-basecamp/result/bin/LogosBasecamp}"

ps_basecamp_ui_usage() {
  echo "Usage: NETWORK=testnet|localnet $0 build|prepare|run" >&2
  exit 2
}

ps_basecamp_ui_export_env() {
  case "$NETWORK" in
    testnet)
      : "${WALLET_HOME:=$REPO_ROOT/.scaffold/e2e/testnet-wallet}"
      : "${FIXTURE_MANIFEST:=$REPO_ROOT/verify/fixtures/testnet-module.json}"
      export CHAIN=testnet
      ;;
    localnet)
      : "${WALLET_HOME:=$REPO_ROOT/.scaffold/wallet}"
      : "${FIXTURE_MANIFEST:=$REPO_ROOT/verify/fixtures/localnet.json}"
      export CHAIN=local
      ;;
    *)
      echo "NETWORK must be testnet or localnet (got '$NETWORK')" >&2
      exit 1
      ;;
  esac
  export USER_DIR
  export REPO="$REPO_ROOT"
  export WALLET_HOME
  export LEE_WALLET_HOME_DIR="$WALLET_HOME"
  export NSSA_WALLET_HOME_DIR="$WALLET_HOME"
  export FIXTURE_MANIFEST
  if [[ -f "$WALLET_HOME/wallet_config.json" ]]; then
    export WALLET_CONFIG="$WALLET_HOME/wallet_config.json"
  fi
  if [[ -f "$WALLET_HOME/storage.json" ]]; then
    export WALLET_STORAGE="$WALLET_HOME/storage.json"
  fi
  # Deposit and claim instructions embed an AT program id. The FFI default is
  # programs::authenticated_transfer().id() (graph AT). Live sequencer AT is
  # the operator LEZ-cache ELF (Step 45). logoscore exports this; Basecamp
  # logos_host must too or public deposits hash-accept and never include.
  # shellcheck source=verify/lib/common.sh
  source "$REPO_ROOT/verify/lib/common.sh"
  ps_export_authenticated_transfer_program_id_hex
}

# Public account ids in wallet storage order (same source list_accounts reads).
ps_basecamp_ui_public_accounts() {
  local storage="$1"
  python3 -c '
import json, sys
path = sys.argv[1]
with open(path, encoding="utf-8") as f:
    obj = json.load(f)
accounts = obj.get("key_chain", {}).get("accounts", [])
ids = []
for entry in accounts:
    if not isinstance(entry, dict):
        continue
    public = entry.get("Public")
    if not isinstance(public, dict):
        continue
    account_id = public.get("account_id")
    if isinstance(account_id, str) and account_id:
        ids.append(account_id)
print("\n".join(ids))
' "$storage"
}

ps_basecamp_ui_prepare() {
  ps_basecamp_ui_export_env
  # shellcheck source=verify/lib/auth_transfer.sh
  source "$REPO_ROOT/verify/lib/auth_transfer.sh"
  # shellcheck source=verify/testnet/fund_testnet.sh
  source "$REPO_ROOT/verify/testnet/fund_testnet.sh"

  if [[ ! -f "${WALLET_STORAGE:-}" ]]; then
    echo "Wallet storage missing: ${WALLET_STORAGE:-$WALLET_HOME/storage.json}" >&2
    echo "Create the wallet and two public accounts first (docs/reproduce/module.md Steps 4-7)." >&2
    exit 1
  fi
  if [[ ! -f "$FIXTURE_MANIFEST" ]]; then
    echo "FIXTURE_MANIFEST missing: $FIXTURE_MANIFEST" >&2
    exit 1
  fi
  if ! ps_seq_reachable; then
    if [[ "$NETWORK" == "localnet" ]]; then
      echo "Localnet sequencer is down. Start it before prepare:" >&2
      echo "  ./verify/lifecycle.sh localnet start" >&2
    else
      echo "Testnet sequencer unreachable at $(ps_seq_url)" >&2
    fi
    exit 1
  fi

  local accounts owner provider
  mapfile -t accounts < <(ps_basecamp_ui_public_accounts "$WALLET_STORAGE")
  if ((${#accounts[@]} < 2)); then
    echo "Wallet $WALLET_HOME has ${#accounts[@]} public account(s); the UI needs two." >&2
    echo "Create owner and provider with docs/reproduce/module.md Steps 6-7, then rerun prepare." >&2
    exit 1
  fi
  owner="${accounts[0]}"
  provider="${accounts[1]}"

  ARTIFACT="${ARTIFACT:-$REPO_ROOT/.scaffold/e2e/basecamp-ui-prepare.jsonl}"
  mkdir -p "$(dirname "$ARTIFACT")"
  : > "$ARTIFACT"
  export ARTIFACT

  echo "NETWORK=$NETWORK WALLET_HOME=$WALLET_HOME"
  echo "Registering authenticated_transfer for owner=$owner provider=$provider"
  if ! ps_auth_transfer_ensure "$owner" "$provider"; then
    echo "authenticated_transfer init failed (see $ARTIFACT)" >&2
    exit 1
  fi

  local deposit owner_target provider_min owner_bal provider_bal
  deposit="${DEPOSIT:-500}"
  owner_target="${OWNER_TARGET:-$((deposit + 50))}"
  provider_min="${PROVIDER_MIN:-50}"
  echo "Funding via pinata (owner target=$owner_target, provider min=$provider_min)"
  if ! owner_bal="$(ps_fund_testnet_account "$owner" "$owner_target" "${OWNER_MAX:-6}")"; then
    echo "Owner funding short: balance=${owner_bal:-0} target=$owner_target" >&2
    exit 1
  fi
  if ! provider_bal="$(ps_fund_testnet_account "$provider" "$provider_min" "${PROVIDER_MAX:-3}")"; then
    echo "Provider funding short: balance=${provider_bal:-0} min=$provider_min" >&2
    exit 1
  fi
  echo "Prepared owner=$owner balance=$owner_bal"
  echo "Prepared provider=$provider balance=$provider_bal"
  echo "Leave logoscore stopped, then: make basecamp-ui-run NETWORK=$NETWORK"
}

ps_basecamp_ui_build() {
  if [[ ! -x "$BASECAMP" ]]; then
    if [[ ! -f "$REPO_ROOT/../logos-basecamp/flake.nix" ]]; then
      echo "Nix-built Basecamp missing at $BASECAMP" >&2
      echo "Clone logos-basecamp as a sibling and run: (cd ../logos-basecamp && nix build '.#bin-bundle-dir')" >&2
      exit 1
    fi
    ( cd "$REPO_ROOT/../logos-basecamp" && nix build '.#bin-bundle-dir' )
  fi
  "$REPO_ROOT/verify/lib/build-wallet-lgx.sh"
  nix build "$REPO_ROOT/module#lgx-portable"
  nix build "$REPO_ROOT/ui#lgx-portable" -o "$REPO_ROOT/ui/result"
  echo "Packages ready. Install into Basecamp user dir $USER_DIR"
  echo "  wallet: $REPO_ROOT/module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/"
  echo "  module: $REPO_ROOT/result/"
  echo "  ui:     $REPO_ROOT/ui/result/"
}

ps_basecamp_ui_run() {
  ps_basecamp_ui_export_env
  if [[ ! -x "$BASECAMP" ]]; then
    echo "Basecamp binary missing at $BASECAMP. Run: make basecamp-ui-build" >&2
    exit 1
  fi
  if [[ ! -f "$FIXTURE_MANIFEST" ]]; then
    echo "FIXTURE_MANIFEST missing: $FIXTURE_MANIFEST" >&2
    exit 1
  fi
  mkdir -p "$USER_DIR"
  if command -v pgrep >/dev/null 2>&1 && pgrep -x logoscore >/dev/null 2>&1; then
    echo "logoscore is running. Stop it so storage.json is free for Basecamp (N52)." >&2
  fi
  echo "NETWORK=$NETWORK WALLET_HOME=$WALLET_HOME FIXTURE_MANIFEST=$FIXTURE_MANIFEST"
  echo "PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX=${PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX:-}"
  exec "$BASECAMP" --user-dir "$USER_DIR"
}

cmd="${1:-}"
case "$cmd" in
  build) ps_basecamp_ui_build ;;
  prepare) ps_basecamp_ui_prepare ;;
  run) ps_basecamp_ui_run ;;
  *) ps_basecamp_ui_usage ;;
esac
