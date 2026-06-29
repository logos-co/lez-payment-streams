# Store integration

Reference integration: LIP-155 payment streams used as **eligibility** for a client–server
protocol. The worked example is Logos **Store** (RFC 73 pattern: proof on request, status on
response; LIP-155 bytes on Store tag `30`).

Hierarchy:

- **Store** — messaging protocol (historical Waku/Logos Store query semantics)
- **`logos-delivery`** — repository with Store codec and `liblogosdelivery` hooks (Steps 14–15)
- **`delivery_module`** — Logos plugin exposing relay/Store and `storeQuery` (Step 16)

This pillar does not define payment streams on-chain semantics (see [on-chain](../on-chain/)) or
module-only lifecycle (see [payment streams module](../payment-streams-module/)).

## Components

| Piece | Role |
| --- | --- |
| `payment_streams_module` | Prepare/verify eligibility, chain actions, wallet I/O |
| `delivery_module` | P2P node, paid `storeQuery`, eligibility hook registration |
| `logos_execution_zone` | LEZ wallet |
| `scripts/e2e/run_local_e2e.py` | Dual-host orchestrator (Flow B) |

Sibling repos on branch `feat/payment-streams-store-eligibility`:
[feature-branch-pins.md](../feature-branch-pins.md),
[program index](../development-map/program-index.md#delivery-integration-branches).

Wire and LogosAPI: [integration contracts](../reference/integration-contracts.md).

## Verification (Flow B)

| Tier | Doc |
| --- | --- |
| Required localnet | [runbook-localnet.md](runbook-localnet.md) |
| Advanced testnet | [runbook-testnet.md](runbook-testnet.md) |
| Matrix summary | [verification-matrix.md](../verification-matrix.md) |

## Recovery

[demo-localnet-recovery.md](../demo-localnet-recovery.md)

## Step 20

Integrator journey (N18 Track A) targets Flow B localnet gates; packet:
[step-20-developer-journey.md](../plan/upcoming/step-20-developer-journey.md).
