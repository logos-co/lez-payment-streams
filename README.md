# lez-payment-streams

LIP-155 payment streams on the Logos Execution Zone (LEZ): an on-chain SPEL program, a Logos
`payment_streams_module` for vault and stream lifecycle, and a reference integration that uses
payment streams as Store eligibility (RFC 73 wire pattern).

Protocol specification: [LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html).

## Documentation map

| Pillar | Topic |
| --- | --- |
| [On-chain program](docs/on-chain/) | Guest, `lez-payment-streams-core`, review guide |
| [Payment streams module](docs/payment-streams-module/) | Logos module + wallet; generic stream payments |
| [Store integration](docs/store-integration/) | `delivery_module`, eligibility, dual-host demo |
| [Development map](docs/development-map/) | Step program, historical runbooks, plan packets |

Reference: [integration contracts](docs/reference/integration-contracts.md),
[decisions and notes](docs/reference/decisions-and-notes.md),
[Logos architecture overview](docs/reference/logos-architecture-overview.md),
[naming conventions](docs/reference/naming-conventions.md).

Agents: [AGENTS.md](AGENTS.md).

## Verification

Supported gates are documented in [docs/verification-matrix.md](docs/verification-matrix.md)
(script stack: [scripts/README.md](scripts/README.md)).

| Tier | Flow | Chain | Make target |
| --- | --- | --- | --- |
| Required | Module only (Flow A) | Localnet | `make verify-module-local` |
| Required | Store integration (Flow B) | Localnet | `make verify-step17-back-to-back` |
| Required | Store integration (Flow B) | Localnet | `make verify-step17` |
| Advanced | Store integration (Flow B) | Testnet | `make verify-step18` |
| Future | Module only (Flow A) | Testnet | unsupported |

On-chain unit tests (no logoscore): `RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core --lib`
after rebuilding the guest when needed.

## In this repository

| Path | Role |
| --- | --- |
| `methods/guest/`, `lez-payment-streams-core/` | LIP-155 guest and shared logic |
| `logos-payment-streams-module/` | Universal Logos module (`.lgx`) |
| `scripts/e2e.sh`, `scripts/module-e2e-local.sh` | Verification automation |
| `scripts/e2e/run_local_e2e.py` | Flow B dual-host orchestrator |

Store wire and hooks ship on sibling repos `logos-delivery` and `logos-delivery-module`
(branch `feat/payment-streams-store-eligibility`; see [feature-branch-pins.md](docs/feature-branch-pins.md)).

## Build (on-chain)

```bash
cargo risczero build --manifest-path methods/guest/Cargo.toml
RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core --lib
```

Logos module package:

```bash
nix build ./logos-payment-streams-module#lgx
```

Deploy and CLI helpers: `make build`, `make idl`, `make deploy` (local LEZ wallet; see
[setup](docs/payment-streams-module/setup.md)).

## License

Licensed under either of MIT ([LICENSE-MIT](LICENSE-MIT)) or Apache 2.0 ([LICENSE-APACHE-v2](LICENSE-APACHE-v2)) at your option.
