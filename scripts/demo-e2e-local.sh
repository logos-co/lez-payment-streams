#!/usr/bin/env bash
# Step 17 — local dual-host paid Store E2E (see docs/step17-e2e-local.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export REPO="${REPO:-$REPO_ROOT}"
E2E_PHASE="${E2E_PHASE:-all}"
SKIP_BUILD="${SKIP_BUILD:-0}"
SKIP_SEED="${SKIP_SEED:-0}"

E2E_BASE="${E2E_BASE:-$REPO/.scaffold/e2e}"
export MODULES_USER="${MODULES_USER:-$E2E_BASE/user/modules}"
export MODULES_PROVIDER="${MODULES_PROVIDER:-$E2E_BASE/provider/modules}"
export LOGOSCORE_CONFIG_USER="${LOGOSCORE_CONFIG_USER:-$E2E_BASE/user/logoscore}"
export LOGOSCORE_CONFIG_PROVIDER="${LOGOSCORE_CONFIG_PROVIDER:-$E2E_BASE/provider/logoscore}"
export PERSIST_USER="${PERSIST_USER:-$E2E_BASE/user/persist}"
export PERSIST_PROVIDER="${PERSIST_PROVIDER:-$E2E_BASE/provider/persist}"
export E2E_PROVIDER_AD="${E2E_PROVIDER_AD:-$E2E_BASE/provider-advertisement.json}"
export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$REPO/fixtures/localnet.json}"
export WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
export WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
export PAYMENT_STREAMS_GUEST_BIN="${PAYMENT_STREAMS_GUEST_BIN:-$REPO/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin}"
export PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF="${PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF:-0}"

ARTIFACT_DIR="$E2E_BASE/artifacts"
mkdir -p "$ARTIFACT_DIR"
ARTIFACT="$ARTIFACT_DIR/demo-e2e-local-$(date +%Y%m%dT%H%M%S).log"

log_phase() {
  local phase="$1" ok="$2"
  shift 2 || true
  local extra="${*:-}"
  if [[ -z "$extra" ]]; then
    extra="{}"
  fi
  PHASE="$phase" OK="$ok" EXTRA="$extra" ARTIFACT="$ARTIFACT" python3 <<'PY' >>"$ARTIFACT"
import json, os
raw = os.environ.get("EXTRA") or "{}"
extra = json.loads(raw)
row = {"phase": os.environ["PHASE"], "ok": os.environ["OK"] == "1", **extra}
print(json.dumps(row, separators=(",", ":")))
PY
}

run_e2e_body() {
  echo "=== Step 17 demo-e2e-local (phase=$E2E_PHASE) ==="
  echo "Artifact: $ARTIFACT"

  ensure_fixture

  if [[ ! -f "$PAYMENT_STREAMS_GUEST_BIN" ]]; then
    echo "--- build guest (make build) ---"
    make build
  fi

  build_and_install

  echo "--- dual-host orchestrator ---"
  python3 "$REPO/scripts/e2e/run_local_e2e.py" \
    --repo "$REPO" \
    --phase "$E2E_PHASE" \
    --artifact "$ARTIFACT"

  echo "=== done ==="
  cat "$ARTIFACT"
}

ensure_fixture() {
  export FIXTURE_MANIFEST
  if [[ "$SKIP_SEED" == "1" && -f "$FIXTURE_MANIFEST" ]]; then
    log_phase seed 1 '{"skipped":true}'
    return 0
  fi
  local need_seed=0
  if [[ ! -f "$FIXTURE_MANIFEST" ]]; then
    need_seed=1
  elif ! curl -sf -X POST http://127.0.0.1:3040 -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getBlockHeight","params":[]}' >/dev/null; then
    need_seed=1
  elif ! "$REPO/scripts/verify-step10a-dod.sh" >/dev/null 2>&1; then
    need_seed=1
  fi
  if [[ "$need_seed" == "1" ]]; then
    echo "--- seed localnet (demo-localnet-fresh) ---"
    SKIP_VERIFY="${SKIP_VERIFY:-0}" "$REPO/scripts/demo-localnet-fresh.sh"
  fi
  if [[ ! -f "$FIXTURE_MANIFEST" ]]; then
    log_phase seed 0 '{"error":"missing manifest"}'
    exit 1
  fi
  FIXTURE_JSON="$(python3 -c "import json,os; print(json.dumps({'manifest': os.environ['FIXTURE_MANIFEST']}))")"
  log_phase seed 1 "$FIXTURE_JSON"
}

install_lgx() {
  local lgx="$1"
  local dest="$2"
  mkdir -p "$dest"
  lgpm --modules-dir "$dest" install --file "$lgx" --force
}

build_and_install() {
  if [[ "$SKIP_BUILD" == "1" ]]; then
    echo "SKIP_BUILD=1 — assuming modules already installed"
    return 0
  fi

  echo "--- build payment_streams_module ---"
  local ps_out
  ps_out="$(nix build "$REPO/logos-payment-streams-module#lgx" -L --no-link --print-out-paths | tail -1)"
  install_lgx "$ps_out"/*.lgx "$MODULES_USER"
  install_lgx "$ps_out"/*.lgx "$MODULES_PROVIDER"

  echo "--- build logos_execution_zone (patched wallet) ---"
  if ! compgen -G "$REPO/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/"*.lgx >/dev/null; then
    "$REPO/scripts/build-wallet-lgx.sh"
  fi
  local wallet_lgx
  wallet_lgx="$(readlink -f "$REPO/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/"*.lgx)"
  install_lgx "$wallet_lgx" "$MODULES_USER"
  install_lgx "$wallet_lgx" "$MODULES_PROVIDER"

  DELIVERY_MODULE_ROOT="${DELIVERY_MODULE_ROOT:-$REPO/../logos-delivery-module}"
  if [[ ! -d "$DELIVERY_MODULE_ROOT" ]]; then
    echo "ERROR: logos-delivery-module not found at DELIVERY_MODULE_ROOT=$DELIVERY_MODULE_ROOT" >&2
    exit 1
  fi
  echo "--- build delivery_module from $DELIVERY_MODULE_ROOT ---"
  local dm_out
  dm_out="$(nix build "$DELIVERY_MODULE_ROOT#packages.x86_64-linux.default" --impure -L --no-link --print-out-paths | tail -1)"
  mkdir -p "$MODULES_USER/delivery_module" "$MODULES_PROVIDER/delivery_module"
  cp -f "$dm_out/lib/delivery_module_plugin.so" "$MODULES_USER/delivery_module/"
  cp -f "$dm_out/lib/liblogosdelivery.so" "$MODULES_USER/delivery_module/"
  cp -f "$dm_out/lib/delivery_module_plugin.so" "$MODULES_PROVIDER/delivery_module/"
  cp -f "$dm_out/lib/liblogosdelivery.so" "$MODULES_PROVIDER/delivery_module/"

  # Nix-cached liblogosdelivery can lag the sibling logos-delivery tree (eligibility
  # JSON / protobuf must match the plugin). Prefer a fresh local build when available.
  LOGOS_DELIVERY_ROOT="${LOGOS_DELIVERY_ROOT:-$REPO/../logos-delivery}"
  if [[ -d "$LOGOS_DELIVERY_ROOT" && -f "$LOGOS_DELIVERY_ROOT/Makefile" ]]; then
    echo "--- overlay liblogosdelivery from $LOGOS_DELIVERY_ROOT (make liblogosdelivery) ---"
    (cd "$LOGOS_DELIVERY_ROOT" && make liblogosdelivery)
    cp -f "$LOGOS_DELIVERY_ROOT/build/liblogosdelivery.so" "$MODULES_USER/delivery_module/"
    cp -f "$LOGOS_DELIVERY_ROOT/build/liblogosdelivery.so" "$MODULES_PROVIDER/delivery_module/"
  fi

  log_phase build 1 "$(python3 -c "import json,os; print(json.dumps({'modules_user': os.environ['MODULES_USER']}))")"
}

chmod +x "$REPO_ROOT/scripts/e2e/"*.py 2>/dev/null || true
chmod +x "$REPO_ROOT/scripts/demo-e2e-local.sh"

# N8 wire + orchestrator need host cargo; tooling nix shell for lgpm/logoscore only.
if [[ -z "${N8_WIRE_HEX:-}" ]]; then
  export N8_WIRE_HEX="$(cargo run -q -p lez-payment-streams-core --bin n8_canonical_wire_hex)"
fi

export ARTIFACT E2E_PHASE SKIP_BUILD SKIP_SEED N8_WIRE_HEX

nix shell \
  github:logos-co/logos-package-manager \
  github:logos-co/logos-logoscore-cli \
  --command bash -c "
    export REPO='$REPO' ARTIFACT='$ARTIFACT' E2E_PHASE='$E2E_PHASE'
    export SKIP_BUILD='$SKIP_BUILD' SKIP_SEED='$SKIP_SEED' N8_WIRE_HEX='$N8_WIRE_HEX'
    export MODULES_USER='$MODULES_USER' MODULES_PROVIDER='$MODULES_PROVIDER'
    export LOGOSCORE_CONFIG_USER='$LOGOSCORE_CONFIG_USER' LOGOSCORE_CONFIG_PROVIDER='$LOGOSCORE_CONFIG_PROVIDER'
    export PERSIST_USER='$PERSIST_USER' PERSIST_PROVIDER='$PERSIST_PROVIDER'
    export FIXTURE_MANIFEST='$FIXTURE_MANIFEST' WALLET_CONFIG='$WALLET_CONFIG' WALLET_STORAGE='$WALLET_STORAGE'
    export PAYMENT_STREAMS_GUEST_BIN='$PAYMENT_STREAMS_GUEST_BIN' E2E_PROVIDER_AD='$E2E_PROVIDER_AD'
    export PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF='${PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF:-0}'
    export DELIVERY_MODULE_ROOT='${DELIVERY_MODULE_ROOT:-$REPO/../logos-delivery-module}'
    export N8_WIRE_HEX='$N8_WIRE_HEX' FIXTURE_MANIFEST='$FIXTURE_MANIFEST'
    $(declare -f run_e2e_body ensure_fixture install_lgx build_and_install log_phase)
    run_e2e_body
  "
