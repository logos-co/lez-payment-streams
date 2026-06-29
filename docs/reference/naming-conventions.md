# Naming conventions

Use this vocabulary consistently in product docs and runbooks.

## External product names

| Term | Meaning |
| --- | --- |
| Module verification | Single-host `payment_streams_module` happy path (`MODE=module`). |
| Store integration | Dual-host Store demo with eligibility (`MODE=store`, default). |

## Verification flows (`MODE`)

| Term | Meaning |
| --- | --- |
| Flow A | `MODE=module` on `scripts/e2e.sh`. No Store, no eligibility in the gate. |
| Flow B | `MODE=store` (default). Dual-host demo with `delivery_module` and LIP-155 on Store. |

Flow A/B are independent of N18 demo track names (development-map only).

## N18 demo tracks (development-map)

| Term | Meaning |
| --- | --- |
| N18 Track A | Integrator narrative: Store + payment streams (developer journey, Store pillars). |
| N18 Track B | Optional payment-streams-only Basecamp UI (not in verification matrix). |

## Logos and protocol names

| Term | Use |
| --- | --- |
| payment streams module | Prose description of the Logos plugin. |
| `payment_streams_module` | Runtime module id. |
| Store | Waku/Logos Store protocol (capitalize). |
| `logos-delivery` | Repository for Store and liblogosdelivery. |
| `delivery_module` | Logos plugin for Delivery/Store. |
| `logos_execution_zone` | LEZ wallet Logos module id. |
| LIP-155 | Hyphenated spec name. |

## Makefile targets

Primary (step-free):

| Make target | Matrix cell |
| --- | --- |
| `make verify-module-local` | Flow A × localnet |
| `make verify-store-local` | Flow B × localnet |
| `make verify-store-testnet` | Flow B × testnet (advanced) |
| `make verify-store-local-lifecycle` | Maintainer only (two runs, one ledger) |

Legacy aliases: `verify-step17`, `verify-step18`, `verify-step17-back-to-back`.

Canonical commands: [verification-matrix.md](verification-matrix.md),
[scripts/README.md](../../scripts/README.md).
