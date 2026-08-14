# lez-payment-streams

[LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html) payment streams on the Logos Execution Zone.
SPEL guest program, Logos `payment_streams_module`, and a reference Store integration (eligibility on paid queries).
FFI lives in `module/ffi/`.

Documentation: [docs/README.md](docs/README.md).
Maintainers: [AGENTS.md](AGENTS.md).
`chainAction` catalogue: [module/README.md#chainaction-catalogue](module/README.md#chainaction-catalogue).

Reproduce:

- Protocol (manual, testnet primary): [docs/reproduce/module.md](docs/reproduce/module.md)
- Store eligibility (orchestrator, local primary): [docs/reproduce/store.md](docs/reproduce/store.md)
- Eligibility for other protocols: [docs/integrate.md](docs/integrate.md)

## Public testnet guest program

The payment-streams guest on public testnet (`https://testnet.lez.logos.co/`) was last deployed on 2026-08-13 from freeze commit `8a0e374a7e7171cd5b60ad20d46b9510b057dfe3` (Step 49 native `token_id` / holding PDA). Pinned `program/methods/guest/Cargo.lock` for Docker guest-builder rustc 1.88.
ImageID / `program_id_hex`: `c30781ea9d7cc7b3be36f459ce9094644b984224d3d3119a644bb1b21ba2982a` (release ELF 373916 bytes).
Deploy transaction `229dddd92e5184f4a44816ddda711b1eac51476248620a686807e091ffefba8b` (block 5873).
SSOT fixture: [verify/fixtures/testnet-module.json](verify/fixtures/testnet-module.json).
After changing guest source or release profile, rebuild (`make build`), redeploy (`make deploy-testnet`), refresh fixtures and bootstrap helpers, then re-run module testnet E2E.

## Prerequisites

Required to run `verify/e2e.sh` and `make verify-*`.
Reproduce docs describe demo outcomes.
This checklist is the install path.

Host and toolchain:

- Linux (Ubuntu 22.04+) or macOS 14+
- Nix with flakes enabled
- Rust toolchain with RISC Zero for the guest ELF (`make build` or `cargo risczero build` under `program/methods/guest/`)
- Logos scaffold CLI (`lgs`) on `PATH`. Manual protocol path: [payment-streams.md](docs/reproduce/module.md)
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

Store integration (`./verify/e2e.sh local run`, `make verify-store-local`, testnet Store targets) needs a `logos-delivery-module` checkout beside this repo.
Default path: `../logos-delivery-module` (override with `DELIVERY_MODULE_ROOT`).
E2E does not clone it.
Prepare builds `delivery_module` from that tree with Nix.
Use the integration branch in [docs/reference/pins.md](docs/reference/pins.md).

A local `../logos-delivery` checkout is optional.
Nix fetches the locked `logos-delivery` flake input when building the module.
Keep the sibling when overlaying `liblogosdelivery.so` while editing delivery.
Set `SKIP_LIBLOGOSDELIVERY_OVERLAY=1` for hermetic installs from the built `.lgx` only.

Module verification (`MODE=module`, `make verify-module-local` / `verify-module-testnet`) skips delivery siblings.

Testnet runs need a one-time fixture bootstrap (`make bootstrap-testnet` for Store, `make bootstrap-testnet-module` for module-only).
See [docs/reference/matrix.md](docs/reference/matrix.md).

Cold start, recovery, artifacts: [verification matrix](docs/reference/matrix.md#cold-start-first-time-on-a-machine).

## Testing

Handles for README, reproduce docs, and the forum draft.

| Handle | Purpose | Commands |
| --- | --- | --- |
| `fast` | No chain | `cargo clippy --workspace`. `RISC0_DEV_MODE=1 cargo test --workspace`. `make check-terminology`. |
| `local-public` | Local E2E, public accounts | `MODE=module ./verify/e2e.sh local run`. `MODE=store ./verify/e2e.sh local run`. |
| `local-private` | Local privacy, stub receipts (`RISC0_DEV_MODE=1` default) | Same with `OWNER_PRIVACY=1 PROVIDER_PRIVACY=1`. |
| `testnet-public` | Public testnet | `MODE=module ./verify/e2e.sh testnet run`. `MODE=store ./verify/e2e.sh testnet run`. |
| `testnet-private` | Testnet full privacy, real proving | `RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0` plus full privacy flags. Step 52 wrap-up dogfood leg. |

`./verify/e2e.sh local` and `./verify/e2e.sh testnet` set the network.
Flag detail and artifacts: [verification-matrix.md](docs/reference/matrix.md).
Orchestrated recipes: [docs/reproduce/store.md](docs/reproduce/store.md).

## License

MIT ([LICENSE-MIT](LICENSE-MIT)) or Apache 2.0 ([LICENSE-APACHE-v2](LICENSE-APACHE-v2)).
