# Naming conventions

Use this vocabulary consistently in product docs and runbooks.

## Verification flows (matrix)

| Term | Meaning |
| --- | --- |
| Flow A | Module-only verification. `MODE=module` on `scripts/e2e.sh`. Single-host happy path through `payment_streams_module` `chainAction`. No Store, no eligibility APIs in the gate. |
| Flow B | Store integration verification. `MODE=store` (default). Dual-host demo with `delivery_module` and LIP-155 eligibility on Store requests. |

Flow A and Flow B are independent of the N18 demo track names below.

## N18 demo tracks (program steps)

| Term | Meaning |
| --- | --- |
| N18 Track A | Integrator narrative: payment streams composed with Logos Delivery Store (Steps 17, 20). Same as verification Flow B for local/testnet gates. |
| N18 Track B | Optional payment-streams-only Basecamp UI (Steps 21–22). Not part of the verification matrix. |

Step 20 deliverable is N18 Track A (Store integrator journey), not Flow A module-only docs.

## Logos and protocol names

| Term | Use |
| --- | --- |
| payment streams module | Prose description of the Logos plugin. |
| `payment_streams_module` | Runtime module id (`logoscore load-module`, LogosAPI). |
| Store | The Waku/Logos Store protocol (capitalize). |
| `logos-delivery` | Repository implementing Store and liblogosdelivery. |
| `delivery_module` | Logos plugin exposing Delivery/Store to logoscore. |
| `logos_execution_zone` | LEZ wallet Logos module id. |
| LIP-155 | Hyphenated spec name (not "LIP 155"). |

## Makefile targets vs flows

Historical step numbers remain in some Make targets:

| Make target | Matrix cell |
| --- | --- |
| `make verify-module-local` | Flow A × localnet |
| `make verify-step17` | Flow B × localnet |
| `make verify-step17-back-to-back` | Flow B × localnet (deterministic lifecycle; see [step-24c](../plan/completed/step-24c-simplify-demo-flow.md)) |
| `make verify-step18` | Flow B × testnet (advanced) |

See [verification-matrix.md](../verification-matrix.md) and [scripts/README.md](../../scripts/README.md).
