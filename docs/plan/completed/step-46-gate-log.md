# Step 46 gate log

Upcoming until D46.11 dogfood legs are recorded.
Packet: [step-46-docs-unify-and-forum-post.md](../upcoming/step-46-docs-unify-and-forum-post.md).

## Docs landed

| Surface | Path |
| --- | --- |
| Front door | [README.md](../../../README.md) (install, Testing handles) |
| Hub | [docs/README.md](../../README.md) |
| Protocol reproduce | [docs/reproduce/payment-streams.md](../../reproduce/payment-streams.md) |
| Store reproduce | [docs/reproduce/store-eligibility.md](../../reproduce/store-eligibility.md) |
| Integrate | [docs/integrate/eligibility.md](../../integrate/eligibility.md) |
| Forum draft | [docs/external/forum-post.md](../../external/forum-post.md) |
| Program identity | README [Public testnet guest program](../../../README.md#public-testnet-guest-program), [fixtures/testnet.json](../../../fixtures/testnet.json), [fixtures/testnet-module.json](../../../fixtures/testnet-module.json) |

`docs/journeys/` and `docs/store-integration/` folded and removed.

## D46.11 dogfood

TODO: wait for results of local-private / testnet-public / testnet-private.

| Handle | Command | Result | Artifact | ImageID |
| --- | --- | --- | --- | --- |
| `local-private` module | `MODE=module OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` (`RISC0_DEV_MODE=1`) | pending | | cite README / fixtures |
| `local-private` store | `MODE=store OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` (`RISC0_DEV_MODE=1`) | pending | | cite README / fixtures |
| `testnet-public` module | `MODE=module ./scripts/e2e.sh testnet run` | pending | | cite README / fixtures |
| `testnet-public` store | `MODE=store ./scripts/e2e.sh testnet run` | pending | | cite README / fixtures |
| `testnet-private` store | `RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 MODE=store OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh testnet run` | pending | | cite README / fixtures |

Forum draft wrap-up claims stay TODO until these rows are filled.

## Terminology

`make check-terminology` after the docs land.
