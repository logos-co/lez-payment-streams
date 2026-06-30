# Step 27 — plan excerpt

Active-work packet for agents. Index: [index.md](../index.md).

### Step 27, Claim Fix and Verification

Debug and fix the payment streams `claim` functionality to ensure
both User Journey (payee claim) and Developer Journey (provider claim)
work reliably on both localnet and TestNet v0.2.

Prerequisite: [Step 26 — TestNet v0.2 Migration](step-26-testnet-v02-migration.md)
(declared complete for verification purposes).

#### Background

Previous testnet integration (Step 18) marked `claim` as optional
("may be optional on testnet" per `testnet-claim-known-issue.md`).
This step elevates `claim` to required functionality for both journeys
and all deployment targets.

#### Investigation scope

| Scenario | Actor | Chain | Expected |
|----------|-------|-------|----------|
| User Journey | Payee (stream recipient) | localnet | `chainAction claim` succeeds, balance increases |
| User Journey | Payee | testnet v0.2 | `chainAction claim` succeeds, balance increases |
| Developer Journey | Provider (paid Store host) | localnet | `claim` succeeds after `store_query_success` |
| Developer Journey | Provider | testnet v0.2 | `claim` succeeds after serving paid query |

#### Deliver

- Root cause analysis of prior claim failures (documented)
- Fix implementation in `lez-payment-streams-core`, guest, or module
- Verification on localnet (both journeys)
- Verification on TestNet v0.2 (both journeys)
- Updated E2E artifacts showing `claim` phase succeeding

#### Definition of done

- [ ] Claim issue root cause identified and documented
- [ ] Fix implemented and tested on localnet
- [ ] `MODE=module CHAIN=local` E2E shows `{"phase":"claim","ok":true}`
- [ ] `MODE=store CHAIN=local` E2E shows provider claim success
- [ ] TestNet v0.2 claim verified for User Journey (payee)
- [ ] TestNet v0.2 claim verified for Developer Journey (provider)
- [ ] `archive/operator/testnet-claim-known-issue.md` updated or retired
- [ ] User Journey documentation includes payee claim example
- [ ] Developer Journey documentation includes provider claim example

#### Verification commands

```bash
# User Journey — localnet
MODE=module CHAIN=local ./scripts/e2e.sh local run

# Developer Journey — localnet
MODE=store CHAIN=local ./scripts/e2e.sh local run

# User Journey — testnet (requires Step 28 for full support)
# Developer Journey — testnet
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
```

All commands must show successful `claim` phase in artifacts.

#### Non-regression

- Vault creation, deposit, stream open/close remain functional
- Store query eligibility verification unchanged
- Localnet paths (`make verify-step17`) continue passing

#### Related

- [step-26-testnet-v02-migration.md](step-26-testnet-v02-migration.md) — provides testnet v0.2 target
- [step-28-user-journey-testnet.md](step-28-user-journey-testnet.md) — enables full User Journey on testnet
- [archive/operator/testnet-claim-known-issue.md](../../archive/operator/testnet-claim-known-issue.md) — prior issue documentation
- [USER_JOURNEY.md](../../../../USER_JOURNEY.md) — to be updated with payee claim
- [DEVELOPER_JOURNEY.md](../../../../DEVELOPER_JOURNEY.md) — to be updated with provider claim
