# Step 44 — payer-close and payee-close

Index: [index.md](../index.md). Status: complete (2026-08).

Absorbs former raw TODO
`e2e-close-payer-authority` (removed; content merged here).

## Goal

Make both close paths work on real LEZ (localnet and testnet), public and private
execution:

1. Owner-close — vault owner signs close (User Journey narrative).
2. Provider-close — stream provider signs close (current E2E / seed path).

LIP-155 already allows either party. Today only provider-close works end-to-end.

## Terminology

Packet filename keeps historical “payer/payee.” Prefer owner-close /
provider-close in new prose.

[Step 47](step-47-unify-role-terminology.md) (complete)
hard-cut living module JSON and journey
env: write/close keys are `owner` / `provider` (not `signer` / `authority`);
journey helpers use `$OWNER` / `$PROVIDER`. Layout types are `StreamProvider*`.
Do not reintroduce legacy keys. `scripts/check-terminology.sh` temporarily
path-allowlists this packet until it moves to `completed/` (D47.5).

| Term | Meaning in this step |
| --- | --- |
| User / vault owner | `VaultConfig.owner`; USER_JOURNEY / module JSON `owner` (`$OWNER`) |
| Provider | `StreamConfig.provider`; USER_JOURNEY / close JSON `provider` (`$PROVIDER`) |
| Module `owner` field | Vault owner account id for PDA derivation on writes/close — not LEZ `#[account(signer)]` |
| Module `provider` field (close) | Optional close JSON; omit → owner path (this step’s five-slot ix); present → provider path |
| Tx submitter | Owner-close: vault owner (`owner` field); provider-close: provider (`provider` field). Precedence `provider` > `owner` (D47.6) |
| `CLOSE_ROLE` | Module E2E close-role switch: `owner` or `provider`. Step 47 already ships the flag (default `provider` on today’s six-slot close). This step makes `CLOSE_ROLE=owner` (omit `provider` key) a working path and may flip the default to `owner`. Orthogonal to Store `E2E_CLOSE_VIA` (`seed` \| `chainaction`) |

## Problem

Guest `close_stream` uses six account slots including both `owner` (non-signer, PDA
binding) and `authority` (signer). When the payer closes, those two slots want the
same account id.

LEZ rejects duplicate account ids in one instruction (`PreStateAccountIdsNotUnique`
/ `"Duplicate account_ids found in message"`). Verified on LEZ tip `v0.2.4`
(`47eba256`) as well as the payment-streams pin `v0.2.0`. Same rule for public and
private execution. Enforcement is visibility-independent (message-level public and
PP checks plus execution-level uniqueness), so “hide the duplicate in a private
slot” does not work.

So omit-`provider` / `provider` = owner is not a working on-chain path with the
current layout. Automated greens (`module-e2e.sh`, Store default close) use
`provider` = stream provider (Step 47 rename of former `authority`). Rust program
tests always use distinct owner and provider ids. USER_JOURNEY teaches
owner-close, but that shape was never a matrix gate.

Secondary issue: `signingRequirementsForAccounts` marks every slot matching the
submitter as `needs_sign`. That is a module shortcut; fixing flags alone does not
clear uniqueness.

## Locked design

Split close into two guest instructions that reuse layouts this program already
has for other ops. Do not invent a third account model.

Token dual-ix (`mint` / `mint_with_authority`) is the LEZ uniqueness precedent
(two fixed account lists + shared inner logic). Role naming does not follow token:
token encodes self vs rotated single authority; close has two peer authorizers
(LIP either-party). Use explicit role names instead of `_with_authority`.

Payment-streams already has both account shapes among stream instructions:

- Owner-signed five-slot — `pause_stream` / `resume_stream` / `top_up_stream`
  (`owner` is `#[account(signer)]`; PDA still binds via `account("owner")`).
- External-authority six-slot — `claim` (non-signing `owner` + signing `provider`).

### Guest instructions

Exact per-slot attributes (SPEL validation / auto-claim; do not copy “same as”
without checking `mut` / `signer`):

`close_stream_by_owner` (five-slot; match `pause_stream` / `top_up_stream` for
slots 0–4; `top_up_stream` is the precedent that already writes `vault_config`
from this layout):

| Slot | Account | Attributes |
| --- | --- | --- |
| 0 | `vault_config` | `mut`, PDA `[vault_config, owner, vault_id]` |
| 1 | `vault_holding` | `mut`, PDA `[vault_holding, vault_config, native]` |
| 2 | `stream_config` | `mut`, PDA `[stream_config, vault_config, stream_id]` |
| 3 | `owner` | `signer` (not `mut`; same as pause/top_up) |
| 4 | `clock` | (no mut/signer) |

`close_stream_by_provider` (six-slot; keep today’s close flags — owner non-signing
`mut`, authority/provider signing without the claim-style `mut` on the signer
unless claim semantics require it; prefer matching today’s `close_stream` so PP
auto-claim behavior for close does not silently change):

| Slot | Account | Attributes |
| --- | --- | --- |
| 0 | `vault_config` | `mut`, PDA `[vault_config, owner, vault_id]` |
| 1 | `vault_holding` | `mut`, PDA `[vault_holding, vault_config, native]` |
| 2 | `stream_config` | `mut`, PDA `[stream_config, vault_config, stream_id]` |
| 3 | `owner` | `mut` (non-signing; PDA binding) |
| 4 | `provider` | `signer` (today’s close uses `authority: signer` without `mut`) |
| 5 | `clock` | (no mut/signer) |

Do not silently adopt claim’s `provider: #[account(mut, signer)]` unless a
deliberate PP auto-claim change is intended. Default: copy today’s close flags.

| Path | Guest instruction | Signer | Auth check |
| --- | --- | --- | --- |
| Payer-close | `close_stream_by_owner` | vault `owner` | `load_owner_stream_context` / `VaultOwnerMismatch` |
| Payee-close | `close_stream_by_provider` | stream `provider` | signer is `StreamConfig.provider`; owner slot non-signing for PDA binding; mismatch → `CloseUnauthorized` (message “not stream provider”, same shape as claim) |

Shared close accounting (D44.13): one helper that performs `close_at_time` +
`checked_total_allocated_after_release` + `write_account_data(vault_config)` +
`write_account_data(stream_config)`. Surrounding loader and
`execute_owner_stream_instruction` /
`execute_stream_instruction_with_explicit_owner` stay per-path. Pause only
writes `stream_config`; close must also write `vault_config.total_allocated` —
same five-slot write pattern as `top_up_stream`. Do not merge the two loaders.

Wire enum (hard cut, Option A). Discriminants are owned by the
`lez-payment-streams-core` `Instruction` enum declaration order (serde). Guest
SPEL dispatch is name-based (`Instruction::PascalName`); guest source order is
an IDL-ordering constraint for `make idl`, not the wire discriminant source.
Keep `Claim` at index 8:

| Index | Before | After |
| --- | --- | --- |
| 7 | `CloseStream` | `CloseStreamByOwner` |
| 8 | `Claim` | `Claim` (unchanged) |
| 9 | — | `CloseStreamByProvider` (appended) |

Do not remove `CloseStream` and append both after `Claim` (that would shift
`Claim` from 8→7). Regenerate IDL via `make idl` → `lez-payment-streams-idl.json`.

`instruction_wire.rs` must assert golden leading words
(`CloseStreamByOwner` → 7, `Claim` → 8, `CloseStreamByProvider` → 9). Round-trip
alone cannot detect a uniform renumbering.

Rejected alternatives (do not pursue):

- One instruction with an optional authority slot — SPEL account lists are fixed
  per instruction; optional slots do not remove uniqueness.
- Drop `owner` from the payer-close list — breaks PDA `account("owner")` binding
  and the LIP intent that owner identity is present.
- Module-only or signing-flag fix — LEZ rejects duplicate ids before program
  logic runs.
- Token-style `close_stream` / `close_stream_with_authority` naming — implies
  rotated single authority; roles here are peer closers.
- Hide duplicate id in a private slot — execution-level uniqueness is
  visibility-independent.

### Module and FFI

Keep a single `chainAction closeStream` surface so USER_JOURNEY omit-`provider`
becomes real. Dispatch is strict (reject mismatches; do not guess).

Identity comparisons are over normalized 32-byte account ids (same helpers as
other write paths: base58 / 64-hex accepted; raw JSON-string equality is wrong).

Identity for the reject matrix comes from on-chain reads before plan/serialize
(D44.11), mirroring `enforceDepositSignerEqualsOwner` as a pre-submit check
precedent:

1. Treat module `owner` as the vault-owner id for PDA derivation (same as today
   for close/claim/pause).
2. Derive vault PDAs; `loadVaultConfigOnChain` via `get_account_public`; require
   a decodable vault config. Because seeds are
   `["vault_config", account("owner"), vault_id]`, a wrong `owner` usually
   yields a missing account (`close_prestate_unavailable`), not
   `close_owner_mismatch`. Keep an equality assert as defense-in-depth if data
   is present; negative E2E for wrong-owner asserts `close_prestate_unavailable`.
3. Stream pre-read is path-dependent (D44.11 / M1 locked as (a)):
   - Owner path: do not pre-read `stream_config` for dispatch. Vault identity
     is enough to choose owner-close. A missing/wrong stream fails later in
     the guest when the five-slot list is executed.
   - Provider path: derive stream PDA; decode stream config (same path as
     `getStreamStatus`); read `StreamConfig.provider` and require
     `provider` JSON match before plan. If stream data is missing/undecodable —
     reject with `close_prestate_unavailable` (no in-module sync loop;
     callers/orchestrators keep `sync_wallet` retries).
4. Apply the dispatch table using those on-chain ids.
5. If any required-for-path account/data is missing — reject without
   submitting (no plan-and-hope wasted tx).

Pre-submit vs guest boundary (S2): pre-submit rejects cover identity and
dispatch errors (wrong closer role, provider mismatch, create
`provider == owner`, missing vault prestate needed to choose a path). On the
owner path, stream existence is not pre-checked — sync-lag robustness wins;
a missing stream fails in the guest. Do not “fix” that asymmetry by adding
an owner-path stream pre-read.

Dispatch selects a path from JSON ids; it does not prove the caller controls
the submitter key. A caller who omits `provider` while passing the vault
owner id as `owner` is routed to the owner path and then fails at the wallet
(or wallet error 7 on PF) if they cannot sign as owner — not at a module
reject token. Do not assert a close_* token for that case in the negative
cell.

Thread the decoded `PsFfiDecodedVaultConfig` (or resolved privacy tier) into
`VaultSubmitContext` so dispatch and `buildAndSubmit` /
`vaultPrivacyTierForSubmit` share one vault read (M3). Do not re-decode the
same account from two independent reads that can disagree.

PP / PseudonymousFunding note (rejects a common false concern): for both
`Public` and `PseudonymousFunding` vaults, `vault_config` and `stream_config`
PDAs are `PublicNoSign` ([Step 36](step-36-payer-funder-unlinkability.md)
account tables). Only the owner (and optionally provider) key slots are private.
`accountDataBytesFromHex` already uses `get_account_public`. Provider-close on a
PF vault can therefore pre-read owner/provider identity without the provider
holding the owner NSK. Do not add a “skip pre-read and submit anyway” PF
exception — that would weaken D44.11 without restoring any lost capability.
`buildAndSubmit` already reads vault tier from the public `VaultConfig` for
private-submit routing; D44.11 extends the same public PDA reads for dispatch.

Host constraint (M10): submitting provider-close on a PF vault still requires
the submitting host to resolve the private owner non-signing slot. In dual-host
Store E2E that means the user host (`cfg_user`), not the provider host
(wallet error 7 if the private owner NSK is absent). Capability is preserved;
dual-host PF payee-close must run where the owner NSK lives. Optional LIP
operator note only if the normative text already discusses private-account
resolution; do not invent a new LIP requirement for host key placement.

Under `PROVIDER_PRIVACY`, `StreamConfig.provider` holds the NPK-derived id
(D37.3). Module now compares caller `authority` to that stored id (newly
load-bearing). Normalize both sides to 32-byte ids; add a PP unit assert that
id-form comparison accepts the stored private-provider id.

| Module call | Dispatch |
| --- | --- |
| `authority` omitted (key absent or JSON null) | Owner path if vault prestate resolves for `signer`; else reject. Do **not** implement this by coercing absent/null `authority` into the signer string before classification — classify absence first, then run owner-path checks. |
| `authority` present and equal to `signer` | Owner path if that id resolves as vault owner; else reject |
| `authority` present, distinct from `signer`, and equal to on-chain stream provider | Provider path; submitter = `authority`; `signer` is vault owner (non-signing slot) |
| `authority` present, distinct from `signer`, not equal to on-chain stream provider | `close_provider_mismatch` |
| `authority` key present but empty / whitespace-only | `close_args_mismatch` |
| `signer` missing or empty / whitespace-only | `close_args_mismatch` |
| Anything not matched above | Reject; never coerce (including today’s “absent authority → copy signer string” shortcut) |

Empty/whitespace checks for `signer` and `authority` MUST run **before**
`signerAccountIdHex` / `ownerBytesFromBase58` (P1). Those helpers return their
own strings (`invalid signer account id`, `account_id_from_base58 failed`); if
empties reach them, the asserted `close_args_mismatch` never appears. Today’s
`authorityBase58 = valid && non-null ? toString() : signer` treats empty string
as present and must be replaced by explicit absence vs empty vs value branches.

Submitter and layout (must not copy-paste today’s always-`StreamAuthority6` close):

| Path | `ctx.layout` | `buildAndSubmit` submitter arg |
| --- | --- | --- |
| Owner-close | `StreamOwner5` (same as pause/resume/top-up) | `signerAccountIdBase58` (owner) |
| Provider-close | `StreamAuthority6` (same as claim / today’s close) | `authorityAccountIdBase58` (provider) |

`signingRequirementsForAccounts` is invoked with that path-specific submitter so
only the true signer slot is marked `needs_sign`. `pfOwnerSlotByLayout` already
maps index 3 for both layouts — no new `VaultIxLayout` value.

Module reject tokens (S1). Negative E2E asserts only the Asserted column.
Reserved tokens may exist in code as defense-in-depth but are not DoD asserts.
Malformed non-empty ids that fail `ownerBytesFromBase58` /
`signerAccountIdHex` keep those helpers’ existing error strings (not remapped
into this table).

| Condition | Error token | Negative E2E |
| --- | --- | --- |
| Vault missing/undecodable, or wrong-`signer` unresolved PDA; provider-path stream missing/undecodable | `close_prestate_unavailable` | Asserted |
| Decoded vault present but `VaultConfig.owner` ≠ normalized `signer` | `close_owner_mismatch` | Reserved (defense-in-depth; wrong-signer normally hits prestate first) |
| Provider path; normalized `authority` ≠ `StreamConfig.provider` | `close_provider_mismatch` | Asserted |
| Empty `signer`, or `authority` key present but empty | `close_args_mismatch` | Asserted |
| `createStream` with `owner` bytes == `provider` bytes | `create_provider_equals_owner` | Asserted |

Guest create reject uses next free `ErrorCode` discriminant `6027` (explicit
enum values `6001`–`6026` today; no shift risk). `make idl` regenerates errors.

FFI: two serialize + two plan entry points (or one pair per path). Same change
set must update all three FFI surfaces:

1. `lez-payment-streams-ffi/src/instruction_abi.rs`
2. Regenerated `lez-payment-streams-ffi/lez_payment_streams_ffi.h` (cbindgen)
3. Hand-maintained module bridge
   `logos-payment-streams-module/src/payment_streams_ffi_bridge.h` and
   `payment_streams_ffi_bridge_writes.c` (`ps_ffi_serialize_close_stream*` /
   `ps_ffi_plan_close_stream*` + status maps) — not produced by cbindgen

### Public and private execution (no regression)

Invariant: both close paths MUST work in public and private execution without
losing Step 36/37 capabilities. Guest remains visibility-agnostic; module
routing stays as today.

| Scenario | Close path | Submit | Evidence after Step 44 |
| --- | --- | --- | --- |
| Public vault, public owner | Owner-close | Public | Required module local + testnet |
| Public vault, public provider authority | Provider-close | Public | Required Store (chainAction) + thin `CLOSE_ROLE=provider` |
| `PseudonymousFunding` vault, private owner | Owner-close | Private; owner signs | `OWNER_PRIVACY=1` module × local (payer); real-prove cell (D44.18) |
| `PseudonymousFunding` vault, provider closes | Provider-close | Private because owner non-signer is private | PP unit on PF fixture + `CLOSE_ROLE=provider OWNER_PRIVACY=1` module × local (D44.14 amended) |
| Public vault, private provider authority | Provider-close | Private (D37.9) | PP unit (existing public-tier private-provider setup) + optional Store/provider-privacy |
| Public vault, private `provider_id` at create | N/A (create) | Public — provider is instruction data only | Unchanged + byte-equality create reject on normalized ids |
| Owner-close with shielded payee | Owner-close | (path as above) | Provider is absent from the five-slot list; a shielded payee cannot block payer-close (product property) |

PF provider-close host constraint: see Module section (M10).

DoD does not require `PROVIDER_PRIVACY=1 CLOSE_ROLE=provider` as a Store cell.
Private-provider submit routing remains D37; covered by PP unit + optional
Store/provider-privacy runs. Thin public-provider `CLOSE_ROLE=provider` still
gates dispatch + `chainAction`.

## Testing structure (layered)

E2E wall clock is dominated by funding, dual-host Store orchestration, and
especially `RISC0_DEV_MODE=0` (Step 39 Store full privacy ~106 min CPU on
testnet; multi-minute PPE per submit). Guest correctness does not need that
cost. Prefer the cheapest layer that can falsify the claim.

| Layer | What it proves | Cost | Step 44 role |
| --- | --- | --- | --- |
| Core / wire unit | Discriminants, planners, create reject, wrong account-count panics | Seconds (`cargo test`) | Required; golden words + wrong-length per path |
| Program / PP harness (`RISC0_DEV_MODE=1`) | Both close ix; public + private owner/provider; PF private-owner non-signer on six-slot; accounting | Seconds–minutes | Required full guest×privacy coverage |
| Module local soft (`RISC0_DEV_MODE=1`) | Module dispatch, layouts, submitter wiring, reject tokens, funding+clock+claim lifecycle | Tens of minutes per full run | Primary combinatorial surface for close role × owner privacy |
| Module local real-prove (`RISC0_DEV_MODE=0`) | New ImageID actually proves PP close (stub receipts do not) | Many minutes; CLOCK_50 windows (D39.25) | One required ImageID re-green (D44.18) |
| Store local/testnet | Dual-host eligibility + default payee-close via `chainAction` | High (hosts, Store build) | Required 2×2 keeps payee-close; no close-role × privacy product |
| Module/Store testnet privacy | Org-gate regression under real prove | Very high | Optional gate-log rows; not full matrix |

Principle: cover every close-path × privacy combination at unit/PP and at
module soft-local where a one-flag variation is cheap; keep Store and
real-prove as sparse gates.

Recommended module soft-local close × owner-privacy square (four cells; not
Store; not testnet):

| | Public owner | `OWNER_PRIVACY=1` |
| --- | --- | --- |
| `CLOSE_ROLE=owner` (default) | Required module × local | DoD privacy module (payer-close) |
| `CLOSE_ROLE=provider` | Thin DoD cell | Thin DoD variation (restores today’s PF provider-close evidence) |

`PROVIDER_PRIVACY` remains claim-focused; it does not select close role
(D44.14). Full `MODE × CHAIN × privacy × close` product stays out of scope.

## E2E coverage mechanics

Locked (D44.4 / D44.15 amended). Payer-close is the primary User Journey and
Required module closing mode. Provider-close remains Required on Store
(default `E2E_CLOSE_VIA=chainaction` with `authority=provider`) plus module soft
cells above. Do not add close-role as a Store or testnet combinatorial axis.

Env naming: use `CLOSE_ROLE=owner|provider` (or `MODULE_E2E_CLOSE_ROLE`) for the
module orchestrator. Do not reuse `CLOSE_VIA` — Store already has
`E2E_CLOSE_VIA` meaning mechanism (`seed` \| `chainaction`).

### Orthogonal coverage

| Concern | Where proven | Close role |
| --- | --- | --- |
| User Journey happy path | Required `MODE=module` local + testnet | Payer (omit `authority`) |
| Store / eligibility | Required `MODE=store` local + testnet | Payee via default `chainAction` (`authority=provider`); seed path still reachable via `E2E_CLOSE_VIA=seed` |
| Private owner submit (payer-close) | `OWNER_PRIVACY=1` module × local; real-prove cell (D44.18) | Payer |
| Private owner non-signer (provider-close on PF) | PP PF fixture + `CLOSE_ROLE=provider OWNER_PRIVACY=1` module × local | Payee |
| Private provider claim | Optional `PROVIDER_PRIVACY=1` module/Store as today | Module close stays **payer** unless `CLOSE_ROLE=provider`; claim remains the privacy focus |
| Provider-close ix + module dispatch | Program/PP + thin `CLOSE_ROLE=provider` (+ PF variation) | Payee |
| Dispatch / create rejects | Localnet negative (includes `create_provider_equals_owner`) | N/A |
| ImageID real prove | One required local privacy cell under `RISC0_DEV_MODE=0` (D44.18): `OWNER_PRIVACY=1` payer-close; claim in the same run covers six-slot PF under real prove | Payer |

### Reduced matrix

Required (same 2×2 square as today; module close flips to payer; Store stays
payee via chainAction by default):

| Cell | Close |
| --- | --- |
| module × local | Payer |
| module × testnet | Payer |
| store × local | Payee (`chainAction`; not seed-by-default) |
| store × testnet | Payee (`chainAction`; not seed-by-default) |

Step 44 DoD extras (local; not a Store/testnet grid):

| Cell | Purpose | Close | Size |
| --- | --- | --- | --- |
| `OWNER_PRIVACY=1` module × local soft | Private owner five-slot close | Payer | Existing full privacy module run |
| `CLOSE_ROLE=provider` module × local soft | Dedicated provider-close via `chainAction` | Payee | Thin — share `module-e2e.sh` body; branch only at close |
| `CLOSE_ROLE=provider OWNER_PRIVACY=1` module × local soft | PF six-slot private-owner non-signer provider-close (restores today’s only automated PF close evidence) | Payee | One-flag variation of the thin cell; single-host wallet holds both keys |
| Module negative × local | Asserted tokens only (`close_prestate_unavailable`, `close_provider_mismatch`, `close_args_mismatch`, `create_provider_equals_owner`); not reserved `close_owner_mismatch` | — | Short |
| Program + PP unit | Both instructions; public + PF + private-provider; wrong-length; create reject; golden wire words | both | `cargo test` |
| Real-prove privacy × local | New ImageID PP close under `RISC0_DEV_MODE=0` | Payer on `OWNER_PRIVACY=1` (D44.18) | Required one green; CLOCK_50 path; script already defaults pause/top-up off under real prove (D39.25) |

Make aliases: `verify-module-local-payee-close` and (if useful)
`verify-module-local-payee-close-privacy` for the PF provider-close cell.
Document in `docs/reference/verification-matrix.md` and `docs/journeys/E2E.md`.

`module-e2e.sh` `close_stream` phase JSON MUST emit `close_role`
(`owner`|`provider`), resolved `RISC0_DEV_MODE` (inherited by the logoscore
daemon the script starts), and `clock_account_id_hex` taken from what the
module actually used — not re-derived in the script from `RISC0_DEV_MODE`
(that would duplicate the dev-mode field and add no evidence). Prefer: module
`closeStream` ok payload echoes `clock_account_id_hex` next to `tx_hash`
(from `clockBytes`); alternatively read the clock id back from the submitted
tx. Soft four-cell evidence is otherwise indistinguishable after the fact.

### Step 44 gate-log row schema

Append-only table (same spirit as Step 39). One row per DoD cell that matters
for the ImageID flip and the soft square:

| Column | Content |
| --- | --- |
| Date | ISO date |
| Commit | git SHA |
| Cell | e.g. `module-local-soft-owner`, `module-local-soft-provider`, `module-local-soft-provider-pf`, `module-local-real-prove-pf-owner`, `module-local-negative`, `module-testnet`, `store-local`, `store-testnet` |
| Artifact | path under `.scaffold/e2e/artifacts/` |
| Result | pass / fail |
| ImageID | hex |
| `RISC0_DEV_MODE` | `0` or `1` |
| Close role | `owner` or `provider` (from phase JSON) |
| Clock | `clock_account_id_hex` from module ok payload (or submitted tx), not script-side re-derive |

Real-prove row: CLOCK_50 in that module-echoed (or tx-read) clock field is
direct evidence soft stubs were not used (`clockBytes` picks CLOCK_50 only when
`RISC0_DEV_MODE=0`).

Optional / not Step 44 DoD:

- `PROVIDER_PRIVACY=1` module × local — claim privacy; close = payer unless
  explicitly overridden.
- Privacy × testnet (including `RISC0_DEV_MODE=0`) — not DoD (D44.18 option A);
  local real-prove is the ImageID prove gate.
- `CLOSE_ROLE=provider` × testnet — optional only; not DoD.

### Explicit non-goals

- No `CLOSE_ROLE` × privacy × chain full product on Store or testnet.
- No second stream inside the primary User Journey run.
- `PROVIDER_PRIVACY` does not select close role by default.
- Soft proving on public testnet remains invalid (D39.4).

## Landing checklist (hard cut blast radius)

Same change set must touch all of:

- Guest SPEL source order MUST be
  `… top_up_stream, close_stream_by_owner, claim, close_stream_by_provider`
  (IDL order only; dispatch is name-based)
- Core `Instruction` enum in the same order + `instruction_wire.rs` round-trip
  **and** golden leading-word asserts (7 / 8 / 9)
- `instruction_accounts.rs` planners + layout tests; `lib.rs` re-exports
- `program_tests/common.rs` (`signed_close_stream`, account-type aliases),
  `close_stream.rs` (both modes + PP + wrong-length),
  `pp_common.rs` (`PpClaimCloseSetup` docs / PF provider-close helper)
- FFI `instruction_abi.rs` + regenerated `lez_payment_streams_ffi.h`
- Module bridge `payment_streams_ffi_bridge.h` +
  `payment_streams_ffi_bridge_writes.c`
- Module aliases / `closeStream` dispatch + create `owner==provider` reject +
  locked error tokens + single vault-config thread into submit context +
  empty-value checks before id helpers + no absent-authority coercion +
  `closeStream` ok payload echoes `clock_account_id_hex` (for S4 gate evidence)
- `examples/src/bin/seed_localnet_fixture.rs` (`close-stream-onchain` →
  provider-path instruction) **and** Store orchestrator
  `scripts/e2e/run_local_e2e.py` `close_body` / `e2e_close_via` branches
- `scripts/module-e2e.sh` close call, narration, `CLOSE_ROLE` switch, and
  `close_stream` phase JSON fields (`close_role`, `RISC0_DEV_MODE`, clock)
- `make idl` → `lez-payment-streams-idl.json` — expect account rename on
  provider-close slot 4: guest param `authority` → `provider` (intentional;
  not an accidental IDL rename in review)
- Old name sweep: no `CloseStream` / `close_stream` instruction symbol left in
  guest, core enum, IDL, FFI, seed, or `program_tests/mod.rs`; update
  `vault.rs` doc comments that still say `close_stream`
- Living privacy account / command docs:
  `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` close command row;
  `docs/on-chain/README.md` loader-to-instruction map;
  integration-contracts (or PRIVACY_ENHANCED_JOURNEY) for the two new close
  visibility rows (do not leave Step 36 historical table as the only SSOT)
- Pins/fixtures/docs listed under [Deploy and pin flip](#deploy-and-pin-flip)
- Step 44 gate log file using the row schema above

## LIP-155 encoding note

Either-party close authorization unchanged. Role distinctness is new (D44.16):
vault owner and stream `provider_id` MUST NOT be the same account id.

Canonical repo for the PR: `logos-lips` (local `rfc-index` tracks the same file).
Open the PR after implementation greens; sync `rfc-index` to match. Use a
separate branch off `logos-lips` `master` (Step 40 `#397` is merged; no stack).
Do not leave the note as a Step 40 deferral of ownership.

Minimum edit set (in scope once PR is cut) — L2 locked (D44.8):

- LEZ reference table: replace the single `CloseStream` row with
  `CloseStreamByOwner` (authorizer: vault owner) and
  `CloseStreamByProvider` (authorizer: stream provider), aligned with guest
  names.
- Split the coupled MUST: `Claim` keeps “owner as explicit non-signing”;
  provider-close keeps that MUST; owner-close states owner is the signing
  authorizer (five-slot), not non-signing.
- Either-party prose (~L231) stays (one semantic Close op; two LEZ reference
  instructions).
- Sequence-diagram / extension prose keep operation-level “close” wording.
- Solidity appendix `closeStream` stays a single EVM-style function: EVM has no
  per-call account-id uniqueness constraint; `msg.sender` selects the
  authorizer at runtime. Optional one-line note that LEZ uses two reference
  instructions.
- Automatic-claim-on-close extension stays as a single semantic op (no split);
  add a one-line LEZ note that the extension’s close maps to either LEZ
  reference instruction depending on authorizer.
- Role distinctness (D44.16): stream create MUST NOT set `provider_id` equal
  to the vault owner account id (same id for both roles). Place next to create
  / stream role prose; LEZ guest and module enforce at create.

Schedule: open/land manually after guest, module, tests, and E2E for both modes
are green.

## Privacy

See [Public and private execution (no regression)](#public-and-private-execution-no-regression)
and [Testing structure (layered)](#testing-structure-layered).

Required privacy tests:

- `test_pp_close_stream_by_owner_private_owner_succeeds` (five-slot; PF /
  `pp_owner_setup` pattern).
- Rename/re-point existing
  `test_pp_close_stream_private_provider_authority_succeeds` to
  `close_stream_by_provider` (public-tier vault + private provider).
- New `test_pp_close_stream_by_provider_private_owner_succeeds` on
  `vault_fixture_pseudonymous_funding_funded_via_native_transfer` /
  `pp_owner_setup` (private owner in non-signing six-slot — today’s missing
  unit coverage).
- PP assert for private-provider id-form match under module-comparable bytes.
- Wrong account-count program test per close path (guest panic /
  count-mismatch, not uniqueness).
- `OWNER_PRIVACY=1` module soft: payer-close.
- `CLOSE_ROLE=provider OWNER_PRIVACY=1` module soft: provider-close.
- One `RISC0_DEV_MODE=0` privacy cell on the new ImageID (D44.18):
  `OWNER_PRIVACY=1` payer-close. One cell suffices: the same module run still
  executes claim afterward, which is the six-slot instruction with a private
  non-signing owner on a PF vault — so that shape is re-greened under real
  prove without a separate provider-close real-prove cell. Script already
  defaults `MODULE_E2E_TOPUP` / `MODULE_E2E_PAUSE_RESUME` to 0 when
  `RISC0_DEV_MODE=0` (D39.25).

## Wire compatibility

Guest rebuild changes ProgramId / ImageID. That is the real hard cut.

`CloseStream{vault_id, stream_id}` and `CloseStreamByOwner{vault_id, stream_id}`
are byte-identical at index 7, so old instruction bytes decode cleanly as
owner-close on the new enum. Cross-version confusion is prevented by the new
ProgramId (and account-count mismatch if someone plans the six-slot list for
the five-slot instruction). Do not read “new binaries speak only the new pair”
as a decode-time guarantee on the words alone.

| Option | Behavior | Status |
| --- | --- | --- |
| A. Hard cut | Remove `CloseStream`. Add role variants; no legacy alias on new ImageID; ProgramId flip is the compatibility boundary | Chosen (D44.10) |
| B. Alias | Keep old name mapped to one role | Rejected |
| C. Dual decode | Accept old bytes as provider-close | Rejected |

## ImageID and deploy

See [Deploy and pin flip](#deploy-and-pin-flip). Step 44 unfreezes the guest
ImageID that Step 39 / Step 45 treat as frozen. Step 45’s baseline moves to
the Step 44 ImageID after this step lands (D44.19). Coordinate dependency bumps
with Step 45 only if a dep change is required to land the guest; ImageID churn
itself stays inside this step.

Step 39 privacy greens (`072a26cc…`, `RISC0_DEV_MODE=0`) are ImageID-scoped.
They are not evidence for the post-rebuild binary. Step 44 gate log supersedes
them for PP prove claims (D44.18). Do not reopen Step 39 Phase 1–3 soft greens
(D39.23); record a new prove green under the new id.

## Owner equals provider

Locked (D44.16). Self-provider streams (“open a stream to oneself”) are
rejected.

Create does not list provider as a second account meta, so
`provider == owner` does not fail uniqueness at create — the stream would
become `ACTIVE` and accrue. Failures appear later: `claim` and
`close_stream_by_provider` need distinct owner and provider slots and hit
`PreStateAccountIdsNotUnique`. After dual close, `close_stream_by_owner` could
still close, but accrued would remain unclaimable on LEZ — a funds lock.

LIP-155 today does not forbid same id for both roles. Product roles assume two
parties; LEZ claim/provider-close encodings require distinct ids.

Disposition:

- Guest `create_stream` MUST reject `provider == owner` (`ErrorCode` 6027 +
  unit test).
- Module `createStream` MUST also reject before submit when normalized `owner`
  and `provider` bytes are equal (`create_provider_equals_owner`).
- Same LIP PR (D44.8): stream create MUST NOT use the same account id for vault
  owner and `provider_id`.
- No `claim` redesign in this step.

## Deploy and pin flip

Sequence (record a Step 44 gate log, same spirit as Steps 32/33/39):

1. Docker / release guest build → `make program-id` → record new ImageID.
2. Update localnet fixtures and snapshot before any localnet E2E:
   `fixtures/localnet.json`, `fixtures/localnet-debug.json` (distinct id
   `16b95d37…` when used), `.scaffold/snapshots/funded/snapshot.json` via
   `make full-reset-localnet` (mandatory after ImageID change per
   feature-branch-pins).
3. Localnet gates green (program/PP tests, Required module, DoD privacy /
   `CLOSE_ROLE` cells, negatives, real-prove cell).
4. `make deploy-testnet` → sync files that hardcode the id as a default:
   `fixtures/testnet.json`, `fixtures/testnet-module.json`,
   `fixtures/testnet.json.example`,
   `scripts/bootstrap-testnet-module.sh`,
   `scripts/e2e/ensure-testnet-vault.sh`,
   `scripts/archive/bootstrap-testnet.sh`,
   `scripts/archive/create-testnet-stream-fixture.sh`,
   plus README / AGENTS / integration-decisions ELF-size or id lines.
5. Testnet gates → append gate log.
6. Touch pin-adjacent docs that cite id or ELF size (`AGENTS.md`, `README.md`,
   `integration-decisions.md`, `USER_JOURNEY.md` program-id / ELF-size lines).

Old ImageID (`072a26cc…` and any prior): PDAs belong to that program. This is
org testnet only — nothing is in production. Prefer a clean cut on redeploy:

1. Inventory known streams under the old id (open or `CLOSED` with
   `accrued > 0`) if any fixture/operator state still matters.
2. Drain or abandon them under the old id before flip if cheap; otherwise
   document abandonment. Do not keep the old program deployed long-term for
   emergency claim — redeploy the new guest and move fixtures/pins in lockstep.
3. New seed always targets the program id of the configured guest bin.

From guest rebuild until org testnet redeploy, `CHAIN=testnet` and journey users
expecting the published id are broken — do not advertise testnet journey green
until step 5 completes.

## Test coverage

Program / unit (public + privacy enumerated above):

- `close_stream_by_owner` succeeds; `close_stream_by_provider` succeeds.
- Unauthorized / mismatch fails per path.
- Wrong account-count fails per path.
- `create_stream` with `provider == owner` fails.
- Accounting (unaccrued release, closed state, later claim) on both paths.
- Golden wire indices 7 / 8 / 9.

Module / E2E:

- Required module local + testnet: payer-close.
- Required Store local + testnet: payee-close via default `chainAction`
  (`authority=provider`); keep seed branch working under `E2E_CLOSE_VIA=seed`.
- DoD: thin `CLOSE_ROLE=provider`; `OWNER_PRIVACY=1` payer-close;
  `CLOSE_ROLE=provider OWNER_PRIVACY=1`; localnet negatives (dispatch + create);
  one real-prove privacy cell.
- After testnet redeploy: manual re-walk of literal
  [USER_JOURNEY.md](../../journeys/USER_JOURNEY.md) Step 14 close (not only
  `module-e2e.sh`) — closes the Step 34 aspirational-doc gap.

## Implementation order

1. Core / guest — dual instructions, accounting helper only, wire indices as
   above; golden-word tests; `create_stream` rejects `provider == owner`
   (D44.16); exact attribute tables; `make idl`.
2. Program tests — both close modes + reject + wrong-length + PP owner-close,
   PP provider-close (public-tier private provider), PP provider-close on PF
   private owner; create self-provider reject.
3. FFI — serialize/plan pairs; regenerate `lez_payment_streams_ffi.h`; update
   module bridge `.h` / `.c`.
4. Module — D44.7 + D44.11 pre-read dispatch; create `owner==provider` reject;
   locked error tokens; `StreamOwner5` / `StreamProvider6` submitter wiring
   (Step 47 already renamed `StreamAuthority6` → `StreamProvider6`);
   thread decoded vault config into submit context (M3).
5. Seed / examples + Store orchestrator `close_body` — provider-path instruction
   name on both branches.
6a. Localnet pin flip — guest build, `make program-id`, fixture +
    `full-reset-localnet` snapshot (must precede E2E).
6b. Localnet E2E — Required module payer-close; thin `CLOSE_ROLE=provider`;
    `OWNER_PRIVACY=1` payer-close; `CLOSE_ROLE=provider OWNER_PRIVACY=1`;
    negative rejects; real-prove cell; Make aliases + verification-matrix +
    E2E.md rows.
7. Deploy-testnet + testnet fixture sync + gate log; drain or abandon old-id
   streams (clean cut).
8a. Normative docs — integration-contracts rewrite (dual instructions; rewrite
    the six-account owner-as-authority paragraph); living privacy close rows.
8b. Mechanical docs — USER_JOURNEY, E2E.md (document `CLOSE_ROLE`; owner-close
    default after this step), PRIVACY_ENHANCED_JOURNEY close command,
    on-chain README loader map, module README, verification-matrix;
    LEZ `#[account(signer)]` vs module `owner`/`provider` JSON (Step 47 already
    hard-cut living rename — do not reintroduce `signer`/`authority` keys);
    `docs/store-integration/README.md` no-op for close; spel/`make cli` auto;
    Step 45 ImageID baseline note.
9. Manual USER_JOURNEY re-walk on testnet.
10. LIP — small `logos-lips` PR (after greens).

## Scope

In scope:

- Guest / core / FFI (including module bridge) / module / seed / Store
  orchestrator / IDL hard cut for both close paths.
- Pre-submit on-chain identity reads for strict dispatch (D44.11).
- Layered test plan (unit/PP full; module soft close×owner-privacy square;
  sparse Store/real-prove); deploy/pin flip + gate log; USER_JOURNEY re-walk.
- Guest `create_stream` and module `createStream` reject `provider == owner`
  (D44.16); LIP MUST NOT in the same logos-lips PR.
- Thin `CLOSE_ROLE=provider` (+ PF variation) local module cells + Make aliases.
- One real-prove privacy green on the new ImageID (D44.18).
- Small LIP-155 LEZ encoding + role-distinctness PR (close policy either-party
  unchanged).
- Landing checklist completeness; Step 45 ImageID baseline move note.

Out of scope:

- Changing LIP either-party close authorization (either party may close remains).
- Redesigning `claim` for owner==provider (forbidden at create instead).
- Full `CLOSE_ROLE` × privacy × chain combinatorial matrix on Store/testnet.
- Step 45 dependency bumps unless required to land the guest change.
- Basecamp UI (Step 21).
- Keeping a deprecated `CloseStream` wire alias on the new ImageID.
- Merging stream context loaders; spel IDL path raw-todo.
- Renaming module JSON `signer`/`authority` → `owner`/`provider` ([Step 47](step-47-unify-role-terminology.md)
  — already implemented on `feat/step-47-unify-role-terminology`; land/merge
  before or with this step’s docs so implementors do not write legacy keys).

## Verification

- Both close paths succeed in program/PP tests (including PF provider-close
  unit); payer-close on Required module local + testnet after redeploy;
  payee-close on Required Store `chainAction` and on thin
  `CLOSE_ROLE=provider` module × local (+ PF variation).
- Privacy / PP: PP owner-close + provider-close (public-tier and PF); soft
  `OWNER_PRIVACY=1` payer-close; soft PF provider-close; one real-prove privacy
  green on the new ImageID; module `PROVIDER_PRIVACY` does not flip close role
  by default.
- Mismatched module args and prestate-unavailable reject without submitting;
  asserted tokens only in the negative cell (D44.21); create
  `provider==owner` covered there.
- Golden wire words 7 / 8 / 9; wrong account-count tests per path.
- Old single-`CloseStream` / `close_stream` instruction name absent from new
  IDL / guest / FFI / seed / `program_tests/mod.rs` (and `vault.rs` comments);
  `Claim` discriminant unchanged (index 8); IDL provider-close slot 4 named
  `provider`.
- Gate log uses the Step 44 row schema (cell, ImageID, `RISC0_DEV_MODE`,
  close role); phase JSON distinguishes soft-square cells; real-prove row
  shows CLOCK_50; Step 39 prove supersession noted.
- Docs match behavior; USER_JOURNEY Step 14 re-walked on testnet;
  Make aliases in verification-matrix and E2E.md.
- LIP PR opened after greens.

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D44.1 | Packet ownership | Step 44; product fix, not script-only polish. |
| D44.2 | Root cause | LEZ account-id uniqueness + six-slot owner+authority layout; not localnet vs testnet. |
| D44.3 | Both paths | Payer-close and payee-close must both work after this step. |
| D44.4 | E2E coverage | Payer-close primary on Required module; provider-close via thin local `CLOSE_ROLE=provider` (+ PF variation) + Required Store default `chainAction` payee (D44.15). |
| D44.5 | Instruction shape | Two guest instructions; reuse five-slot owner-stream and six-slot close flags (not silent claim `mut` on provider). |
| D44.6 | Naming | `close_stream_by_owner` and `close_stream_by_provider`. |
| D44.7 | Module API | Single `closeStream`; omit / matching owner → owner path; distinct provider `authority` → provider path; reject mismatches; comparisons on normalized 32-byte ids. |
| D44.8 | LIP | Separate branch off `master`; L2 reference ops; Claim MUST split; auto-claim extension stays one op + one-line LEZ map; MUST NOT same owner/`provider_id` (D44.16); Solidity `closeStream` stays single (EVM has no account-id uniqueness). |
| D44.9 | Seed / Store | Required Store teardown is default `chainAction` payee-close (`authority=provider`); seed remains via `E2E_CLOSE_VIA=seed`; both branches migrate. |
| D44.10 | Wire enum | Hard cut; core enum owns discriminants; guest source order is IDL-only; Claim@8 unchanged; golden-word tests required. ProgramId flip is the compatibility boundary (index-7 bytes are identical to old `CloseStream`). |
| D44.11 | Dispatch identity | Pre-read public vault PDA before plan. Stream pre-read only on provider path; owner path skips stream pre-read (state-existence stays in guest for sync-lag robustness). Pre-submit rejects = identity/dispatch only. No PF skip-pre-read exception. No in-module sync loop. |
| D44.12 | Deploy | Pin-flip + gate log (self-describing rows: cell, ImageID, dev-mode, close role); testnet clean cut — drain or abandon old-ImageID streams; no long-lived old-program keep-alive (not production). Localnet fixture/snapshot reset before localnet E2E. |
| D44.13 | Shared helper | `close_at_time` + total_allocated release + writebacks for vault_config and stream_config; loaders/output builders stay per-path; `top_up_stream` is the five-slot vault_config write precedent. |
| D44.14 | Privacy tests | PP owner-close + provider-close on public-tier and PF; soft `OWNER_PRIVACY=1` payer-close; soft `CLOSE_ROLE=provider OWNER_PRIVACY=1`; `PROVIDER_PRIVACY` does not select close role by default. |
| D44.15 | Reduced matrix | No Store/testnet close-role axis; module soft local DoD is the four-cell close×owner-privacy square; sparse real-prove. |
| D44.16 | Owner ≠ provider | Guest + module reject `provider == owner` before/at create; LIP MUST NOT same id (same LIP PR); ErrorCode 6027. |
| D44.17 | PP no-regression | Both close paths work public and private; PF uses public PDA pre-read + existing private-submit routing; evidence includes PF provider-close unit + soft E2E; host must hold private owner NSK to submit PF provider-close (M10). |
| D44.18 | Real-prove ImageID | DoD = one local `RISC0_DEV_MODE=0` `OWNER_PRIVACY=1` payer-close cell. Same run’s later claim re-greens six-slot PF private non-signing owner under real prove. Testnet privacy re-run is not DoD. Step 39 prove greens are ImageID-scoped and superseded by the Step 44 gate log. |
| D44.19 | Step 45 baseline | Step 45 ImageID freeze moves to the Step 44 ImageID after this step; Step 44 does not wait on Step 45 dep work. |
| D44.20 | Module JSON naming (N2) | Docs-only in this step: `signer` means vault owner id; on provider-close the tx signer is `authority`. No rename/alias here. Full `owner`/`provider` rename is [Step 47](step-47-unify-role-terminology.md). |
| D44.21 | Reject tokens | Asserted: `close_prestate_unavailable`, `close_provider_mismatch`, `close_args_mismatch` (empty/whitespace `signer` or empty `authority` key — checked before id helpers), `create_provider_equals_owner`. Reserved: `close_owner_mismatch`. Catch-all: unmatched combinations reject, never coerce. |

## Open for discussion

None remaining. Locked since prior passes: four-cell soft square (D44.15); M1
option (a) (D44.11); N2 docs-only → Step 47 (D44.20); D44.18 option A (local
real-prove DoD only).

## Done when

- Both close paths work on chain (public + private); Required 2×2 greens;
  soft module close×owner-privacy square greens; Store default payee
  `chainAction`; seed branch still works under `E2E_CLOSE_VIA=seed`.
- Make aliases for payee-close (and PF payee-close if split) referenced in
  verification-matrix and E2E.md.
- Locked reject tokens asserted per D44.21 (not reserved `close_owner_mismatch`);
  create reject in the negative cell; landing checklist complete (including
  module FFI bridge, orchestrators, old-name sweep, IDL slot-4 rename note);
  IDL/`Claim` index golden-tested.
- One local real-prove privacy green on the new ImageID (D44.18 option A);
  gate-log row self-describing; Step 39 prove supersession recorded.
- Deploy/pin flip + gate log; old-ImageID streams drained or abandoned (testnet
  clean cut); testnet greens after redeploy; Step 45 baseline note landed.
- USER_JOURNEY Step 14 re-walked on testnet; docs match behavior (8a/8b),
  including PRIVACY_ENHANCED_JOURNEY close command and living visibility rows;
  `signer`/`authority` vs LEZ signer clarified (D44.20); rename deferred to
  Step 47.
- LIP PR opened on `logos-lips` after greens (L2 + MUST NOT same id).
- No open design discussion items remaining.
