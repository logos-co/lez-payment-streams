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

#### Root cause analysis (Step 26 verification)

Step 26 verification surfaced the claim defect as the sole blocker for
Store-mode end-to-end. The failure is a runtime invariant violation
introduced by LEZ v0.2.0's stricter account-ownership enforcement, not a
logic error in the payment-streams program itself.

Symptom.
`MODE=store CHAIN=local ./scripts/e2e.sh local run` fails at the
`deposit` phase with:

```
ProgramExecutionFailed("Guest panicked: Sender has insufficient balance")
```

The same phase passes under `MODE=module`, because the module-mode
prefund path routes value through an account the guest program owns.

What changed in v0.2.0.
Three upstream changes combine to produce the failure:

1. `program_owner` is now enforced at execution time. A program may
   only decrease the balance of an account whose `program_owner` field
   is set to that program's ID. Any debit on a non-owned account panics
   with "Sender has insufficient balance" regardless of the account's
   actual balance.
2. System programs (`authenticated_transfer`, `clock`) moved out of the
   `lee` crate into the new `programs` and `clock_core` crates and are
   no longer auto-registered on `V03State`. Callers must register them
   explicitly.
3. Genesis/account initialization semantics tightened: accounts created
   without an explicit `program_owner` default to a value that does not
   authorize the guest program to debit them.

Why deposit fails in Store mode.
`deposit` chains a guest-program call into `authenticated_transfer` to
move funds from the sender into the vault. In Store mode the sender
account is prefunded by the standalone `wallet` flow; that account's
`program_owner` is not the payment-streams program. When the guest
program invokes `authenticated_transfer` to debit the sender, v0.2.0's
enforcer rejects the debit and the guest panics. In Module mode the
prefund path routes value through an account the guest program owns, so
the same chained call succeeds.

Does this contradict the program's semantics?
No. The payment-streams program's intent is unchanged: the sender
authorizes the program to move a specific amount into the vault. What
changed is the strictness of enforcement: v0.2.0 requires that
authorization to be materialized as `program_owner` on the debited
account (or as an explicit claim step), where rc5 inferred it from
balance alone.

Possible fixes (in order of preference).

1. Set `program_owner` on the sender/owner account at prefund or
   genesis. The Store-mode prefund flow (`scripts/fixture.sh` /
   `wallet` calls) must initialize the sender account with
   `program_owner = <payment_streams_program_id>` so the chained
   `authenticated_transfer` debit is authorized. This is the smallest
   change and mirrors how `test_helpers.rs` already constructs owned
   genesis accounts for the unit tests.
2. Have the guest program claim/authorize the owner account before the
   chained transfer. If prefund cannot set `program_owner` (e.g., the
   standalone `wallet` CLI does not expose it), the guest program must
   issue an authorization step that establishes ownership before
   debiting. This is more invasive but keeps prefund unchanged.
3. Bypass `authenticated_transfer` for the vault deposit and debit the
   sender account directly inside the guest program, which already owns
   the vault account. This avoids the chained-call enforcement path
   entirely but changes the program's fund-flow shape and must be
   reconciled with the User Journey payee-claim path.

Fix 1 is the default scope for this step. Fix 2 is the fallback if the
standalone `wallet` CLI cannot set `program_owner`. Fix 3 is reserved
for the case where both prefund-side fixes prove incompatible with the
Store-mode deployment shape.

Note on `lgs` / `wallet` tooling.
Step 26 verification also exposed two tooling mismatches that are
prerequisites for this step's localnet verification:

- `lgs` 0.1.1 expects sequencer configs at the repo root; LEZ v0.2.0
  moved them under `lez/`. A symlink workaround (`sequencer -> lez/sequencer`)
  is in place on the v0.2.0 cache checkout and must be applied on any
  fresh `lgs setup`.
- The cargo-installed `wallet` CLI (0.1.0) cannot read v0.2.0's wallet
  storage format. Use the LEZ-built `wallet` binary from
  `~/.cache/logos-scaffold/repos/lez/<rev>/target/release/wallet` by
  prepending that directory to `PATH`.

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
