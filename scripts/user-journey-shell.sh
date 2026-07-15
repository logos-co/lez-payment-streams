#!/usr/bin/env bash
# Opens a shell with logoscore and lgpm for docs/journeys/USER_JOURNEY.md.
# Installs lgs via cargo when missing. Host still needs Nix, Docker, Rust, RISC Zero for later steps.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_BIN="${HOME}/.cargo/bin"
export PATH="${CARGO_BIN}:${PATH}"

ensure_lgs() {
  if command -v lgs >/dev/null 2>&1; then
    return 0
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    echo "lgs not on PATH and cargo not found; install Rust (rustup), then re-run." >&2
    exit 1
  fi
  local cache="${XDG_CACHE_HOME:-${HOME}/.cache}/logos-scaffold/scaffold-src"
  if [[ ! -d "${cache}/.git" ]]; then
    mkdir -p "$(dirname "${cache}")"
    echo "Installing lgs from logos-co/scaffold (one-time)..." >&2
    git clone --depth 1 https://github.com/logos-co/scaffold.git "${cache}"
  fi
  cargo install --path "${cache}" --locked --force
  if ! command -v lgs >/dev/null 2>&1; then
    echo "lgs install finished but lgs is still not on PATH (expected ${CARGO_BIN}/lgs)." >&2
    exit 1
  fi
}

ensure_lgs

exec nix shell --accept-flake-config \
  github:logos-co/logos-logoscore-cli/0.2.0 \
  --command bash -c "export PATH=\"${CARGO_BIN}:\${PATH}\"; cd \"${REPO_ROOT}\" && exec bash"
