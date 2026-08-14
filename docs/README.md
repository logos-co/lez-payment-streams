# Documentation

Entry for integrators and maintainers.
Protocol text lives in [LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html).
Cite the LIP for semantics.

## Choose a path

| Path | When |
| --- | --- |
| [program/README.md](../program/README.md) | Guest program, Rust tests, review order |
| [module/README.md](../module/README.md#chainaction-catalogue) | chainAction API reference |
| [reproduce/module.md](reproduce/module.md) | Manual LIP-155 via the module (testnet primary) |
| [reproduce/store.md](reproduce/store.md) | Paid Store + eligibility (`e2e.sh`, local primary) |
| [integrate.md](integrate.md) | Eligibility for a request-response protocol |

`plan/` and `archive/` are internal.

## Verify

Testing recipes: [root README Testing](../README.md#testing).
Maintainer flags, artifacts, cold start: [reference/matrix.md](reference/matrix.md).
Scripts: [verify/README.md](../verify/README.md).

## Reference

| Doc | Role |
| --- | --- |
| [wire.md](reference/wire.md) | Cross-repo APIs, tag 30 |
| [decisions.md](reference/decisions.md) | Integration decisions |
| [pins.md](reference/pins.md) | Fork branches and flakes |
| [names.md](reference/names.md) | `MODE` values, Makefile names, [scaffold layout](reference/names.md#scaffold-layout) |
| [matrix.md](reference/matrix.md) | Mode × network matrix |
| [localnet-recovery.md](reference/localnet-recovery.md) | Localnet failure recovery |

## Archive

Historical step runbooks and operator notes: [archive/](archive/).

## Maintainers

[wire.md](reference/wire.md) and [AGENTS.md](../AGENTS.md).
Lifecycle regression: `make verify-store-local-lifecycle`.
