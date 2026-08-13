# lez-payment-streams

[LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html) payment streams on the Logos Execution Zone.
SPEL guest program, Logos `payment_streams_module`, and a reference Store integration (eligibility on paid queries).

Documentation: [docs/README.md](docs/README.md).
Maintainers: [AGENTS.md](AGENTS.md).
`chainAction` catalogue: [docs/payment-streams-module/README.md#chainaction-catalogue](docs/payment-streams-module/README.md#chainaction-catalogue).

Reproduce:

- Protocol (manual, testnet primary): [docs/reproduce/payment-streams.md](docs/reproduce/payment-streams.md)
- Store eligibility (orchestrator, local primary): [docs/reproduce/store-eligibility.md](docs/reproduce/store-eligibility.md)
- Eligibility for other protocols: [docs/integrate/eligibility.md](docs/integrate/eligibility.md)

## Public testnet guest program

The payment-streams guest on public testnet (`https://testnet.lez.logos.co/`) was last deployed on 2026-07-22 from freeze commit `c1f5b605705a8d8d2030d2c547ec7b9b9e77236a` (tree includes clock normalization after `6772238b`). Pinned `methods/guest/Cargo.lock` for Docker guest-builder rustc 1.88.
ImageID / `program_id_hex`: `dea010d9cb75887e8350f3dbd45b0efb8517e822fa105bc3e7b9fa2c9a2908ba` (release ELF 366868 bytes).
SSOT fixture: [fixtures/testnet-module.json](fixtures/testnet-module.json).
After changing guest source or release profile, rebuild (`make build`), redeploy (`make deploy-testnet`), refresh fixtures and bootstrap helpers, then re-run module testnet E2E.

## Prerequisites

Required to run `scripts/e2e.sh` and `make verify-*`.
Reproduce docs describe demo outcomes.
This checklist is the install path.

Host and toolchain:

- Linux (Ubuntu 22.04+) or macOS 14+
- Nix with flakes enabled
- Rust toolchain with RISC Zero for the guest ELF (`make build` or `cargo risczero build` under `methods/guest/`)
- Logos scaffold CLI (`lgs`) on `PATH`. Manual protocol path: [payment-streams.md](docs/reproduce/payment-streams.md)
- Internet access for Nix flakes (and for testnet runs)

Run verification inside a shell that provides `logoscore` and `lgpm`:

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

Store integration (`./scripts/e2e.sh local run`, `make verify-store-local`, testnet Store targets) needs a `logos-delivery-module` checkout beside this repo.
Default path: `../logos-delivery-module` (override with `DELIVERY_MODULE_ROOT`).
E2E does not clone it.
Prepare builds `delivery_module` from that tree with Nix.
Use the integration branch in [docs/reference/feature-branch-pins.md](docs/reference/feature-branch-pins.md).

A local `../logos-delivery` checkout is optional.
Nix fetches the locked `logos-delivery` flake input when building the module.
Keep the sibling when overlaying `liblogosdelivery.so` while editing delivery.
Set `SKIP_LIBLOGOSDELIVERY_OVERLAY=1` for hermetic installs from the built `.lgx` only.

Module verification (`MODE=module`, `make verify-module-local` / `verify-module-testnet`) skips delivery siblings.

Testnet runs need a one-time fixture bootstrap (`make bootstrap-testnet` for Store, `make bootstrap-testnet-module` for module-only).
See [docs/reference/verification-matrix.md](docs/reference/verification-matrix.md).

Cold start, recovery, artifacts: [verification matrix](docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine).

## Testing

Handles for README, reproduce docs, and the forum draft.

| Handle | Purpose | Commands |
| --- | --- | --- |
| `fast` | No chain | `cargo clippy --workspace`. `RISC0_DEV_MODE=1 cargo test --workspace`. `make check-terminology`. |
| `local-public` | Local E2E, public accounts | `MODE=module ./scripts/e2e.sh local run`. `MODE=store ./scripts/e2e.sh local run`. |
| `local-private` | Local privacy, stub receipts (`RISC0_DEV_MODE=1` default) | Same with `OWNER_PRIVACY=1 PROVIDER_PRIVACY=1`. |
| `testnet-public` | Public testnet | `MODE=module ./scripts/e2e.sh testnet run`. `MODE=store ./scripts/e2e.sh testnet run`. |
| `testnet-private` | Testnet full privacy, real proving | `RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0` plus full privacy flags. Wrap-up dogfood leg. |

`./scripts/e2e.sh local` and `./scripts/e2e.sh testnet` set the network.
Flag detail and artifacts: [verification-matrix.md](docs/reference/verification-matrix.md).
Orchestrated recipes: [docs/reproduce/store-eligibility.md](docs/reproduce/store-eligibility.md).

## License

MIT ([LICENSE-MIT](LICENSE-MIT)) or Apache 2.0 ([LICENSE-APACHE-v2](LICENSE-APACHE-v2)).
