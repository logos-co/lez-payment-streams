# Build once

```bash
./scripts/e2e.sh build
```

This builds `payment_streams_module`, the patched wallet lgx, and (for Store mode) `delivery_module` + `liblogosdelivery`. Run it once after pulling or after changing module source.

# Run without rebuilding

Pass `SKIP_BUILD=1` so `cmd_prepare` reuses the already-built `.lgx` under `.scaffold/e2e/*/modules`. The four demo options, with on-chain txids printed:

```bash
# User Journey, localnet
SKIP_BUILD=1 MODE=module CHAIN=local ./scripts/e2e.sh local run --verbosity verbose

# User Journey, testnet
SKIP_BUILD=1 MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run --verbosity verbose

# Developer Journey, localnet
SKIP_BUILD=1 E2E_VERBOSITY=verbose MODE=store CHAIN=local ./scripts/e2e.sh local run

# Developer Journey, testnet
SKIP_BUILD=1 E2E_VERBOSITY=verbose MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
```



# Caveats

- `SKIP_BUILD=1` only skips the module build step. Store mode still runs cargo for the N8-wire and seed binaries; those are cargo cache hits after the first build, but recompile if you change core/examples source.
- The guest ZK binary (`methods/guest/target/.../lez_payment_streams.bin`) is not rebuilt by `./scripts/e2e.sh build`; rebuild it separately via `cargo risczero build` if guest source changes.
- After any module source change, drop `SKIP_BUILD=1` (or rerun `./scripts/e2e.sh build`) before the next demo run, otherwise you test stale `.lgx`.
- A `flake.lock` bump or nixpkgs update invalidates the Nix cache and forces rebuilds regardless of `SKIP_BUILD`.

