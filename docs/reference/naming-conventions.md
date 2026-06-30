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
| User Journey | `MODE=module` on `scripts/e2e.sh`. Payment streams in isolation (no Store, no eligibility gate). |
| Developer Journey | `MODE=store` (default). Dual-host demo with `delivery_module` and LIP-155 eligibility on Store. |

Makefile targets use the Journey names directly where applicable.

## N18 demo tracks (plan index)

| Term | Meaning |
| --- | --- |
| N18 | Developer Journey vs User Journey: Store integration (Step 20) vs payment-streams-only UI (Steps 21–22). |

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
| `make verify-module-local` | User Journey × localnet |
| `make verify-store-local` | Developer Journey × localnet |
| `make verify-store-testnet` | Developer Journey × testnet (advanced) |
| `make verify-store-local-lifecycle` | Maintainer only (two runs, one ledger) |

Legacy aliases: `verify-step17`, `verify-step18`, `verify-step17-back-to-back`.

Canonical commands: [verification-matrix.md](verification-matrix.md),
[scripts/README.md](../../scripts/README.md).
