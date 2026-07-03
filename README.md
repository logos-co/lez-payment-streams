# lez-payment-streams

[LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html) payment streams on the Logos
Execution Zone: SPEL guest program, Logos `payment_streams_module`, and a reference Store
integration (eligibility on paid queries).

Documentation hub: [docs/README.md](docs/README.md). Maintainers: [AGENTS.md](AGENTS.md).

## Prerequisites

Everything below is required to run the verification scripts from this repository (`scripts/e2e.sh`, `make verify-*`).
Journey docs under [docs/journeys/](docs/journeys/) describe demo outcomes; they do not replace this checklist.

Host and toolchain:

* Linux (Ubuntu 22.04+) or macOS 14+
* Nix with flakes enabled
* Rust toolchain with RISC Zero for the guest ELF (`make build` or `cargo risczero build` under `methods/guest/`)
* Logos scaffold CLI (`lgs`) on `PATH` for localnet
* Internet access for Nix flakes (and for testnet runs)

Run verification inside a shell that provides `logoscore` and `lgpm`, for example:

```bash
nix shell --accept-flake-config \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-package-manager \
  --command bash
```

From the repo root (in that shell, with `lgs` available):

```bash
lgs init    # if .scaffold/ is missing
lgs setup   # if scaffold.toml / LEZ layout is missing
```

Store integration (`./scripts/e2e.sh local run`, `make verify-store-local`, testnet Store targets) additionally needs a `logos-delivery-module` checkout beside this repo.
Default path: `../logos-delivery-module` (override with `DELIVERY_MODULE_ROOT`).
E2E does not clone it; prepare builds `delivery_module` from that tree with Nix and fails if the directory is missing.
Use the integration branch recorded in [docs/reference/feature-branch-pins.md](docs/reference/feature-branch-pins.md).

A local `../logos-delivery` checkout is optional.
Nix fetches the locked `logos-delivery` flake input when building the module.
Keep the sibling only when overlaying `liblogosdelivery.so` while editing delivery; set `SKIP_LIBLOGOSDELIVERY_OVERLAY=1` for hermetic installs from the built `.lgx` only.

Module-only verification (`MODE=module`, `make verify-module-local` / `verify-module-testnet`) does not need delivery siblings.

Testnet runs need a one-time fixture bootstrap before the first pass on that machine (`make bootstrap-testnet` for Store, `make bootstrap-testnet-module` for module-only).
See [docs/reference/verification-matrix.md](docs/reference/verification-matrix.md).

Step-by-step cold start, recovery, and artifact expectations:
[verification matrix](docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine).

## Verify

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
./scripts/e2e.sh local run
```

Advanced testnet Store path and artifact expectations:
[docs/reference/verification-matrix.md](docs/reference/verification-matrix.md).

## License

MIT ([LICENSE-MIT](LICENSE-MIT)) or Apache 2.0 ([LICENSE-APACHE-v2](LICENSE-APACHE-v2)).
