# Step 27 — plan excerpt

Active-work packet for agents. Index: [index.md](../index.md).

### Step 27, Claim Fix and Verification

Debug and fix the payment streams `claim` and `deposit` runtime path so
the Developer Journey (provider claim) works reliably on localnet and
TestNet v0.2, and the 52 `lez-payment-streams-core` `program_tests`
failing under LEZ v0.2.0 pass. User Journey testnet verification is
owned by Step 28; Step 27 keeps User Journey localnet as a
non-regression check only.

Prerequisite: [Step 26 — TestNet v0.2 Migration](step-26-testnet-v02-migration.md)
(declared complete for verification purposes).

#### Background

Previous testnet integration (Step 18) marked `claim` as optional
("may be optional on testnet" per `testnet-claim-known-issue.md`).
This step elevates `claim` to required functionality for both journeys
and all deployment targets.

#### Root cause analysis (Step 26 verification)

Step 26 verification produced three distinct failure clusters that this
step must disentangle before any fix is attempted. They are NOT one bug.

Symptom A — Store-mode `deposit` runtime failure (the Step 26 blocker).
`MODE=store CHAIN=local ./scripts/e2e.sh local run` fails at the
`deposit` phase with:

```
ProgramExecutionFailed("Guest panicked: Sender has insufficient balance")
```

`initialize_vault` confirmed on chain after `wallet deploy-program`, so
the guest program is deployed and known. The panic originates inside the
`authenticated_transfer` system program's own balance check, reached via
the guest `deposit` instruction's `ChainedCall` (see
`methods/guest/src/bin/lez_payment_streams.rs`, `pub fn deposit`). The
guest `deposit` cannot debit `owner` directly because the guest does not
own `owner` (the guest only owns the vault PDAs it creates); it chains
into `authenticated_transfer` to move `amount` from `owner` to
`vault_holding`. The chained call panics with "Sender has insufficient
balance".

Two candidate explanations were confirmed by the Phase 1 diagnostic
(reading `owner`'s on-chain `balance` and `program_owner` after
`initialize_vault` confirms and before `deposit`):

- A1 (CONFIRMED). The `owner` account on chain has zero balance. The
  `seed_localnet_fixture prefund-onchain` subcommand runs
  `initialize_vault` then `deposit`; it does NOT separately mint or
  airdrop tokens to `owner`. The localnet genesis
  (`lez/sequencer/service/configs/debug/sequencer_config.json`) funds
  only a fixed set of supply accounts, NOT wallet-derived owner
  accounts. After `initialize_vault` the owner had `balance: 0`,
  `program_owner: 11111111…` (DEFAULT_PROGRAM_ID), `nonce: 1`.
- A2 (also present, secondary). Once the owner is touched by
  `initialize_vault` (nonce > 0) but still has DEFAULT_PROGRAM_ID,
  v0.2.0's `NonDefaultAccountWithDefaultOwner` invariant blocks the
  pinata faucet from claiming/crediting it. The owner must be
  initialized under `authenticated_transfer` (`wallet auth-transfer
  init`) BEFORE `initialize_vault` touches it, then funded via pinata
  claims.

Diagnostic result: A1 is the primary cause (zero balance); A2 is the
sequencing constraint (init under auth-transfer before any other program
touches the account). The fix is to fund the owner via
`wallet auth-transfer init` + `wallet pinata claim` (looped to reach the
deposit amount) BEFORE `prefund-onchain` runs. This matches the
module-mode path (`lgs wallet topup` wraps the same init+pinata sequence).

Symptom B — `lez-payment-streams-core` unit-test failures (52 tests).
Separately from the runtime, 52 `program_tests` failed under v0.2.0. The
harness `test_helpers.rs::create_state_with_guest_program` was already
updated to (i) register `authenticated_transfer` and `clock` via
`.with_programs([...])` and (ii) set `program_owner` on genesis accounts.
The previous fix set `program_owner = guest_program.id()`, which caused
`UnauthorizedBalanceDecrease` because the `deposit` instruction's
`ChainedCall` to `authenticated_transfer` debits the owner, and v0.2.0
requires the executing program (`authenticated_transfer`) to own the
account it debits — not the guest program. The fix: set
`program_owner = authenticated_transfer().id()` on genesis accounts,
matching the runtime path where `wallet auth-transfer init` sets the
owner's `program_owner` to the `authenticated_transfer` program. This
resolved all 52 failures: 138/138 `program_tests` pass.

Symptom C — prior testnet-only claim known issue.
`docs/archive/operator/testnet-claim-known-issue.md` (status 2026-06-28)
records that on the public testnet (rc5 era) provider `claim` never
confirmed while `close` (same signer, same account order) did. Localnet
was unaffected. This is a THIRD failure, observed on rc5 testnet only,
and is NOT explained by v0.2.0 `program_owner` enforcement (which did
not exist in rc5). See Decision log Q1/Q7 for why `close` vs `claim`
diverge and why retiring the known-issue doc is premature.

Fund-flow facts (from the guest source, grounding the analysis).
- `deposit` (`guest:384`): debits `owner`, credits `vault_holding`, via
  a `ChainedCall` to `authenticated_transfer`. The guest does not touch
  native balance directly.
- `withdraw` (`guest:431`): debits `vault_holding`, credits
  `withdraw_to`, directly inside the guest (no chained call). Uses
  `AutoClaim::Claimed` when `withdraw_to` was default-owned.
- `close_stream` (`guest:766`): does NOT move native balance. It only
  updates `vault_config.total_allocated` and `stream_config` state, then
  returns the accounts. This is why `close` confirms where `claim` does
  not on rc5 testnet: `close` triggers no balance-decrease enforcement
  at all.
- `claim` (`guest:830`): debits `vault_holding` (a program-owned PDA)
  and credits `provider`, directly inside the guest. No chained call.
  The provider account is credited, not debited, so `program_owner` on
  `provider` is not the blocker for `claim` itself; the blocker would be
  `program_owner` on `vault_holding` (which the guest owns) or, on rc5
  testnet, something in the message/sequencer path.

Possible fixes (decision deferred — see Decision log Q3).

- F1 (prefund-side): ensure `owner` has balance AND
  `program_owner = <payment_streams_program_id>` before `deposit`.
  Requires either extending `seed_localnet_fixture` / the `wallet` CLI
  to set `program_owner`, or routing prefund through the guest program
  so it claims `owner`.
- F2 (guest-side claim): add an explicit claim/authorize step in the
  guest before the chained transfer, so `owner` is owned by the guest
  before `authenticated_transfer` runs. More invasive, prefund-agnostic.
- F3 (bypass): drop the chained `authenticated_transfer` in `deposit`
  and debit `owner` directly inside the guest (requires `owner` to be
  guest-owned, i.e. F2's claim step). Changes the fund-flow shape.

Fix selection is gated on the A1/A2 diagnostic and on whether
`seed_localnet_fixture` / `wallet` can set `program_owner` (Q3).

Note on `lgs` / `wallet` tooling (prerequisites, not in-scope code).
Step 26 verification exposed two tooling mismatches that block
localnet verification. They are environment setup, not Step 27 code
changes, but the implementer must apply them:

- `lgs` 0.1.1 expects sequencer configs at the LEZ repo root; v0.2.0
  moved them under `lez/`. Workaround: `ln -s lez/sequencer sequencer`
  at the v0.2.0 cache checkout root
  (`~/.cache/logos-scaffold/repos/lez/<rev>/`). This is a per-checkout
  symlink, not committed; see Decision log Q9 for whether `scripts/`
  should auto-create it (deferred: track as a follow-up, do not block
  Step 27 on it).
- The cargo-installed `wallet` CLI (0.1.0) cannot read v0.2.0's wallet
  storage format and ignores `LEE_WALLET_HOME_DIR`. Use the LEZ-built
  `wallet` from
  `~/.cache/logos-scaffold/repos/lez/<rev>/target/release/wallet` by
  prepending that directory to `PATH`. This is a per-run PATH setting;
  the implementer should document the exact export in the verification
  artifact but not modify `scripts/` for it in this step.

#### Decision log (open questions resolved)

Q1. One root cause or two? Three, not one. Symptom A (Store-mode
`deposit` runtime) and Symptom B (52 unit tests) are both v0.2.0
ownership/balance issues but at different layers (runtime vs. harness).
Symptom C (rc5 testnet `claim` vs `close`) is a separate, older issue
not caused by v0.2.0. Step 27 targets A and B; C is investigated only
if A's fix lands and testnet `claim` still fails.

Q2. User Journey testnet verification in scope? No. Step 28 owns
`MODE=module CHAIN=testnet` (User Journey testnet). Step 27's testnet
scope is Developer Journey (`MODE=store CHAIN=testnet`) only. The DoD
line "TestNet v0.2 claim verified for User Journey (payee)" is moved to
Step 28; Step 27 keeps only the Developer Journey testnet line. The
verification-commands comment "# User Journey — testnet (requires Step
28 for full support)" is correct and stays.

Q3. Can `seed_localnet_fixture` / `wallet` set `program_owner`? Resolved.
`seed_localnet_fixture.rs` has no `--program-owner` flag and cannot set
`program_owner`. The `wallet` CLI's `auth-transfer init` subcommand
initializes an account under the `authenticated_transfer` program
(sets `program_owner = authenticated_transfer`), and `wallet pinata
claim` funds it from the faucet. The Phase 1 diagnostic confirmed A1
(owner balance is 0) plus the A2 sequencing constraint (the owner must
be auth-transfer-initialized BEFORE `initialize_vault` touches it, or
the `NonDefaultAccountWithDefaultOwner` invariant blocks the pinata
claim). Fix F1 (prefund-side) was implemented: `cmd_prefund` in
`scripts/fixture.sh` now calls `fund_owner_account` (auth-transfer init
+ pinata claim loop) before `prefund-onchain`, and also deploys the
guest program idempotently. `examples/src/bin/seed_localnet_fixture.rs`
was NOT modified; the funding step lives in `fixture.sh` because it is
a wallet-CLI operation, not a seed-binary operation.

Q4. Does module-mode claim also fail? No. Step 26 verification recorded
`{"phase":"claim","ok":true}` for `MODE=module CHAIN=local`. Module
mode deploys and drives the program through `logoscore` + the wallet
module, which routes prefund through the guest and avoids the Store-mode
`owner`-balance gap. The module-mode `claim` path is green on v0.2.0.
The DoD gate `MODE=module CHAIN=local` E2E shows `{"phase":"claim","ok":true}`
is a non-regression check, not a fix target.

Q5. Fix 1 vs Fix 2 decision criteria. Resolved — Fix 1 (F1, prefund-side)
was chosen and implemented. The Phase 1 diagnostic confirmed A1 (owner
balance is 0), so the fix is a prefund/airdrop change: `fund_owner_account`
in `fixture.sh` runs `wallet auth-transfer init` + `wallet pinata claim`
(looped) before `prefund-onchain`. F2 (guest-side claim) and F3 (bypass)
were not needed. The unit-test fix (Symptom B) was the mirror: set
`program_owner = authenticated_transfer().id()` on genesis accounts in
`test_helpers.rs`. Both fixes are verified: 138/138 unit tests pass;
`full-reset-localnet` prepare passes (pinata funds owner to 1050,
init+deposit confirm, snapshot saved).

Q6/Q9. Tooling prerequisites — environment vs code. The `lgs` symlink
and LEZ-built `wallet` PATH are per-environment setup, already applied
on the v0.2.0 cache checkout used for Step 26 verification. They are
NOT committed code. The implementer applies them on any fresh
`lgs setup`. Auto-creating the symlink in `scripts/e2e.sh` or
`scripts/lifecycle.sh` is a follow-up, not Step 27 scope (deferred to
avoid scope creep; tracked as a note here).

Q7. Why does the prior testnet known issue persist if root cause is
`program_owner`? Because Symptom C predates v0.2.0 and rc5 had no
`program_owner` enforcement. The `close` vs `claim` divergence on rc5
testnet is explained by the fund-flow facts: `close` moves no native
balance, so it triggers no balance-decrease enforcement of any kind;
`claim` debits `vault_holding` and credits `provider`. On rc5 testnet
the rejection was at sequencer validation/mempool, not execution, which
points to message encoding or mempool policy, not `program_owner`.
Retiring `testnet-claim-known-issue.md` is premature. The DoD line
"`archive/operator/testnet-claim-known-issue.md` updated or retired" is
amended to "updated": after A/B are fixed, re-run testnet `claim` and
record whether C is still reproducible; if it is, C becomes a separate
follow-up. The first diagnostic for C (per the known-issue doc's own
"Next diagnostic step") is a byte-level comparison of `close` vs
`claim` messages on localnet.

Q8. `USER_JOURNEY.md` / `DEVELOPER_JOURNEY.md` ownership. The
implementer updates them directly in this step, at CLI-example depth
(single worked example with expected output per journey), not a full
editorial pass. Full walkthrough polish is a separate documentation
step. Depth: one claim CLI invocation + expected confirmation output,
cross-linked to the verification artifact.

Q10. Exact Store-mode claim success artifact. The DoD check "MODE=store
CHAIN=local E2E shows provider claim success" is made concrete as: the
orchestrator artifact (`scripts/e2e/run_local_e2e.py`
`demo_teardown`/`seed_claim_onchain`) logs `demo_claim` with
`ok=True, via="seed_claim_onchain"` (the direct-submit path) OR
`ok=True` via `chainAction` with a confirmed `tx_hash`, AND the
provider's on-chain balance increases by `payout`. Exit code 0 from
`scripts/e2e.sh local run` is necessary but not sufficient; the
`demo_claim` artifact field must be `True`.

#### Investigation scope

Step 27 targets Symptom A (Store-mode `deposit`/`claim` runtime) and
Symptom B (52 unit tests), on localnet and Developer Journey testnet.
Symptom C (rc5 testnet `claim` vs `close`) is investigated only after A
is fixed. User Journey testnet is owned by Step 28.

| Scenario | Actor | Chain | Expected | Owner |
|----------|-------|-------|----------|-------|
| Developer Journey | Provider (paid Store host) | localnet | `demo_claim` `ok=True`, provider balance rises | Step 27 |
| Developer Journey | Provider | testnet v0.2 | `claim` succeeds after serving paid query | Step 27 |
| User Journey | Payee (stream recipient) | localnet | `{"phase":"claim","ok":true}` (non-regression) | Step 27 (non-regression only) |
| User Journey | Payee | testnet v0.2 | `chainAction claim` succeeds | Step 28 |

#### Deliver

- A1/A2 diagnostic result (on-chain `owner` balance + `program_owner`
  read between `initialize_vault` and `deposit`), documented in the
  verification artifact
- Fix implementation in `lez-payment-streams-core`, guest, module, or
  `examples/` (per Q3 — `examples/` is in scope if F1 requires a new
  `seed_localnet_fixture` subcommand)
- 52 `lez-payment-streams-core` `program_tests` passing (Symptom B,
  fixed in lockstep with the chosen runtime fix)
- Verification on localnet (Developer Journey) and TestNet v0.2
  (Developer Journey)
- Updated E2E artifacts showing `claim` phase succeeding
- `testnet-claim-known-issue.md` updated with Symptom C re-test result
  (not retired — see Q7)
- `USER_JOURNEY.md` / `DEVELOPER_JOURNEY.md` claim CLI examples (Q8 depth)

#### Definition of done

- [ ] A1/A2 diagnostic completed and recorded (owner balance +
      `program_owner` read)
- [ ] Store-mode `deposit` fix implemented (F1/F2/F3 per diagnostic)
- [ ] 52 `lez-payment-streams-core` `program_tests` pass (Symptom B)
- [ ] Fix tested on localnet (Developer Journey)
- [ ] `MODE=module CHAIN=local` E2E shows `{"phase":"claim","ok":true}`
      (non-regression — module mode already green)
- [ ] `MODE=store CHAIN=local` E2E shows `demo_claim` `ok=True` with
      provider balance increase (Q10 concrete criterion)
- [ ] TestNet v0.2 claim verified for Developer Journey (provider)
- [ ] `archive/operator/testnet-claim-known-issue.md` updated with
      Symptom C re-test result (not retired)
- [ ] User Journey documentation includes payee claim example (localnet)
- [ ] Developer Journey documentation includes provider claim example

#### Verification commands

```bash
# Developer Journey — localnet (primary fix target)
MODE=store CHAIN=local ./scripts/e2e.sh local run

# User Journey — localnet (non-regression; module mode already green)
MODE=module CHAIN=local ./scripts/e2e.sh local run

# Developer Journey — testnet v0.2
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run

# User Journey — testnet: owned by Step 28, not Step 27
# MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run
```

Tooling prerequisites (apply per environment before running — see Q6/Q9):
`ln -s lez/sequencer sequencer` at the v0.2.0 cache checkout root;
prepend `~/.cache/logos-scaffold/repos/lez/<rev>/target/release` to
`PATH` for the LEZ-built `wallet`; export
`LEE_WALLET_HOME_DIR="$PWD/.scaffold/wallet"`.

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
