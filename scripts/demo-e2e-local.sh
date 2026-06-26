#!/usr/bin/env bash
# Step 17 — local dual-host paid Store E2E (see docs/step17-e2e-local.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export REPO="${REPO:-$REPO_ROOT}"
CHAIN="${CHAIN:-local}"
export CHAIN

if [[ "$CHAIN" == "testnet" ]]; then
  export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$REPO/fixtures/testnet.json}"
  export LEZ_TESTNET_WALLET_CONFIG="${LEZ_TESTNET_WALLET_CONFIG:-$REPO/.scaffold/e2e/testnet-wallet/wallet_config.json}"
  export LEZ_TESTNET_WALLET_STORAGE="${LEZ_TESTNET_WALLET_STORAGE:-$REPO/.scaffold/e2e/testnet-wallet/storage.json}"
  # shellcheck source=scripts/testnet-common.sh
  source "$REPO/scripts/testnet-common.sh"
  export WALLET_CONFIG="${WALLET_CONFIG:-$(patch_510_wallet_config_for_testnet)}"
  export WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
  if [[ -n "${LEZ_TESTNET_SUBMIT:-}" ]]; then
    export PATH="$(dirname "$LEZ_TESTNET_SUBMIT"):$PATH"
  elif [[ -x "$REPO/tools/lez-testnet-submit/target/release/lez-testnet-submit" ]]; then
    export LEZ_TESTNET_SUBMIT="$REPO/tools/lez-testnet-submit/target/release/lez-testnet-submit"
    export PATH="$REPO/tools/lez-testnet-submit/target/release:$PATH"
  fi
  export TESTNET_AUTH_TRANSFER_ELF_PATH="${TESTNET_AUTH_TRANSFER_ELF_PATH:-$HOME/.cache/logos-scaffold/repos/lez/62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60/artifacts/program_methods/authenticated_transfer.bin}"
  if [[ -n "${LEZ_TESTNET_SUBMIT:-}" && -x "$LEZ_TESTNET_SUBMIT" ]]; then
    RC3_AUTH_ELF_FILE="${RC3_AUTH_TRANSFER_ELF_PATH:-$REPO/.scaffold/e2e/rc3-auth-transfer-elf.hex}"
    mkdir -p "$(dirname "$RC3_AUTH_ELF_FILE")"
    "$LEZ_TESTNET_SUBMIT" auth-transfer-elf-hex >"$RC3_AUTH_ELF_FILE"
    export RC3_AUTH_TRANSFER_ELF_PATH="$RC3_AUTH_ELF_FILE"
  fi
  export PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF="${PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF:-1}"
else
  export FIXTURE_MANIFEST="${FIXTURE_MANIFEST:-$REPO/fixtures/localnet.json}"
  export WALLET_CONFIG="${WALLET_CONFIG:-$REPO/.scaffold/wallet/wallet_config.json}"
  export WALLET_STORAGE="${WALLET_STORAGE:-$REPO/.scaffold/wallet/storage.json}"
fi

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
    --artifact "$ARTIFACT" || exit 1

  echo "=== done ==="
  cat "$ARTIFACT"
}

ensure_fixture() {
  export FIXTURE_MANIFEST
  if [[ "$CHAIN" == "testnet" ]]; then
    if [[ ! -f "$FIXTURE_MANIFEST" ]]; then
      echo "ERROR: testnet fixture missing: $FIXTURE_MANIFEST (Part B bootstrap)" >&2
      log_phase seed 0 '{"error":"missing testnet manifest"}'
      exit 1
    fi
    log_phase seed 1 '{"skipped":"testnet_no_localnet"}'
    return 0
  fi
  if [[ "$SKIP_SEED" == "1" && -f "$FIXTURE_MANIFEST" ]]; then
    log_phase seed 1 '{"skipped":true}'
    return 0
  fi
  echo "--- prepare localnet (Step 17b restore + stream) ---"
  export E2E_LATE_STREAM_CREATE="${E2E_LATE_STREAM_CREATE:-1}"
  skip_stream=0
  if [[ "$E2E_LATE_STREAM_CREATE" == "1" ]]; then
    skip_stream=1
  fi
  FULL_RESET="${FULL_RESET:-0}" SKIP_VERIFY="${SKIP_VERIFY:-1}" SKIP_STREAM_CREATE="$skip_stream" \
    "$REPO/scripts/demo-localnet-prepare.sh"
  if [[ "$E2E_LATE_STREAM_CREATE" == "1" ]]; then
    echo "--- fixture manifest stub (stream on chain at proof time) ---"
    # shellcheck disable=SC1090
    source "$REPO/.lez_payment_streams-state"
    PROVIDER="$(cat "$REPO/.lez_payment_streams-fixture-provider")"
    cargo run --quiet --manifest-path "$REPO/examples/Cargo.toml" --bin seed_localnet_fixture -- write-manifest \
      --program-bin "$PAYMENT_STREAMS_GUEST_BIN" \
      --owner "$SIGNER_ID" \
      --provider "$PROVIDER" \
      --output "$FIXTURE_MANIFEST"
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
  local dm_lgx_out
  dm_lgx_out="$(nix build "$DELIVERY_MODULE_ROOT#lgx" -L --no-link --print-out-paths | tail -1)"
  install_lgx "$dm_lgx_out"/*.lgx "$MODULES_USER"
  install_lgx "$dm_lgx_out"/*.lgx "$MODULES_PROVIDER"

  # Nix-cached liblogosdelivery can lag the sibling logos-delivery tree (eligibility
  # JSON / protobuf must match the plugin). Prefer a fresh local build when available.
  if [[ "${SKIP_LIBLOGOSDELIVERY_OVERLAY:-0}" == "1" ]]; then
    echo "SKIP_LIBLOGOSDELIVERY_OVERLAY=1 — using liblogosdelivery.so from delivery .lgx"
  else
  LOGOS_DELIVERY_ROOT="${LOGOS_DELIVERY_ROOT:-$REPO/../logos-delivery}"
  if [[ -d "$LOGOS_DELIVERY_ROOT" && -f "$LOGOS_DELIVERY_ROOT/Makefile" ]]; then
    echo "--- overlay liblogosdelivery from $LOGOS_DELIVERY_ROOT (make liblogosdelivery) ---"
    (cd "$LOGOS_DELIVERY_ROOT" && make liblogosdelivery)
    cp -f "$LOGOS_DELIVERY_ROOT/build/liblogosdelivery.so" "$MODULES_USER/delivery_module/"
    cp -f "$LOGOS_DELIVERY_ROOT/build/liblogosdelivery.so" "$MODULES_PROVIDER/delivery_module/"
  fi
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
    export LEZ_TESTNET_WALLET_CONFIG='${LEZ_TESTNET_WALLET_CONFIG:-}'
    export LEZ_TESTNET_WALLET_STORAGE='${LEZ_TESTNET_WALLET_STORAGE:-}'
    export LEZ_TESTNET_SUBMIT='${LEZ_TESTNET_SUBMIT:-}'
    export PAYMENT_STREAMS_GUEST_BIN='$PAYMENT_STREAMS_GUEST_BIN' E2E_PROVIDER_AD='$E2E_PROVIDER_AD'
    export CHAIN='$CHAIN'
    export PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF='${PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF:-0}'
    export E2E_LATE_STREAM_CREATE='${E2E_LATE_STREAM_CREATE:-1}'
    export DELIVERY_MODULE_ROOT='${DELIVERY_MODULE_ROOT:-$REPO/../logos-delivery-module}'
    export SKIP_LIBLOGOSDELIVERY_OVERLAY='${SKIP_LIBLOGOSDELIVERY_OVERLAY:-0}'
    export N8_WIRE_HEX='$N8_WIRE_HEX' FIXTURE_MANIFEST='$FIXTURE_MANIFEST'
    $(declare -f run_e2e_body ensure_fixture install_lgx build_and_install log_phase)
    run_e2e_body
  "
