# Step 34 — plan excerpt

Completed. Index: [index.md](../index.md).

In-repo User Journey as a testnet manual CLI walkthrough.
Published logos-docs packet (Step 22):
[logos-docs#370](https://github.com/logos-co/logos-docs/issues/370).
Does not replace completed Step 28; does not change Developer Journey (Step 20).

### Step 34, User Journey manual walkthrough (testnet CLI)

Prerequisite: Steps 26–28 complete (testnet v0.2 module E2E); one-time
`make bootstrap-testnet-module` on the machine (or equivalent wallet + `fixtures/testnet-module.json`
layout — see E2E.md). Step 22 is the logos-docs packet
([logos-docs#370](https://github.com/logos-co/logos-docs/issues/370));
automated verification recipes live in [E2E.md](../../journeys/E2E.md).

Independent of Store claim gate work (Step 32, now complete): USER_JOURNEY and
module×testnet E2E content only.

#### Problem (historical)

[USER_JOURNEY.md](../../journeys/USER_JOURNEY.md) documented `make verify-module-*` and artifact
JSONL, not hands-on commands. End users needed a step-by-step path that shows LIP-155 payment streams
as a payment mechanism on the deployed testnet program, without Store or eligibility.

#### Architectural context

| Track | Audience | Doc | Chain |
| --- | --- | --- | --- |
| User Journey walkthrough | End user learning payment streams | USER_JOURNEY.md | **Testnet v0.2 only** |
| E2E verification | Maintainers and integrators re-running gates | E2E.md | Localnet + testnet, 2×2 matrix |
| Developer Journey | Store integrators | DEVELOPER_JOURNEY.md | Unchanged in this step |

N18: module-only user story stays separate from Store integration
([N18](../../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)).

Publication: in-repo SSOT is USER_JOURNEY.md; logos-docs packet is Step 22 /
[logos-docs#370](https://github.com/logos-co/logos-docs/issues/370).

#### Terminology (user-facing prose)

- **Payer** — vault owner who deposits and opens the stream.
- **Payee** — stream recipient who claims accrued funds.
- LIP and `chainAction` JSON keep wire names **`owner`** (payer on claim/read) and **`provider`**
  (payee on `createStream` / `claim`). A single **Glossary** section in USER_JOURNEY maps prose ↔
  JSON and defines payment-stream-specific terms (`accrued_lo`, `stream_state`, etc.).
- Do not use user/provider as the primary story in USER_JOURNEY.

#### Scenario

Single host, one `logoscore` daemon, one wallet storage with **two public accounts** (payer and payee).

Narrative order (differs from current `module-e2e.sh` close signer until
[raw-todos](../raw-todos/e2e-close-payer-authority.md) is done):

1. Payer creates vault (or reuses an empty vault id), deposits, opens stream to payee.
2. Funds accrue; reader polls `getStreamStatus` until `accrued_lo` is sufficient.
3. **Payer closes** the stream (`closeStream` with `signer` = payer; omit `authority` so the module
   defaults signing to `signer` — implemented in `payment_streams_module_writes.cpp`).
4. Payee claims residual accrued on the closed stream.

**Pre-dry-run check (implementor):** Run one `closeStream` on testnet with payer-only params before
locking USER_JOURNEY copy. `module-e2e.sh` still passes payee as `authority` until
[raw-todos/e2e-close-payer-authority.md](../raw-todos/e2e-close-payer-authority.md); the walkthrough
must not copy that JSON shape.

Pedagogy: after stream creation, include an **out-of-band** step — copy payer account id,
`vault_id`, and `stream_id` into a short “payee notes” block before claim (simulates two-party
coordination). One sentence: in production, payee learns these coordinates outside the app.

Footnotes only (not alternate walkthrough): LIP allows payee claim while stream is still Active;
demo uses close-then-claim to keep settlement simple. Pause, resume, and top-up are out of scope;
see module command catalogue.

#### Prerequisites section (USER_JOURNEY)

State explicitly:

- Public TestNet v0.2 sequencer (`https://testnet.lez.logos.co/` or value from fixture).
- Deployed payment-streams **program id** (SSOT hex in doc, e.g. from
  `fixtures/testnet-module.json` — currently org-deployed guest; users do not redeploy).
- Tooling: `logoscore` and `lgpm` 0.2.0 (same as
  [logos-docs build-and-run](https://docs.logos.co/core/build-modules/build-and-run-a-logos-core-module));
  `lgs init` / `lgs setup` (builds scaffold `wallet` at `~/.cache/logos-scaffold/.../target/release/wallet`
  used for testnet pinata claims — same binary as `scripts/lib/fund_testnet.sh`).
- Modules: `logos_execution_zone` + `payment_streams_module` only (no `delivery_module`).
- Install: `nix build` portable `.lgx` from this repo + `lgpm --modules-dir … install --file …`
  (catalogue install optional future; not required for this step).
- Wallet + payment streams modules: install per [feature-branch-pins.md](../../reference/feature-branch-pins.md)
  (repo `nix build` + `lgpm` portable `.lgx`). Module-only walks exercise generic public tx via
  `send_generic_public_transaction_json`; they do **not** call patched `sign_public_payload`
  (Store eligibility only). Upstream-only wallet builds are a separate
  [raw-todo](../raw-todos/upstream-wallet-and-patch-inventory.md), not a Step 34 prerequisite.
- **One-time machine setup:** `make bootstrap-testnet-module` (details in E2E.md). Walkthrough does
  not replay bootstrap’s on-chain seeding; it starts at runtime + accounts onward.
- **Accounts (default path):** reuse `owner_account_id` / `provider_account_id` from
  `fixtures/testnet-module.json` after bootstrap (matches `module-e2e.sh` testnet). Optional appendix:
  create fresh accounts via `create_account_public` if the reader wants a clean pair (requires full
  AT + pinata funding for both).
- Security: fixture/test keys are for test networks only.

#### Walkthrough sections (command-level deliverable)

Each write step: intent, exact `logoscore call …`, then **sync** (`sync_to_block` to latest height)
and **read** verification (`getVaultStatus` / `getStreamStatus` / payee balance). Keep inclusion
troubleshooting minimal (one line: if reads lag, sync and poll again).

| Block | Content |
| --- | --- |
| Runtime | `logoscore -D -m <modules>`; `load-module` wallet + payment_streams_module; wallet `open` / `create_new` |
| Accounts + AT | Per account: AT registration before any pinata or stream write. If using standalone
  `wallet auth-transfer init`: **close** wallet in logoscore first (`logoscore call logos_execution_zone close`),
  run `LEE_WALLET_HOME_DIR=… wallet auth-transfer init --account-id Public/<b58>`, then
  `logoscore call logos_execution_zone open …` again (same pattern as `PS_AT_LOGOSCORE_WALLET_HANDOFF`
  in [auth_transfer.sh](../../../scripts/lib/auth_transfer.sh)). Alternative: `register_public_account`
  via logoscore only (no standalone wallet) when that path works for the account. |
| Funding | Scaffold `wallet pinata claim --to Public/<base58>` (after `lgs setup`), loop until balance ≥
  target (payer: deposit + gas buffer; payee: gas for claim). AT before pinata. |
| Vault id | Scan with `getVaultStatus` per candidate id under payer; pick empty or new id |
| Stream lifecycle | `initializeVault`, `deposit`, `createStream` (JSON `"provider"` = payee base58),
  accrual poll, `closeStream`, OOB copy-paste, `claim` |
| Clarifications | Short notes: `accrued_lo` / `unaccrued_lo`; `stream_state` (0 Active, 2 Closed) before
  claim; unaccrued returns to vault on close; claim pays accrued (zero accrued → nothing to claim).
  Accrual: with rate 1 and min accrued 1, one included block often suffices; testnet inclusion and
  reads can lag tens of seconds — sync and poll every few seconds. |

**Testnet sizing SSOT:** cite values from `fixtures/testnet-module.json` in USER_JOURNEY (currently
`demo_deposit_amount` 500, `allocation` 80, `stream_rate` 1). Do not use localnet-only examples
(allocation 400) in the testnet walkthrough. `module-e2e.sh` env overrides may differ; fixture
fields are what the doc teaches unless the author documents explicit overrides.

#### `chainAction` catalogue (module SSOT)

Add SSOT to [docs/payment-streams-module/README.md](../../payment-streams-module/README.md):

- **All** `chainAction` operations from
  [module-chain-writes-runbook.md](../../archive/steps/module-chain-writes-runbook.md) (writes, reads,
  pause/resume/top-up included).
- Per op: JSON keys table + one-line semantics; note which ops the USER_JOURNEY walkthrough exercises.
- Archive runbook may point here as current reference.

Link to this catalogue from:

- USER_JOURNEY (full op list note),
- Root [README.md](../../../README.md) (one paragraph + link),
- E2E.md where integrators need API shape.

#### E2E.md

Create [docs/journeys/E2E.md](../../journeys/E2E.md):

- Merge all content from [slides/RUN.md](../../journeys/slides/RUN.md) into E2E.md, then **delete**
  `docs/journeys/slides/RUN.md` (no stub file). No other repo files link to RUN.md today.
- **Doc boundary:**
  - [verification-matrix.md](../../reference/verification-matrix.md) — SSOT for 2×2 Required tiers,
    cold start, maintainer notes, artifact locations. Trim or replace its per-cell **command** block
    with “Recipes: [E2E.md](../../journeys/E2E.md)” once E2E.md lands (avoid duplicating long command lists).
  - **E2E.md** — SSOT for per-cell **run recipes** (prepare/bootstrap one-liners, `e2e.sh` / make
    invocations, expected exit code and key artifact phases).
- Cells in this step: **module × local/testnet** — full recipes (merge RUN.md). **store × local** —
    full recipe from RUN.md / matrix. **store × testnet** — full recipe in E2E.md (Step 32 D3
    closed; default `E2E_CLAIM_OPTIONAL=0`); do not block USER_JOURNEY or module E2E work on
    that cell.

#### Scripts and alignment (non-blocking for doc land)

`module-e2e.sh` still uses payee as `closeStream` `authority` while narrative says payer closes.
Track fix in [raw-todos/e2e-close-payer-authority.md](../raw-todos/e2e-close-payer-authority.md).
After fix, optional one-line note in E2E.md expected phases; USER_JOURNEY already teaches payer close.

#### Stale doc pass (after implementation)

Mostly landed (module README has testnet verify; matrix links E2E.md; Step 22 phase
table is close-then-claim). Remaining: keep USER_JOURNEY env defaults aligned with
`module-e2e.sh` as scripts evolve.

#### Definition of done

- [x] [USER_JOURNEY.md](../../journeys/USER_JOURNEY.md) rewritten as testnet manual walkthrough per
  scenario and blocks above (single Glossary section per Terminology above).
- [x] [E2E.md](../../journeys/E2E.md) exists with RUN.md content merged; `slides/RUN.md` removed;
  matrix links to E2E.md.
- [x] [payment-streams-module/README.md](../../payment-streams-module/README.md) contains SSOT
  `chainAction` catalogue; root README links to it.
- [x] User Journey considered complete; logos-docs packet
  [logos-docs#370](https://github.com/logos-co/logos-docs/issues/370) (Step 22).

Tooling entrypoint: [USER_JOURNEY.md#prerequisites](../../journeys/USER_JOURNEY.md#prerequisites)
(`scripts/user-journey-shell.sh`).

#### Not in scope

- Developer Journey / Store / `delivery_module` / eligibility proofs.
- Step 21 UI.
- LIP rename of `provider` wire field.
- Catalogue publish of `payment_streams_module` to logos-modules.
- Localnet content in USER_JOURNEY (localnet stays in E2E.md only).
