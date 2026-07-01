# Step 28 — plan excerpt

Active-work packet for agents. Index: [index.md](../index.md).

### Step 28, User Journey on TestNet

Enable User Journey (`MODE=module`) to run on TestNet v0.2, completing
support for both localnet and testnet across both User Journey and
Developer Journey.

Prerequisite: [Step 26 — TestNet v0.2 Migration](../completed/step-26-testnet-v02-migration.md).

#### Scope

User Journey (payment streams protocol only, no Store integration) currently
supports `CHAIN=local`. This step adds `CHAIN=testnet` support to
`scripts/e2e.sh` and supporting infrastructure.

| Journey | LocalNet | TestNet v0.2 |
|---------|----------|--------------|
| User Journey (`MODE=module`) | Supported | **This step** |
| Developer Journey (`MODE=store`) | Supported | Step 26 + Step 27 |

#### Deliver

- `CHAIN=testnet` support for `MODE=module` in `scripts/e2e.sh`
- One-time bootstrap command for module-only testnet users (no `delivery_module` needed)
- Testnet fixture policy for module-only flows (vault, stream, no Store)
- Updated verification matrix: both journeys Required on both chains
- Updated User Journey documentation with testnet commands

#### Bootstrap requirements

Unlike Developer Journey, User Journey does not require `delivery_module`
or sibling delivery repo checkouts. Bootstrap for testnet module-only:

```bash
# One-time (per operator)
make bootstrap-testnet-module
# or integrated into: ./scripts/e2e.sh testnet bootstrap --mode=module
```

This creates `fixtures/testnet-module.json` with:
- `sequencer_addr` (testnet v0.2 endpoint)
- `wallet_config` paths
- `program_id_hex` (from Step 26 org deploy)
- No Store-related fields (no `store_node_multiaddr`, no `provider_account_id`)

#### Verification gates

| Gate | Command | Pass Criteria |
|------|---------|---------------|
| Module smoke | `MODE=module CHAIN=testnet make verify-step28-module-smoke` | Read operations succeed |
| Full module E2E | `MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run` | `vault_init`, `create_stream`, `claim` all pass |
| Non-regression | `MODE=module CHAIN=local ./scripts/e2e.sh local run` | Still passes |

#### Definition of done

- [ ] `scripts/e2e.sh` accepts `MODE=module CHAIN=testnet`
- [ ] `make bootstrap-testnet-module` (or equivalent) creates module-only fixture
- [ ] User Journey E2E passes on testnet v0.2 with all phases
- [ ] `claim` phase verified on testnet (depends on Step 27)
- [ ] Verification matrix updated: both journeys Required on both chains
- [ ] Step 22 (User Journey doc) updated with testnet commands
- [ ] Non-regression: localnet User Journey still passes

#### Related

- [step-26-testnet-v02-migration.md](../completed/step-26-testnet-v02-migration.md) — provides testnet v0.2 base
- [step-27-claim-fix-verification.md](../completed/step-27-claim-fix-verification.md) — claim must work for this step
- [step-22-ui-journey.md](step-22-ui-journey.md) — User Journey doc to update
- [verification-matrix.md](../../reference/verification-matrix.md) — status update
