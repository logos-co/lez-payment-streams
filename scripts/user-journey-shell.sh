#!/usr/bin/env bash
# Toolchain shell for docs/journeys/USER_JOURNEY.md (pinned logoscore + lgpm, auto lgs).
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/user-journey-env.sh
source "$REPO_ROOT/scripts/lib/user-journey-env.sh"

CARGO_BIN="${HOME}/.cargo/bin"
export PATH="${CARGO_BIN}:${PATH}"

patch_scaffold_wallet_config_path() {
  local cache="$1"
  local state_rs="${cache}/src/state.rs"
  [[ -f "$state_rs" ]] || return 0
  if grep -q 'lez/wallet/configs/debug/wallet_config.json' "$state_rs"; then
    return 0
  fi
  if ! grep -q 'wallet/configs/debug/wallet_config.json' "$state_rs"; then
    return 0
  fi
  echo "Patching scaffold checkout for LEZ v0.2 wallet config path..." >&2
  python3 <<'PY' "$state_rs"
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
text = path.read_text()
needle = '        let cfg_src = lez_repo.join("wallet/configs/debug/wallet_config.json");\n        if !cfg_src.exists() {\n            bail!("missing wallet debug config in lez repo");\n        }\n'
if needle not in text:
    sys.exit(0)
replacement = '''        let cfg_src = ["lez/wallet/configs/debug/wallet_config.json", "wallet/configs/debug/wallet_config.json"]
            .into_iter()
            .map(|rel| lez_repo.join(rel))
            .find(|p| p.exists())
            .ok_or_else(|| anyhow!("missing wallet debug config in lez repo"))?;
'''
text = text.replace(needle, replacement)
path.write_text(text)
PY
}

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
  patch_scaffold_wallet_config_path "${cache}"
  cargo install --path "${cache}" --locked --force
  if ! command -v lgs >/dev/null 2>&1; then
    echo "lgs install finished but lgs is still not on PATH (expected ${CARGO_BIN}/lgs)." >&2
    exit 1
  fi
}

ensure_lgs

exec nix shell --accept-flake-config \
  "${USER_JOURNEY_LOGOSCORE_FLAKE}" \
  "${USER_JOURNEY_LGPM_FLAKE}" \
  --command bash -c "export USER_JOURNEY_SHELL=1; export PATH=\"${CARGO_BIN}:\${PATH}\"; cd \"${REPO_ROOT}\" && echo 'USER_JOURNEY shell: logoscore + lgpm (dev module variants). Optional reset: ./scripts/user-journey-reset.sh' && exec bash"
