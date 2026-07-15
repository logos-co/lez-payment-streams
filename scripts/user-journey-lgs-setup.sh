#!/usr/bin/env bash
# Step 4 helper: lgs init/setup with LEZ v0.2 wallet debug config fallback.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="${HOME}/.cargo/bin:${PATH}"

if ! command -v lgs >/dev/null 2>&1; then
  echo "lgs not on PATH; run ./scripts/user-journey-shell.sh first." >&2
  exit 1
fi

LEZ_PIN="$(grep -A2 '^\[repos.lez\]' "$REPO_ROOT/scaffold.toml" | sed -n 's/^pin = "\(.*\)"/\1/p')"
SCAFFOLD_LEZ_CACHE="${HOME}/.cache/logos-scaffold/repos/lez/${LEZ_PIN}"
SCAFFOLD_WALLET="${SCAFFOLD_LEZ_CACHE}/target/release/wallet"
WALLET_SCAFFOLD_HOME="$REPO_ROOT/.scaffold/wallet"

cd "$REPO_ROOT"
lgs init

if ! lgs setup; then
  nested_cfg="${SCAFFOLD_LEZ_CACHE}/lez/wallet/configs/debug/wallet_config.json"
  if [[ -f "$nested_cfg" ]]; then
    mkdir -p "$WALLET_SCAFFOLD_HOME"
    cp "$nested_cfg" "$WALLET_SCAFFOLD_HOME/wallet_config.json"
    lgs setup
  else
    echo "lgs setup failed and LEZ wallet debug config not found under $SCAFFOLD_LEZ_CACHE" >&2
    exit 1
  fi
fi

if [[ ! -x "$SCAFFOLD_WALLET" ]]; then
  echo "Expected wallet CLI at $SCAFFOLD_WALLET after lgs setup" >&2
  exit 1
fi

echo "Scaffold wallet CLI: $SCAFFOLD_WALLET"
