# Step 27 — plan excerpt

Active-work packet for agents. Index: [index.md](../index.md).

### Step 27, Claim Fix and Verification

Debug and fix the payment streams `claim` and `deposit` runtime path so
the Developer Journey (provider claim) works reliably on localnet and
TestNet v0.2, and the 52 `lez-payment-streams-core` `program_tests`
failing under LEZ v0.2.0 pass. User Journey testnet verification is
owned by Step 28; Step 27 keeps User Journey localnet as a
non-regression check only.

Prerequisite: [Step 26 — TestNet v0.2 Migration](../completed/step-26-testnet-v02-migration.md)
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

Symptom C — prior testnet-only claim known issue (rc5 era).
`docs/archive/operator/testnet-claim-known-issue.md` (status 2026-06-28)
records that on the public testnet (rc5 era) provider `claim` never
confirmed while `close` (same signer, same account order) did. Localnet
was unaffected. This is a THIRD failure, observed on rc5 testnet only,
and is NOT explained by v0.2.0 `program_owner` enforcement (which did
not exist in rc5). See Decision log Q1/Q7 for why `close` vs `claim`
diverge and why retiring the known-issue doc is premature.

Symptom D — Store-mode `claim` false-positive on v0.2.0 (NEW, found by
the Phase 4 diagnostic). The Store-mode E2E reports `demo_claim
ok=True` and exits 0, but the claim does NOT move funds. The provider
on-chain balance stays 0 and `vault_holding` stays unchanged. The
`demo_claim ok=True` is a false positive from two compounding issues:

- D1 (orchestrator false positive). The orchestrator treats
  `seed_claim_onchain` as success when the seed binary returns 0, and
  the seed binary's `TxPoller::poll_tx`
  (`lez/wallet/src/poller.rs:33`) returns `Ok(tx)` as soon as
  `get_transaction` finds the tx hash in any block — without checking
  the tx's execution status. The LEZ sequencer includes rejected
  transactions in blocks ("Created block with 1 transactions" right
  after the rejection log line), so `poll_tx` always succeeds. The
  chainAction fallback path (`run_local_e2e.py` `chain_action_success`)
  has the same lax check. The Q10 "provider balance increase" criterion
  is the only reliable success signal; `demo_claim ok=True` alone is
  insufficient.
- D2 (root cause — guest `claim` cannot credit a default-owned provider
  on v0.2.0). The guest `claim` (`methods/guest/src/bin/
  lez_payment_streams.rs:830`) credits `provider.account.balance`
  directly and returns the provider via
  `execute_stream_instruction_with_explicit_owner` (raw `Account`s, no
  claim). The spel-framework `#[instruction]` macro's auto-claim
  transformer rewrites `SpelOutput::execute` into
  `execute_with_claims(&accounts, &__claims_claim(...), calls)`, but
  for `#[account(mut, signer)]` the generated `__claims_claim` returns
  `AutoClaim::None` (no claim). On v0.2.0 the provider account on
  localnet has `program_owner: DEFAULT_PROGRAM_ID` and `nonce: 1`
  (incremented by prior signer txs like `create_stream`). The
  macro-generated dispatcher then applies a filter
  (`vendor/spel-framework-macros/src/lib.rs:303-329`) that DROPS any
  post-state where `pre.program_owner == DEFAULT_PROGRAM_ID` AND
  `pre.account != Account::default()` AND `post.required_claim()`
  is `None`. The provider matches all three, so its post-state is
  dropped from the program output. The sequencer's balance-conservation
  invariant (`lee/state_machine/core/src/program.rs:734-752`) then
  fails with `MismatchedTotalBalance` (pre 1050, post 850): the
  provider's +200 credit is silently discarded, so the debit from
  `vault_holding` is unmatched. The guest's `claim` is structurally
  broken on v0.2.0 for any provider account that has been touched
  (nonce > 0) but never initialized under a program.

The `withdraw` instruction (`guest:431`) does NOT have this bug because
it explicitly claims a default-owned recipient via
`AutoClaim::Claimed(Claim::Authorized)` when `withdraw_to` was
`Account::default()` (line 488). For non-default recipients, `withdraw`
assumes the recipient is already program-owned (in the unit tests,
genesis recipients have `program_owner = authenticated_transfer`). The
`claim` instruction lacks the equivalent claim logic for the provider.

Fix for Symptom D (the real claim fix, supersedes the earlier
"prefund-only" framing). The guest `claim` credits the provider directly
(no chained call needed) — the fix is preventive, not curative. v0.2.0
rejects modifying any non-default account that still has
`program_owner: DEFAULT_PROGRAM_ID` (check 7,
`NonDefaultAccountWithDefaultOwner`), and the spel macro's post-state
filter (D2) drops DEFAULT-owned, nonce-incremented accounts from the
program output to avoid that error — but this breaks balance
conservation when the dropped account carries a credit. The claim chain
(`authenticated_transfer`) cannot recover a non-default DEFAULT-owned
provider either: the chained `transfer` claims the recipient, but
`validate_execution` check 7 runs before claims are applied and rejects
the non-default DEFAULT-owned post-state.

The only viable fix is to ensure the provider is initialized under a
program (so `program_owner != DEFAULT`) BEFORE any signer transaction
touches it (incrementing its nonce). The fixture does this via
`wallet auth-transfer init` on the provider (mirroring the owner prefund
for Symptom A), which sets `program_owner = authenticated_transfer`
while the account is still default. After that, `create_stream` may
safely use the provider as a signer (nonce increments to 1), and the
guest `claim`'s direct credit survives (the filter keeps the pair
because `pre.program_owner != DEFAULT`).

This is D-fix-fixture (preventive init), chosen over D-fix-2 (chained
call) after empirical testing showed the chained call path also fails
check 7 on the `authenticated_transfer` side for non-default
DEFAULT-owned recipients. The guest `claim` source is unchanged; the
fix lives entirely in `scripts/fixture.sh::init_provider_account`. A
new unit test (`test_claim_succeeds_with_auth_transfer_owned_nonce_
incremented_provider`) guards the fixture-shaped provider (auth-transfer
-owned, nonce > 0) claim path.

Fund-flow facts (from the guest source, grounding the analysis).
- `deposit` (`guest:384`): debits `owner`, credits `vault_holding`, via
  a `ChainedCall` to `authenticated_transfer`. The guest does not touch
  native balance directly.
- `withdraw` (`guest:431`): debits `vault_holding`, credits
  `withdraw_to`, directly inside the guest (no chained call). Uses
  `AutoClaim::Claimed(Claim::Authorized)` when `withdraw_to` was
  default-owned, which is the claim logic `claim` is missing.
- `close_stream` (`guest:766`): does NOT move native balance. It only
  updates `vault_config.total_allocated` and `stream_config` state, then
  returns the accounts. This is why `close` confirms where `claim` does
  not: `close` triggers no balance-decrease enforcement and no
  default-owned-account credit, so the macro filter never drops a
  balance-bearing post-state.
- `claim` (`guest:830`): debits `vault_holding` (a program-owned PDA)
  and credits `provider`, directly inside the guest. No chained call.
  On v0.2.0 this is broken (Symptom D): the provider post-state is
  dropped by the macro filter because the provider is default-owned,
  non-default (nonce > 0), and not claimed, causing
  `MismatchedTotalBalance`.

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
Symptom A is resolved (F1 prefund). Symptom D requires a guest-side
fix (D-fix-2, chained call for the provider credit) plus a fixture
extension (auth-transfer-init the provider); see Symptom D above and
Q11.

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
orchestrator artifact logs `demo_claim` with `ok=True` AND the
provider's on-chain balance increases by `payout` AND
`vault_holding`'s on-chain balance decreases by `payout`. The
provider-balance check is MANDATORY because the `demo_claim ok=True`
field is a known false positive (Symptom D1): the seed binary's
`poll_tx` returns `Ok` for rejected transactions, and the chainAction
fallback's `chain_action_success` is equally lax. Exit code 0 from
`scripts/e2e.sh local run` is necessary but not sufficient. The
diagnostic that exposed this: after a "passing" Store-mode E2E, the
provider had `balance: 0, nonce: 2` and `vault_holding` was unchanged
at 1000; the sequencer log showed the claim txs failed with
`MismatchedTotalBalance`.

Q11. Does Symptom D change the fix scope? Yes. The earlier framing
("prefund the owner and the 52 unit tests + Store deposit are fixed")
was incomplete: it resolved Symptom A (deposit) and Symptom B (unit
tests) but left the Store-mode `claim` itself broken. Symptom D is the
real claim bug on v0.2.0 and is the primary remaining deliverable for
Step 27. The fix (D-fix-2, chained call to `authenticated_transfer`
for the provider credit) requires both a guest change (rewrite
`claim`'s provider credit as a `ChainedCall`) and a fixture change
(auth-transfer-init the provider in `fund_owner_account`, same as the
owner). The unit tests must be re-checked: the current unit tests pass
because the genesis provider has `program_owner =
authenticated_transfer().id()`, which hides the D2 filter drop (the
filter only drops DEFAULT-owned non-default accounts). A new unit test
with a DEFAULT-owned, nonce-incremented provider should be added to
guard the D2 path.

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

- [x] A1/A2 diagnostic completed and recorded (owner balance +
      `program_owner` read)
- [x] Store-mode `deposit` fix implemented (F1 prefund-side, Symptom A)
- [x] 52 `lez-payment-streams-core` `program_tests` pass (Symptom B)
- [x] Fix tested on localnet (Developer Journey deposit path)
- [x] `MODE=module CHAIN=local` E2E shows `{"phase":"claim","ok":true}`
      (non-regression — module mode already green)
- [x] Symptom D root cause fixed: provider auth-transfer-init in fixture
      (`scripts/fixture.sh::init_provider_account`) before any signer tx
      touches the provider; guest `claim` source unchanged (preventive
      fix, not curative)
- [x] `MODE=store CHAIN=local` E2E shows `demo_claim` `ok=True` with
      provider balance increase AND `vault_holding` decrease (Q10
      concrete criterion, false-positive-resistant) — verified: provider
      0→200, vault_holding 1000→800, no sequencer rejections
- [x] New unit test: auth-transfer-owned nonce-incremented provider
      claim path (guards the fixture-shaped provider)
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

- [step-26-testnet-v02-migration.md](../completed/step-26-testnet-v02-migration.md) — provides testnet v0.2 target
- [step-28-user-journey-testnet.md](step-28-user-journey-testnet.md) — enables full User Journey on testnet
- [archive/operator/testnet-claim-known-issue.md](../../archive/operator/testnet-claim-known-issue.md) — prior issue documentation
- [USER_JOURNEY.md](../../../../USER_JOURNEY.md) — to be updated with payee claim
- [DEVELOPER_JOURNEY.md](../../../../DEVELOPER_JOURNEY.md) — to be updated with provider claim
