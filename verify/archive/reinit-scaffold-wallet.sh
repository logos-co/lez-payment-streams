#!/usr/bin/env bash
# Re-create 491-format wallet storage under .scaffold/wallet (see docs/archive/steps/local-chain-fixture.md).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

WALLET_DIR="$REPO_ROOT/.scaffold/wallet"
export LEE_WALLET_HOME_DIR="$WALLET_DIR"
SETUP_PASSWORD="${SCAFFOLD_WALLET_SETUP_PASSWORD:-scaffold-local-dev}"

LEZ_PIN="$(grep -A2 '\[repos.lez\]' scaffold.toml | grep '^pin' | sed 's/.*"\([^"]*\)".*/\1/')"
WALLET_BIN="$HOME/.cache/logos-scaffold/repos/lez/${LEZ_PIN}/target/release/wallet"
if [[ ! -x "$WALLET_BIN" ]]; then
  echo "ERROR: run lgs setup first (missing $WALLET_BIN)" >&2
  exit 1
fi

mkdir -p "$WALLET_DIR"
if [[ ! -f "$WALLET_DIR/wallet_config.json" ]]; then
  echo "ERROR: missing $WALLET_DIR/wallet_config.json — run lgs init && lgs setup" >&2
  exit 1
fi

if [[ -f "$WALLET_DIR/storage.json" ]]; then
  ts="$(date +%Y%m%d-%H%M%S)"
  cp "$WALLET_DIR/storage.json" "$WALLET_DIR/storage.json.bak-${ts}"
  rm -f "$WALLET_DIR/storage.json"
  echo "Backed up previous storage.json to storage.json.bak-${ts}"
fi

echo "Creating new encrypted storage (password from SCAFFOLD_WALLET_SETUP_PASSWORD)…"
printf '%s\n' "$SETUP_PASSWORD" | "$WALLET_BIN" account list >/dev/null

PUBLIC_ID="$("$WALLET_BIN" account list 2>&1 | sed -n 's|^/ Public/\([A-Za-z0-9]*\).*|\1|p' | head -1)"
if [[ -z "$PUBLIC_ID" ]]; then
  echo "ERROR: could not read public account after setup" >&2
  exit 1
fi

lgs wallet default set --address "Public/$PUBLIC_ID" 2>/dev/null || true

rm -f .lez_payment_streams-state .lez_payment_streams-state.tmp .lez_payment_streams-fixture-provider fixtures/localnet.json

echo "Scaffold wallet re-init done."
echo "  LEE_WALLET_HOME_DIR=$WALLET_DIR"
echo "  default public account: $PUBLIC_ID"
echo "Next: ./scripts/seed-localnet-fixture.sh (or make seed-fixture)"
