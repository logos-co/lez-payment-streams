#!/usr/bin/env bash
# Wrapper for LEZ v0.2.1 cargo-installed wallet (nix-linked build needs runtime libs).
set -euo pipefail
PCSC="${PS_WALLET_PCSC_LIB:-/nix/store/ggnjxpakmyi3md87316vj0jg3lfib92n-pcsclite-2.4.1-lib/lib}"
GCCLIB="${PS_WALLET_GCC_LIB:-/nix/store/w2nk3f7prwzpm6h13rfxh8lh46yfdysj-gcc-15.3.0-lib/lib}"
GCC_S="${PS_WALLET_GCC_S_LIB:-/nix/store/hmi4qvjrcdvzsing2l56ybcwb6hkmf6y-gcc-15.3.0-libgcc/lib}"
GLIBC="${PS_WALLET_GLIBC_LIB:-/nix/store/fjkx1l5cnskzrqacf08z7i8z17256w0j-glibc-2.42-61/lib}"
WALLET_BIN="${PS_WALLET_V021_BIN:-$HOME/.cargo/bin/wallet}"
exec env LD_LIBRARY_PATH="$PCSC:$GCCLIB:$GCC_S:$GLIBC${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
  "$WALLET_BIN" "$@"
