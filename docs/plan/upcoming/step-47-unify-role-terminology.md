# Step 47 — unify role terminology

Upcoming. Index: [index.md](../index.md).

Spun out of Step 44 N2. Discuss and implement separately from the close hard cut.

Land implementation on a feature branch (one PR; multiple commits per D47.9).
Do not commit the rename series directly on `master`.

## Goal

Align living module API, helpers, journeys, policy identifiers, and related
layout/Store names with LIP-155’s layered roles, and reserve **signer** for LEZ
must-sign only.

After this step:

1. Module JSON and module-owned code use vault **owner** and stream **provider**.
2. **signer** means only the LEZ account-meta / tx-signing concept
   (`#[account(signer)]`, “this account must sign”).
3. Informal **payer** / **payee** (and journey `PAYER` / `PAYEE`) are retired from
   living surfaces in favor of the map below (meta-usage only; see D47.15).
4. Policy / FFI symbols and living prose that say `payee` / `payer` for chain or
   service roles become `provider` / `user` (D47.13).
5. Six-slot layout types use `StreamProvider*` (D47.16).
6. Store verify param `requesterPeerId` → `userPeerId` (D47.17).

## Role layers (do not collapse to one word)

LIP-155 already separates mechanism from the service use case. The accounting
model (vault, allocation, accrual, claim) is generic; the normative Roles
section and the product focus are user–provider for Logos request-response
services. Journeys added a fourth informal pair (`payer` / `payee`) that never
appears on the wire and duplicates both layers.

Unify by documenting the map and retiring the informal pair — not by inventing a
single global synonym. Collapse pressure is illusory: privacy already broke 1:1
(`funder` shields into a private `owner`; service `user` and on-chain `owner`
are related, not identical). Collapsing toward `owner` drags PDA vocabulary into
Store-session narrative; collapsing toward `user` would rewrite LIP Roles
(out of scope).

| Layer | Terms | Use for |
| --- | --- | --- |
| Service / protocol (LIP Roles, Store hosts) | `user`, `provider` | Who requests service vs who delivers. `provider` is the same party as the chain-layer provider (cross-layer term). |
| On-chain identity (guest, module JSON, manifests) | `owner`, `provider` | `VaultConfig.owner`, `StreamConfig.provider` / claim account. `provider` denotes the same party as the service-layer provider. |
| Tx submitter (derived) | `submitter` | Who signs/submits this tx, per D47.6 precedence. Not a module JSON key. |
| Privacy funding (ops only) | `funder` | Public account that shields into a private owner — not the vault owner |
| Informal (retire in living docs/code) | `payer`, `payee` | Historical journey slang; do not use in new code or living docs |

In the primary use case they coincide:

```text
LIP user     ≈  vault owner  ≈  module/JSON owner
LIP provider ≈  stream provider / claim account ≈  module/JSON provider
submitter    =  owner or provider per D47.6 (op-dependent)
```

Prose rule:

- Chain ops and module examples → `owner` / `provider`.
- Service / eligibility / Store session narrative → `user` / `provider`.
- Do not keep `payer` / `payee` “for genericity.” Genericity is already
  owner + `provider_id`; the product vocabulary remains user–provider.

Allowlisted `payer`/`payee` only (D47.15):

- This packet’s “Informal (retire)” row and decision text.
- One “formerly `payer`/`payee`” pointer line in naming-conventions (muscle-memory
  greps land on the map).
- Path-allowlisted historical records (`completed/`, `wontfix/`, `archive/`).

LIP-155 normative text is out of scope (no user→payer rewrite).

Draft this table into [naming-conventions.md](../../reference/naming-conventions.md)
in commit 1 (include the formerly-pointer line).

## Problem

Pre–Step 47 (and after Step 44’s dual close lands): write ops still use JSON
`signer` for the vault owner account id (PDA derivation). That was a local
module convention, not a LIP or LEZ wire term.

On provider-close, JSON `signer` remains the vault owner id while the
transaction submitter is JSON `authority` (the provider) — the LEZ `signer`
collision that motivated this packet. `claim` and reads already use `owner` /
`provider` (claim keys are not part of the JSON rename; only close’s
`authority` → `provider` and write `signer` → `owner`).

USER_JOURNEY and helpers still use `PAYER` / `PAYEE` for those account ids.
Policy / FFI still name provider bindings `*payee*` and use living `payer`
prose. Layout types still say `StreamAuthority*`. Store verify still says
`requesterPeerId`.

## Target vocabulary

| Role / concept | Preferred | Avoid |
| --- | --- | --- |
| Vault owner account id (JSON, C++, journey env) | `owner` / `OWNER` | `signer`, `PAYER`, `payer` |
| Stream provider account id (JSON, C++, journey env, policy) | `provider` / `PROVIDER` | close `authority`, `PAYEE`, `payee` (when meaning provider) |
| Tx submitter (derived; helpers may say `submitter*`) | per D47.6 | Do not name owner-id helpers `signer*` |
| Withdraw payout destination | `withdraw_to` (unchanged) | — |
| Privacy funding account | `funder` / `PUBLIC_FUNDER` (unchanged) | Do not rename to owner |
| Store verify peer (service user) | `userPeerId` | `requesterPeerId` |
| Six-slot account layout types | `StreamProvider*` / `StreamProvider6` | `StreamAuthority*` / `StreamAuthority6` |
| LEZ account must-sign | guest attrs / LEZ prose only | Module owner helpers named `*Signer*` / `signer*` |

Submitter-selection rule (D47.6): op-scoped precedence **`provider` > `owner`**
for ops where `provider` is a submitter candidate:

| Op | Submitter |
| --- | --- |
| `closeStream` with distinct `provider` | provider |
| `closeStream` owner-only (no `provider` key) | owner |
| `claim` | provider |
| `createStream` and other owner writes | owner (`provider` on create is instruction data only) |

A future third submitter role extends this table; do not re-litigate a fuzzy
“most specific” phrase.

Close after rename (illustrative):

- Owner-close: `{"owner":"<vault_owner>","vault_id":…,"stream_id":…}`
- Provider-close: `{"owner":"<vault_owner>","provider":"<provider>","vault_id":…,"stream_id":…}`

Other writes: `signer` → `owner`. Keep `withdraw_to` as the optional payout
destination (already a clear role name; avoids needing `payee` as English for
payout destination).

Internal helpers: full rename in this step (D47.8), e.g.
`signerAccountIdHex` → `ownerAccountIdHex`,
`ownerBytesFromSignerField` → `ownerBytesFromOwnerField` (or similar),
privacy `depositSigner*` / `enforceDepositSignerEqualsOwner` → owner wording,
and `signingRequirementsForAccounts(..., signerHex)` → `submitterHex` (DoD —
keeps the terminology gate mechanical with no meaning-triage exception for that
param; it already means LEZ must-sign / tx submitter under D47.3, not vault
owner).

Guest / wire / FFI:

- Keep comments that correctly mean LEZ `#[account(signer)]`.
- Policy / FFI `*payee*` → `*provider*` and living `payer` prose → `user`
  (D47.13).
- `StreamAuthority*` / `StreamAuthority6` → `StreamProvider*` / `StreamProvider6`
  (D47.16; DoD after Step 44).

Id formats (D47.7): unify account-id write (and related) params on base58 **or**
64-hex via one helper. Detection rule (fixed order):

1. If the trimmed input is exactly 64 hex characters (`[0-9a-fA-F]{64}`) → decode
   as hex to 32 bytes.
2. Else → decode as base58; require decoded length 32.
3. Otherwise → error.

Do not “try both” without this order: a 64-char hex string without digit `0` can
be charset-valid base58, so two call sites could diverge on the same input.

### Journey env rename (D47.14)

Hard-cut in `docs/journeys/USER_JOURNEY.md` and journey helpers
(`scripts/user-journey-auth-transfer.sh`, related journey scripts):

| Today | After |
| --- | --- |
| `PAYER` | `OWNER` |
| `PAYEE` | `PROVIDER` |

Rationale against `USER`/`PROVIDER`: POSIX `$USER` is the login name; a
walkthrough `export USER=""` clobbers an OS-set var and sits awkwardly next to
scaffold host vocabulary (`MODULES_USER`, `e2e/user/`). The vars hold chain
account ids for `chainAction` JSON and wallet commands, so chain-layer names are
right. `module-e2e.sh` already uses `$OWNER` / `$PROVIDER` shell locals.

Prose in the walkthrough: prefer LIP `user` / `provider` for the service story;
use `owner` / `provider` when describing vault/stream `chainAction` fields.
Also flip living journey siblings under `docs/journeys/` (e.g.
`PRIVACY_ENHANCED_JOURNEY.md`, `E2E.md`) — they still carry `signer` /
`authority` / payer–payee examples.

External spot-check list: see D47.4 (once).

### Policy `*payee*` / living `payer` prose (D47.13)

Include in this step (not deferred). Compiler-checked surface (private fn +
locals + one header doc comment; no cbindgen `payee` ABI). Commit-5 isolation
with focused `cargo test` is proportionate.

In the same commit, fix living `payer` prose in those files to service-layer
`user` (D47.11 would flag them anyway), e.g.:

- `lez-payment-streams-ffi/src/policy_abi.rs` (“runs on payer + provider”)
- `lez-payment-streams-core/src/stream_provider_policy.rs`
- `lez-payment-streams-core/src/policy/predicates.rs`

### Layout types `StreamAuthority*` → `StreamProvider*` (D47.16)

DoD in this step (after Step 44). After dual close, both six-slot users (claim,
provider-close) have the stream provider as the signing authority, so
`StreamProvider6` / `StreamProviderInstructionAccounts` is truthful everywhere.
Leaving “Authority” in core type names permanently recreates the debt class this
step removes. Touch at least:

- `StreamAuthorityInstructionAccounts` and alias `ClaimStreamInstructionAccounts`
- `stream_authority_instruction_accounts` / `authority_account_id` params
- module `VaultIxLayout::StreamAuthority6`
- FFI `plan_stream_authority_*`
- call sites (~10)

### Store verify peer (D47.17)

Hard-cut `requesterPeerId` → `userPeerId` on
`verifyEligibilityForStoreQuery` (impl, eligibility.cpp, error strings such as
`requesterPeerId is required`, contracts, Store orchestrator call sites).
Aligns with LIP Roles and symmetry with `providerPeerId`. Do not leave a fifth
informal vocabulary unexamined.

## Compatibility (D47.4 — hard cut)

Hard cut: no legacy aliases on living surfaces for any rename in this step
(`signer` / `authority` / `PAYER` / `PAYEE` / policy `*payee*` /
`requesterPeerId` / `StreamAuthority*`). Flip callers in the commits that change
each layer.

Deliberate posture: module JSON has no version/schema field, so renames in this
family are hard cuts by default — a decision, not an accident. Recorded once
here so future steps do not re-open “temporary aliases” without a new decision.

External spot-check at implement (record in PR / gate log):

- logos-docs User Journey (#370)
- Basecamp bindings if they serialize old keys or journey env names
- `logos-delivery` fork docs / hook glue that show write/`closeStream` JSON

## Multi-commit plan (D47.9)

Do the work on a feature branch. Full rename lands in **one Step 47 PR**, but as
**multiple commits**. After each commit, run at least the cheap regression for
that layer (unit / focused tests) before the next. Do not pile JSON + helpers +
docs into one unverified blob.

Suggested commit sequence (adjust if inventory finds a cleaner cut):

| Commit | Change | Verify after (minimum) |
| --- | --- | --- |
| 1 | Inventory note + naming-conventions role-layer table (incl. submitter row, provider double-duty, formerly payer/payee pointer, funder) | docs-only; no code gate |
| 2 | Unify id parsers (D47.7 detection order); wire withdraw / provider / close-authority / reads without renaming public JSON yet | Focused module/Rust tests as touched |
| 3 | Rename C++ helpers/params (`signer*` → `owner*` / role names; privacy deposit-owner helpers; `submitterHex` DoD) | Same + grep no owner-id `signer*` helpers |
| 4 | Hard-cut JSON keys + scripts; `StreamProvider*` + `ClaimStream*` layout rename (D47.16); `userPeerId` (D47.17); **rebuild and reinstall** module `.lgx` (`SKIP_BUILD=1` forbidden) | Required rename smoke (below) |
| 5 | Policy / FFI `*payee*` → `*provider*` + living `payer` prose → `user` (D47.13) | Focused `cargo test` on core policy + ffi |
| 6 | Journeys (`USER_JOURNEY`, `PRIVACY_ENHANCED_JOURNEY`, `E2E`, helpers): `PAYER`/`PAYEE` → `OWNER`/`PROVIDER` (D47.14); contracts / store-integration / glossary | `scripts/check-terminology.sh` |
| 7 | Final terminology script + D47.4 external spot-check note; wire script into `scripts/README.md` (and a Make target if one already fits) | Full gate; re-run Required smoke if commit 4 was not that run |

Rule: after every code commit, at least unit/focused tests green before the next
commit.

### Required rename smoke (D47.9) — not a bare default public run

Thinning vs Step 44 still holds: no Store/testnet/privacy matrix (retest
overlap). Forbidding `SKIP_BUILD=1` remains the highest-value clause.

But a default public `MODE=module` run does **not** parse every renamed write
key: `MODULE_E2E_TOPUP` / `MODULE_E2E_PAUSE_RESUME` default to 0; there is no
withdraw phase; post–Step 44 default close is owner-role (close `provider` key
absent). Each op has its own `qv(...)` site, so a per-op typo is exactly what
the smoke must catch.

Required smoke (one local public fresh-build run, or one continuous scripted
sequence with the same install):

1. `SKIP_BUILD` unset / forbidden; rebuild and reinstall `.lgx`.
2. `MODULE_E2E_TOPUP=1 MODULE_E2E_PAUSE_RESUME=1` so `pauseStream` /
   `resumeStream` / `topUpStream` hit new `owner` keys.
3. One `CLOSE_ROLE=provider` close so closeStream’s `provider` key is parsed
   (thin cell Step 44 already defines).
4. One `withdraw` `chainAction` (small new phase or appended call in the same
   run) so `owner` / `withdraw_to` parse sites are exercised.

Default-covered ops (`initializeVault`, `deposit`, `createStream`) remain in
that run. `claim` already uses `owner`/`provider` keys — not a rename smoke
requirement (lifecycle coverage stays Step 44). Helper renames stay
compiler-checked; parser unification stays unit/focused-test territory.

| Gate | Step 47 DoD? |
| --- | --- |
| Unit/focused after each code commit | Yes |
| `scripts/check-terminology.sh` (D47.11) | Yes |
| Fresh-build Required rename smoke (flags + provider-close + withdraw above) | Yes — Required |
| Store local public | No — optional only if script gate is inconclusive |
| Testnet / privacy / real-prove | No — Step 44 / later pin bumps |

### Terminology gate script (D47.11)

Encode the gate as `scripts/check-terminology.sh` (**write in this step**; does
not exist yet — patterns + allowlists in code). Prose-only “meaning allowlist”
decays; a script is rerunnable by Step 46 and later work.

Mechanical rules (non-exhaustive; script is SSOT):

- Quoted literals `"signer"` and `"authority"` in
  `logos-payment-streams-module/src` → count **0** after JSON flip (catches
  parse-site typos meaning-triage would miss).
- No living `PAYER` / `PAYEE` / non-allowlisted `payer` / `payee`.
- No module-owned owner-id helpers named `signer*` / `*Signer*`.
- Allowed: guest `#[account(signer)]`, LEZ must-sign prose, `funder` /
  `PUBLIC_FUNDER`, meta-usage in this packet and the naming-conventions
  formerly-pointer, path allowlists below.

Path allowlist:

- `docs/plan/completed/`
- `docs/plan/wontfix/`
- `docs/archive/`
- This Step 47 packet (self-allowlist; it must discuss retired words)
- Gate-log / artifact trees under `.scaffold/` (if grepped at all)

Assumption at gate time: Step 44’s packet has moved to `docs/plan/completed/`
(D47.5 waits on USER_JOURNEY re-walk + ImageID cut). If 47’s gate runs while 44
still sits in `upcoming/`, either move/complete 44 first or temporarily
path-allowlist that packet — do not fail the gate on 44’s historical
payer/payee tables.

Scan at least: `logos-payment-streams-module/`,
`lez-payment-streams-core/src/policy/`,
`lez-payment-streams-core/src/stream_provider_policy.rs`,
`lez-payment-streams-core/src/instruction_accounts.rs` (and layout call sites),
`lez-payment-streams-ffi/`, `scripts/`, `docs/journeys/`, `docs/reference/`,
`docs/store-integration/`, `docs/on-chain/`, `docs/plan/upcoming/` (minus
self-allowlisted 47 / any temporary 44 allowlist), root `README.md` /
`AGENTS.md`, module README.

Missing-key tokens after rename (D47.12):

| Op | Missing/empty owner (or empty role key) token |
| --- | --- |
| `closeStream` | `close_args_mismatch` (D44.21; empty/absent `owner`, empty present `provider`) |
| Other writes (`initializeVault`, `deposit`, `withdraw`, `createStream`, `pauseStream`, `resumeStream`, `topUpStream`) | shared `args_mismatch` |

## Scope

In scope:

- Module `chainAction` JSON param names for writes + close.
- Module C++ / bridge helper and parameter renames (full; D47.8), including
  privacy deposit-owner helpers and `submitterHex`.
- Unified account-id parsing with fixed hex/base58 detection (D47.7).
- `StreamProvider*` layout rename (D47.16).
- `userPeerId` rename (D47.17).
- Policy / FFI `*payee*` → `*provider*` and living `payer` → `user` (D47.13).
- Journey `PAYER` / `PAYEE` → `OWNER` / `PROVIDER` (D47.14).
- `scripts/check-terminology.sh` + Required rename smoke (D47.9 / D47.11).
- Scripts, contracts, naming-conventions role-layer glossary, store-integration /
  Developer Journey examples (Step 20).
- Inventory of other module JSON surfaces — “looked, left” only after explicit
  decide (no silent fifth vocab).

Out of scope:

- LIP-155 normative rename (keep LIP `user` / `provider` Roles).
- Guest instruction renames (`close_stream_by_owner` / `_by_provider` stay).
- Changing LEZ `#[account(signer)]` semantics.
- Step 44 ImageID / dual-instruction work.
- Renaming `withdraw_to` (already role-correct).
- Renaming `funder` / `PUBLIC_FUNDER` / `E2E_PUBLIC_FUNDER` (distinct privacy role).
- Localnet state `SIGNER_ID` (fixture/Makefile debt; inventory note only —
  contained at state-file boundary; scripts already alias to local `OWNER`).

## Relation to other steps

- Step 44: docs-only clarification until this step. Land Step 47 **after**
  Step 44’s USER_JOURNEY testnet re-walk and packet move to `completed/`
  (D47.5). Prefer owner-close / provider-close in new prose; packet filename may
  stay historical.
- Step 20: store-integration / Developer Journey module-call examples.
- Step 21: Basecamp bindings if they serialize old keys or journey env names.
- Step 46: prefer final names; rerun `scripts/check-terminology.sh`.

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D47.1 | Packet ownership | Step 47; terminology / API consistency, not close mechanics. |
| D47.2 | Target roles | Module JSON uses `owner` and `provider`; drop `signer`/`authority` as public keys. |
| D47.3 | LEZ term boundary | “signer” only for LEZ must-sign; module-owned code must not use `signer` for vault owner. |
| D47.4 | Compatibility | Hard cut — no aliases; future renames in this family are hard cuts by default (no JSON schema version). Spot-check logos-docs #370, Basecamp, logos-delivery fork. |
| D47.5 | Ordering vs Step 44 | After Step 44 USER_JOURNEY testnet re-walk, ImageID cut, and 44 packet in `completed/` (or temporary path-allowlist). |
| D47.6 | Submitter rule | Op-scoped: close/claim use precedence `provider` > `owner`; `createStream.provider` and other owner writes keep owner as submitter. |
| D47.7 | Id formats | One helper; fixed order: 64 hex chars → hex/32 bytes, else base58/32 bytes, else error. |
| D47.8 | Internal rename | Full helper/param rename; privacy deposit-owner helpers; `submitterHex` DoD (mechanical gate, no meaning-triage exception). |
| D47.9 | Landing shape / smoke | Feature branch → one PR, multiple commits; Required E2E = fresh-build rename smoke with `MODULE_E2E_TOPUP=1 MODULE_E2E_PAUSE_RESUME=1`, `CLOSE_ROLE=provider`, and one `withdraw`; `SKIP_BUILD=1` forbidden. No Store/testnet/privacy matrix. |
| D47.10 | `withdraw_to` | Keep name; only rename withdraw’s vault-owner JSON `signer` → `owner`. |
| D47.11 | Terminology gate | `scripts/check-terminology.sh`; zero-count `"signer"`/`"authority"` in module src after flip; self-allowlist this packet; path allowlist historical trees. |
| D47.12 | Missing-key tokens | close → `close_args_mismatch`; other writes → shared `args_mismatch`. |
| D47.13 | Policy rename | Required: `*payee*` → `*provider*` and living `payer` prose → `user` in same commit; focused cargo tests. |
| D47.14 | Journey env | `PAYER`/`PAYEE` → `OWNER`/`PROVIDER`; not `USER` (POSIX `$USER` + scaffold host collision). |
| D47.15 | Role-layer principle | Keep dual service vs chain vocab; `provider` cross-layer; add `submitter` derived row; retire living payer/payee except meta-usage allowlist; keep funder; do not collapse LIP Roles. |
| D47.16 | Layout types | `StreamAuthority*` → `StreamProvider*` DoD (post–Step 44). |
| D47.17 | Store verify peer | `requesterPeerId` → `userPeerId` hard cut. |

## Open for discussion

None remaining after critique pass. At implement time only: exact helper symbol
spellings and whether commit 3–4 boundaries bisect cleaner.

## Done when

- Public write/close JSON uses `owner` / `provider` (no legacy aliases).
- Module-owned helpers/params no longer use `signer` for vault owner;
  `submitterHex` (or equivalent) names the tx-submitter helper param (D47.8);
  remaining “signer” mentions are LEZ must-sign only under
  `scripts/check-terminology.sh` (D47.11).
- `StreamProvider*` layout names landed (D47.16); `userPeerId` landed (D47.17).
- Policy / FFI no longer use `*payee*` / living `payer` for those roles (D47.13).
- USER_JOURNEY and journey helpers use `OWNER` / `PROVIDER` (D47.14); living
  payer/payee prose retired per D47.15 allowlist.
- naming-conventions carries the role-layer table + formerly pointer (D47.15).
- Submitter rule documented as op-scoped table (D47.6) and matches close/claim.
- Id parsing unified per D47.7 detection order; documented in naming-conventions.
- Docs and scripts match; inventory note for `SIGNER_ID` only.
- Missing/empty owner keys use Step 44-style tokens (D47.12).
- Multi-commit history shows per-commit verification; JSON-flip commit verified
  with fresh module build and the Required rename smoke (D47.9).
- `scripts/check-terminology.sh` passes; Store local optional only (not DoD).
- D47.4 external spot-check recorded.
