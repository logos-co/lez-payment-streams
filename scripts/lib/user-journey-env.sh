#!/usr/bin/env bash
# Shared pins and guards for docs/journeys/USER_JOURNEY.md scripts.
[[ -n "${USER_JOURNEY_ENV_SOURCED:-}" ]] && return 0
USER_JOURNEY_ENV_SOURCED=1

user_journey_repo_root() {
  if [[ -n "${REPO_ROOT:-}" ]]; then
    echo "$REPO_ROOT"
  else
    cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd
  fi
}

# Pinned together for linux-amd64-dev module variants (differs from E2E #lgx-portable).
readonly USER_JOURNEY_LOGOSCORE_FLAKE="github:logos-co/logos-logoscore-cli/679a9af8fd0064c2997c2ea3ed1fa53b422bfe0d"
readonly USER_JOURNEY_LGPM_FLAKE="github:logos-co/logos-package-manager/205d6bb295c43e9432aef367dd32dac82e39bddf"

user_journey_require_shell() {
  if [[ "${USER_JOURNEY_SHELL:-}" != 1 ]]; then
    echo "Run this from the journey toolchain shell: ./scripts/user-journey-shell.sh" >&2
    exit 1
  fi
}

user_journey_require_tools() {
  local missing=()
  for cmd in logoscore lgpm nix; do
    command -v "$cmd" >/dev/null 2>&1 || missing+=("$cmd")
  done
  if ((${#missing[@]} > 0)); then
    echo "Missing on PATH: ${missing[*]}. Use ./scripts/user-journey-shell.sh" >&2
    exit 1
  fi
}

user_journey_default_modules_dir() {
  local root
  root="$(user_journey_repo_root)"
  echo "${root}/.scaffold/e2e/user/modules"
}

user_journey_default_wallet_home() {
  local root
  root="$(user_journey_repo_root)"
  echo "${root}/.scaffold/e2e/testnet-wallet"
}
