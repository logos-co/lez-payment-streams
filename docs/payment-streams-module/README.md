# Payment streams module

Universal Logos module `payment_streams_module` drives LIP-155 vault and stream lifecycle on LEZ
through `logos_execution_zone`. This is a generic payment-stream layer: the stream payee is a LEZ
account, not necessarily a “service provider” on another protocol.

External runbooks cover:

- Installing wallet + module artifacts and guest ELF prerequisites
- Flow A verification (module-only happy path on localnet)
- Manual `logoscore call payment_streams_module chainAction …` sequences matching the automated gate

They do not cover Store, `delivery_module`, or eligibility prepare/verify (see
[Store integration](../store-integration/)).

## Quick links

| Document | Role |
| --- | --- |
| [runbook.md](runbook.md) | Tier 1 and tier 2 verification (Flow A) |
| [setup.md](setup.md) | Build `.lgx`, pins, wallet, guest env |
| [integration contracts](../reference/integration-contracts.md) | `chainAction` and public LogosAPI |
| [Logos architecture overview](../reference/logos-architecture-overview.md) | Hosts and module boundaries |

## Verification (Flow A)

Required gate: `make verify-module-local` (see [verification matrix](../verification-matrix.md)).

Equivalent: `MODE=module CHAIN=local ./scripts/e2e.sh local run`.

Implementation: [scripts/module-e2e-local.sh](../../scripts/module-e2e-local.sh).
