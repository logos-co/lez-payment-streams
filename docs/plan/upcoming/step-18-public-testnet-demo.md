# Step 18 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 18, Public sequencer E2E (local Store and relay)

Prerequisite: Step 17 definition of done satisfied on local LEZ
(`scripts/demo-e2e-local.sh`, [N12](../../reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)).

Branch: `feat/step18-public-testnet` from `master`. All changes are additive and gated by a
`CHAIN` selector (`CHAIN=local` default, `CHAIN=testnet` for this step). The local-LEZ path
(`make verify-step17`) must keep passing unchanged on `master` throughout the work; see the
non-regression guard at the end of this packet.

Architectural context:

Step 17 uses a disposable local LEZ sequencer (`127.0.0.1:3040`) and two `logoscore`
processes on one machine. Relay, GossipSub, and Store traffic stay on **localhost** (disjoint
`portsShift`, user `staticnode` dial to provider multiaddr from
[`E2E_PROVIDER_AD`](../../step17-e2e-local.md#provider-service-advertisement-off-band-mimic)).

Step 18 keeps that **same dual-host P2P layout**. Only **LEZ chain access** moves to the org
public testnet sequencer (testnet v0.2 target). The public endpoint is documented in
`logos-docs/docs/lez/get-started/quickstart-for-the-logos-execution-zone-wallet.md` as
`https://testnet.lez.logos.co/` (explorer `https://explorer.testnet.lez.logos.co/`). That doc
asserts the endpoint exists but does not document the `wallet_config.json` schema, the
`sequencer_addr` field, the testnet LEZ revision, whether `wallet pinata claim` works against
it, or how to deploy a LEE guest program there. Those gaps are closed by the reconnaissance
phase below, not by the docs.

Do not apply local reset-first policy from
[`demo-localnet-recovery.md`](../../demo-localnet-recovery.md). See testnet persistence in
[`step12-user-eligibility.md`](../../step12-user-eligibility.md) (Persistence across runs).

Hosting a Store provider on the public Logos mesh (infra-operated node, dialable from the
internet) is **Step 23** (optional). Step 18 does not require it.

#### Lifecycle separation (core design decision)

The local-LEZ path conflates three lifecycles because the ledger is disposable. The testnet
path must decouple them:

1. Build the guest ELF (reproducible; produces a stable `program_id_hex` for a given source
   tree via `Program::new(bytecode).id()`, `examples/src/bin/seed_localnet_fixture.rs:205-216`).
   No chain I/O.
2. Deploy the program to a chain (once per chain; testnet once ever, local every fresh reset).
3. Run the demo against an already-deployed, already-funded program (every run; needs funded
   vault + stream, not a redeploy).

`program_id_hex` is deterministic from the ELF bytes, so the deployed address is reusable
across runs and across operators as long as the guest source is unchanged. The testnet path
runs (1) on every `make build`, (2) once, (3) every demo run. The local path keeps coupling
all three because the ledger is wiped on every fresh seed.

The existing `seed_localnet_fixture.rs` subcommand structure already supports this split:
`write-manifest` (no chain I/O), `prefund-onchain` (vault + deposit), `create-stream-onchain`
(stream on funded vault). The testnet path calls them in one-time mode against a different
`--sequencer-url` and skips the localnet-start / snapshot-restore preamble.

Open question to resolve in reconnaissance: whether `wallet deploy-program` against the public
testnet succeeds and the deployed program is immediately callable by a second wallet on the
same chain (the provider host). If yes, program deploy is network-one-time. If each operator
must deploy their own program instance, the fixture manifest gains a per-operator
`program_id_hex` and deploy becomes per-operator-one-time. The docs do not answer this.

#### Reconnaissance results (2026-06-23)

Phase 1 reconnaissance is complete. Results:

- Sequencer liveness: `https://testnet.lez.logos.co/` is live; `getLastBlockId` returns
  block `66256` (JSON-RPC `result` int, same shape `run_local_e2e.py:142-147` parses).
- Compatible LEZ pin: `v0.2.0-rc3` = `cf3639d8252040d13b3d4e933feb19b42c76e14a`. Confirmed
  via `wallet check-health` (exit 0) and tag probes. Piñata funding confirmed: balance `150`
  after `wallet pinata claim`.
- Current pin `62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60` (PR 510 merge) is **not** compatible
  with the public testnet: `check-health` panics with "Local ID for authenticated transfer
  program is different from remote" (builtin `AUTHENTICATED_TRANSFER_ID` mismatch,
  `lee/state_machine/src/program.rs:114-119`). `62d9ba10` is a descendant of `cf3639d8`
  (merge-base is `cf3639d8` itself), so the divergence is forward from rc3.
- Guest ImageID (for `fixtures/testnet.json`, independent of LEZ builtin pin):
  `f9f4147cb438738a64487ea766afe1d1a6fa5542b402cf3f567fbd813a84cc0b`.

FFI surface gap (the hard constraint on the pin choice):

The testnet-compatible pin `cf3639d8` (`v0.2.0-rc3`) predates the wallet-ffi refactor that
moved `wallet-ffi/` under `lez/wallet-ffi/` and added the generic-transaction / program-deploy
FFI surface our integration depends on. Concretely, `cf3639d8`'s `wallet-ffi/src/` is missing
the following symbols that `62d9ba10`'s `lez/wallet-ffi/src/` provides and our patched wallet
module + `payment_streams_module` call:

- `wallet_ffi_send_generic_public_transaction` (chain writes: `createStream`, `topUpStream`,
  `claim` via `chainAction`)
- `wallet_ffi_send_generic_private_transaction`
- `wallet_ffi_program_deployment` (guest program deploy)
- `wallet_ffi_transfer_elf`, `wallet_ffi_token_elf`, `wallet_ffi_amm_elf`, `wallet_ffi_ata_elf`
  (builtin ELF accessors)
- `wallet_ffi_import_private_account`, `wallet_ffi_import_public_account`,
  `wallet_ffi_resolve_private_account`, `wallet_ffi_resolve_public_account`,
  `wallet_ffi_create_private_accounts_key`, `wallet_ffi_free_ffi_program`,
  `wallet_ffi_free_instruction_words`, `wallet_ffi_free_transaction_result`,
  `wallet_ffi_free_account_identity`, `wallet_ffi_serialization_helper`

Source: `git grep -hoE "wallet_ffi_[a-z_0-9]+" <pin> -- <wallet-ffi dir>/src/` at each pin.
Our `feature-branch-pins.md` documents `send_generic_public_transaction_json` and
`sign_public_payload` as the Step 11b chain-write path; `wallet_ffi_program_deployment` is the
Step 11d deploy path. Both are absent at `cf3639d8`.

Implication: we cannot simply bump `scaffold.toml` to `cf3639d8` — the local-LEZ path
(`make verify-step17`) and the payment-streams module would lose their chain-write and
program-deploy FFI. The testnet pin must be isolated behind `CHAIN=testnet` with a separate
wallet binary path, while the local path stays on `62d9ba10`.

The testnet wallet binary at `cf3639d8` can still drive the recon tests (account creation,
Piñata claim, `auth-transfer init`, reads) because those use the older transfer-specific FFI
that exists at both pins. But the full Step 18 demo flow (which needs `chainAction`
`createStream` / `topUpStream` / `claim` against testnet) requires generic public transaction
submission, which is the FFI surface missing at `cf3639d8`.

#### FFI gap resolution — decision and rationale

Step 18 resolves the FFI gap with a **temporary Rust submit helper**, not a LEZ fork/backport.
The rationale, verified by reading the rc3 source:

- The rc3 wallet already constructs and submits generic `PublicTransaction`s. The AMM facade
  (`wallet/src/program_facades/amm.rs:115-122` at `cf3639d8`) and the Piñata facade
  (`wallet/src/pinata_interactions.rs:26-28`) both run the same four-step pattern:
  `Message::try_new` → `WitnessSet::for_message` → `PublicTransaction::new` →
  `sequencer_client.send_transaction(NSSATransaction::Public(tx))`. These primitives all exist
  at `cf3639d8`; only the single generic entry point is missing.
- PR #491's `wallet_ffi_send_generic_public_transaction` body is mechanically that same four
  steps plus FFI marshalling (it calls `wallet.send_pub_tx(...)`, which is a thin wrapper over
  the same pattern). So the helper reproduces an existing in-repo pattern at the same commit,
  not a reverse-engineered protocol.
- Program deployment is even cheaper: rc3 already ships `wallet deploy-program <elf>` as a CLI
  subcommand (`wallet/src/cli/mod.rs` `Command::DeployProgram`), which wraps
  `ProgramDeploymentTransaction::new` + `send_transaction`. No helper needed for deploy; the
  testnet path shells out to the rc3 wallet CLI.

Rejected alternatives:

- Backport / fork LEZ: cherry-pick #491 and #510 onto `cf3639d8`. Rejected because #491 was
  authored after the `wallet-ffi → lez/wallet-ffi` / `nssa → lee` restructure (commit
  `50be7458`), so every diff targets paths that do not exist at rc3. This means maintaining a
  fork branch of an entire upstream repo for the duration of the testnet gap, with merge-conflict
  risk on every upstream sync. Retirement cost is a pin re-point plus re-verification, not a
  pure deletion.
- Bisect for an intermediate revision: find a commit between `cf3639d8` and `62d9ba10` that is
  both testnet-compatible (builtin IDs match) and has the generic FFI. Rejected as a
  time-limited probe that may yield nothing, and any revision found would still be a non-standard
  pin with the same retirement cost as the fork. Worth a quick check before building the helper,
  but not the primary path.
- Degraded demo (bootstrap on testnet, claim on local): rejected; it defeats Step 18's purpose.

The helper is the chosen path because its retirement cost is minimal: when the testnet rebases
past #491/#510, delete the helper, delete the `CHAIN=testnet` submit branch, and restore the FFI
path. No changes to `payment_streams_module`, `payment_streams_ffi`, `instruction_abi.rs`, or any
core code. The helper is reached through a `CHAIN` selector; flipping it back to `ffi` is a
one-line or config change.

Design discipline: the helper's input/output contract must mirror what
`send_generic_public_transaction_json` already expects, so the `ffi` and `helper` branches are
interchangeable from `payment_streams_module`'s perspective. The helper reads the same JSON
shape the module already produces.

This FFI gap is the critical path for Step 18. The helper is a temporary bridge, not a fork.

#### What stays local vs what moves to testnet

| Layer | Step 17 | Step 18 |
| --- | --- | --- |
| LEZ sequencer | `127.0.0.1:3040` (`lgs localnet`) | Public testnet URL (documented) |
| `logoscore` user + provider | Two local daemons | Same: two local daemons |
| Delivery P2P / Store | Local TCP ports, provider archives SQLite | Same topology; multiaddrs remain loopback in the default script |
| Eligibility hooks | Local provider verifier + user provider registration | Same module behavior; chain reads/writes hit testnet |
| Program on chain | Deploy per local seed (every fresh reset) | Deploy once on testnet; stable `program_id_hex` in fixture |
| Vault/stream bootstrap | `seed-onchain` / `prefund-onchain` + `create-stream-onchain` per fresh seed | One-time `bootstrap-testnet`; reused across runs |
| Chain reset | `demo-localnet-fresh.sh` / `FULL_RESET=1` wipes ledger | Never reset testnet chain; only reset local `PERSIST_*` |

#### Configuration that must match (sync checklist)

All of the following must refer to the **same** public network and deployed program.
Mismatches produce deploy failures, `STREAM_NOT_ACTIVE`, or verify rejections with no Store
symptoms.

| Setting | Where | Purpose |
| --- | --- | --- |
| `sequencer_addr` | Each host `wallet_config.json` used at `logos_execution_zone open` | Wallet RPC: reads, writes, sync |
| `sequencer_url` | `fixtures/testnet.json` (`FIXTURE_MANIFEST`) | Operator docs, scripts, verify parity with wallet |
| LEZ revision | [`feature-branch-pins.md`](../../feature-branch-pins.md): local pin `62d9ba10` (default), testnet pin `cf3639d8` in a testnet-only flake input for the helper | Client must speak the same LEZ as the public sequencer |
| `program_id_hex` | `fixtures/testnet.json` | Module PDA derivation and `chainAction` program binding ([N10](../../reference/decisions-and-notes.md#n10-step-11b-module-writes-decisions)) |
| Owner / provider account ids, vault/stream ids, derived PDAs | `fixtures/testnet.json` | Prepare/verify and on-chain stream state |
| `clock_10_account_id` | `fixtures/testnet.json` | Testnet clock account (do not copy local-only id without validation) |
| Guest program for submits | `PAYMENT_STREAMS_GUEST_BIN` (local FFI path) or helper's `--program-elf` (testnet path) | Writes (`createStream`, `claim`) against **testnet** deployed ELF |
| Provider libp2p identity | `E2E_PROVIDER_AD` (written by script) | User `registerProviderMapping` + `storeQuery` target — still **local** provider peer in Step 18 |
| Provider LEZ payee | `provider_account_id` in manifest | Must match bytes in proofs and on-chain `StreamConfig.provider` ([N5](../../reference/decisions-and-notes.md#n5-provider-identity-mapping)) |
| Module install | `MODULES_USER`, `MODULES_PROVIDER` | Same forked `.lgx` set as Step 17 ([feature-branch-pins.md](../../feature-branch-pins.md)) |
| Off-chain eligibility state | `--persistence-path` per host (`PERSIST_USER`, `PERSIST_PROVIDER`) | Session keys, mappings, provider acceptances ([N4](../../reference/decisions-and-notes.md#n4-persistence-policy)); separate from chain persistence |

Wallet and manifest: [N10 fixture and config](../../reference/decisions-and-notes.md#n10-step-11b-module-writes-decisions).
Delivery `createNode` defaults: [step17-e2e-local.md](../../step17-e2e-local.md#delivery-createnode-defaults) (local ports unchanged).

#### Implementation order

Work proceeds in phases on `feat/step18-public-testnet`. Each phase ends with the local-LEZ
regression gate still green (see non-regression guard below). No code in the run path ships
until reconnaissance confirms the testnet is usable.

##### Phase 0 — Branch and convention

1. Create `feat/step18-public-testnet` from `master`.
2. Introduce the `CHAIN` env convention: `CHAIN=local` (default, current behavior) vs
   `CHAIN=testnet`. Initially documentation only; scripts read it but the `local` path is
   byte-for-byte unchanged when `CHAIN` is unset or `local`.

##### Phase 1 — Reconnaissance (DONE 2026-06-23)

3. Sequencer liveness confirmed: `https://testnet.lez.logos.co/` responds to `getLastBlockId`
   (block `66256`). No code change needed in `run_local_e2e.py` — it already parses the
   `result` int shape.
4. LEZ revision: compatible pin is `cf3639d8` (`v0.2.0-rc3`); our current pin `62d9ba10` is
   incompatible (builtin `AUTHENTICATED_TRANSFER_ID` mismatch). See "Reconnaissance results"
   above for the FFI surface gap that this creates.
5. Piñata funding confirmed on testnet: `wallet pinata claim` succeeds, balance `150`.
6. Program deploy feasibility: deferred pending the FFI gap resolution below. `wallet
   deploy-program` exists as a CLI at `cf3639d8` and should work for the one-time deploy,
   but the per-run `chainAction` writes (`createStream`, `topUpStream`, `claim`) need the
   generic-transaction FFI that is absent at `cf3639d8`.

##### Phase 1b — Quick bisect probe (time-boxed, before building the helper)

7. Before committing to the helper, spend at most half a day checking whether any revision
   between `cf3639d8` and `62d9ba10` is both testnet-compatible (builtin
   `AUTHENTICATED_TRANSFER_ID` matches the testnet sequencer) and already carries the
   generic-transaction FFI from PR #491. If such a revision exists, no helper is needed: pin
   it behind `CHAIN=testnet` and skip Phases 1c/1d.
8. Probe method: `git log --oneline cf3639d8..62d9ba10 -- lez/wallet-ffi/src/generic_transaction.rs`
   finds the first commit that introduces `wallet_ffi_send_generic_public_transaction`
   (PR #491 merge `a8c81f5`). For a small set of commits in
   `[cf3639d8 .. a8c81f5]`, run `wallet check-health` against the testnet. If any passes, that
   revision is a candidate. If none pass (the builtin ID diverged before #491 landed), fall
   through to Phase 1c.
9. Record the probe outcome in `feature-branch-pins.md`. If negative, proceed to Phase 1c.

##### Phase 1c — Build the temporary submit helper

10. Add a small Rust binary crate `lez-testnet-submit` under `tools/` (name tentative). It is
    a standalone CLI, not a `wallet_ffi` extension. It links against the rc3 LEZ crates
    (`nssa`, `wallet`) pinned at `cf3639d8` via a testnet-only Nix flake input, completely
    separate from the local-path `62d9ba10` pin.
11. The helper must implement exactly the operations `payment_streams_module` needs for
    `chainAction` writes, with an input/output contract that mirrors
    `send_generic_public_transaction_json`:
    - `submit-public-tx`: read a JSON arg `{ program_id_hex, account_identities, instruction
      words (or raw instruction bytes), }`, open the wallet at the configured path, resolve
      account nonces via `wallet_core.get_accounts_nonces`, fetch signing keys from wallet
      storage, then run the four-step pattern the rc3 AMM facade uses:
      `Message::try_new(program.id(), account_ids, nonces, instruction)` →
      `WitnessSet::for_message(&message, &private_keys)` →
      `PublicTransaction::new(message, witness_set)` →
      `sequencer_client.send_transaction(NSSATransaction::Public(tx))`. Print the tx hash as
      JSON `{ success, tx_hash }`, matching the FFI result shape.
    - No `deploy-program` subcommand in the helper: rc3's `wallet deploy-program <elf>` CLI
      already covers one-time deploy. The helper is only for the per-run `chainAction` writes
      that have no rc3 CLI equivalent.
12. The helper does **not** touch `wallet_ffi`, `payment_streams_ffi`, or the C bridge. It is
    invoked via subprocess (or a thin LogosAPI method on a testnet-only module build), not via
    FFI dlopen. This keeps the local FFI path untouched.
13. Unit-test the helper against a local rc3 sequencer first (start rc3 locally, submit a
    trivial public tx, assert the hash is non-empty). This isolates helper correctness from
    testnet liveness.
14. Gate the helper behind `CHAIN=testnet`. When `CHAIN` is unset or `local`, the helper is
    never compiled into the default `nix build` and never invoked. The local path continues
    to call `wallet_ffi_send_generic_public_transaction` through the existing C bridge.

##### Phase 1d — Selector in `payment_streams_module`

15. Add a `CHAIN` (or `WALLET_BACKEND`) selector to the module's chain-write dispatch. When
    `CHAIN=testnet`, chain writes shell out to `lez-testnet-submit submit-public-tx` instead of
    calling `send_generic_public_transaction_json` over the FFI bridge. The JSON contract is
    identical, so the instruction-serialization code (`instruction_abi.rs`,
    `instruction_wire.rs`) is shared unchanged.
16. The selector is a single dispatch point, not a scatter of conditionals. Concretely: one
    function `submit_chain_action(action_json)` that branches on `CHAIN`. Everything upstream
    of it (instruction encoding, PDA derivation, account-id resolution) is backend-agnostic and
    shared.
17. The `CHAIN=local` branch is byte-for-byte the current FFI call. Verify this with a diff
    review: the only new code on the local path is the selector reading the env var.

##### Phase 2 — Testnet program deploy (one-time, CLI-only)

18. Add a Makefile target `deploy-testnet` that:
    - Reads `WALLET_CONFIG` pointing at a testnet wallet config
      (`sequencer_addr = https://testnet.lez.logos.co/`).
    - Runs `wallet deploy-program $(PROGRAM_BIN)` (rc3 CLI, not the helper) against the
      testnet sequencer.
    - Records the resulting `program_id_hex` (should equal `make program-id` output).
    - Is idempotent: re-running against an already-deployed program on testnet should either
      no-op or fail with a recognizable "already deployed" error. Verify the wallet/sequencer
      behavior and document it.
19. Do not wire this target into `make verify-step18`. It is a one-time bootstrap step, not a
    per-run step.

##### Phase 3 — Testnet fixture bootstrap (one-time, or rare)

20. Add a Makefile target `bootstrap-testnet` that runs `initialize_vault`, `deposit`, and
    `create_stream` against testnet. Vault/stream bootstrap uses the helper's
    `submit-public-tx` (these are `chainAction` writes: `initializeVault`, `deposit`,
    `createStream`) since rc3 has no CLI for them. Writes `fixtures/testnet.json` with the
    resulting ids and `program_id_hex` from Phase 2.
21. This is the second one-time step. Kept separate from Phase 2 because funding (Piñata or
    faucet) may need retries and is independent of program deployment.
22. Commit `fixtures/testnet.json.example` (no secrets; may include stable `program_id_hex`
    after org-wide deploy). Operators copy to gitignored `fixtures/testnet.json` or set
    `FIXTURE_MANIFEST`.

##### Phase 4 — Testnet wallet template

23. Create a testnet wallet template under `.scaffold/e2e/testnet-wallet/wallet_config.json`
    with `sequencer_addr` set to the testnet URL. Document that operators copy this to a
    gitignored path or set `WALLET_CONFIG`.
24. The existing `verify-step10b-dod.sh` `127.0.0.1:3040` assertion is the local-LEZ gate and
    stays unchanged. It is not run for `CHAIN=testnet`.

##### Phase 5 — Run path changes (gated, additive)

25. Teach `scripts/demo-e2e-local.sh` (or a new `scripts/demo-e2e-testnet.sh`) to read `CHAIN`
    and pick `FIXTURE_MANIFEST` / `WALLET_CONFIG` accordingly. When `CHAIN=testnet`:
    - Skip `lgs localnet start`, `demo-localnet-fresh.sh`, `demo-localnet-prepare.sh`, and
      snapshot restore.
    - Skip `make deploy` / `make setup` (program is already deployed, accounts already
      funded).
    - Still run `nix build` + `lgpm install` for the three modules, and still start two local
      `logoscore` daemons for user and provider.
    - The `sequencer_url` in the manifest drives wallet sync; no code change needed in
      `run_local_e2e.py` (it already reads `manifest.get("sequencer_url", ...)`).
26. The late-stream-create / top-up path in `run_local_e2e.py` (`ensure_fresh_demo_stream`,
    `user_prepare_proof`) stays. It is a vault lifecycle op, not a program deploy, and is
    needed when the testnet stream is depleted. Document that testnet stream depletion is
    real (accrual runs between prepare and verify) and top-up is the expected remedy.
27. Add `make verify-step18` that runs the testnet path. It should fail fast (not hang) if
    the testnet sequencer is unreachable. It should not start a local sequencer.

##### Phase 6 — Runbook and docs

28. Write `docs/step18-public-sequencer-e2e.md` covering:
    - Prerequisites (internet egress, testnet wallet, funded account).
    - One-time bootstrap: `make deploy-testnet`, `make bootstrap-testnet`.
    - Repeatable run: `make verify-step18`.
    - The temporary helper: what it is, why it exists, and the retirement trigger (testnet
      rebases past PR #491 / #510).
    - Persistence rules (do not wipe chain state; only reset local `PERSIST_*`).
    - Failure triage for testnet-specific cases (unreachable sequencer, stale program id,
      faucet rate limits, stream depletion).
29. Update [`feature-branch-pins.md`](../../feature-branch-pins.md) with:
    - The testnet LEZ revision (`cf3639d8`).
    - The deployed `program_id_hex`.
    - A "retirement" section recording that the helper and `CHAIN=testnet` submit branch are
      deleted once the testnet runs a revision at or past `62d9ba10`, and the local FFI path
      becomes the sole backend.

##### Phase 7 — Retirement (deferred, triggered by testnet catch-up)

30. This phase is not executed on the feature branch. It is the documented retirement
    procedure, triggered when the public testnet rebases to a LEZ revision at or past
    `62d9ba10` (i.e. one that includes PR #491 and #510):
    - Delete `tools/lez-testnet-submit` (or its successor location).
    - Remove the `CHAIN=testnet` branch from `submit_chain_action`; the FFI call becomes the
      sole path.
    - Drop the testnet-only Nix flake input for the rc3 LEZ pin.
    - Re-point the testnet wallet pin to the new testnet revision in `feature-branch-pins.md`.
    - Re-run `make verify-step18` to confirm the FFI path works against the updated testnet.
31. No changes to `payment_streams_module` instruction encoding, `payment_streams_ffi`, the C
    bridge, `instruction_abi.rs`, or any core code are required at retirement. This is the
    key property the helper approach was chosen for.

#### Deliver

- Temporary submit helper `tools/lez-testnet-submit` (name tentative): a standalone Rust CLI
  that reproduces the rc3 AMM-facade public-tx pattern (`Message::try_new` →
  `WitnessSet::for_message` → `PublicTransaction::new` → `send_transaction`) for the
  `chainAction` writes rc3 has no CLI for. Links rc3 LEZ crates via a testnet-only Nix flake
  input. Never compiled or invoked when `CHAIN` is unset or `local`.
- `CHAIN` selector in `payment_streams_module` at a single `submit_chain_action` dispatch
  point: `CHAIN=testnet` shells out to the helper; `CHAIN=local` (default) calls
  `send_generic_public_transaction_json` over the FFI bridge unchanged.
- Committed `fixtures/testnet.json.example` (no secrets; may include stable `program_id_hex`
  after org-wide deploy). Operators copy to gitignored `fixtures/testnet.json` or set
  `FIXTURE_MANIFEST`.
- Testnet bootstrap runbook (`docs/step18-public-sequencer-e2e.md`): one-time program deploy
  (rc3 `wallet deploy-program`), one-time vault/stream bootstrap (via helper), repeatable demo
  run; document one-time vs repeatable steps explicitly; document the helper's temporary
  nature and retirement trigger.
- Testnet wallet template(s): `sequencer_addr` aligned with manifest `sequencer_url` (per-host
  copies under `.scaffold/e2e/` or documented paths).
- `scripts/demo-e2e-testnet.sh` or `CHAIN=testnet` in the Step 17 orchestrator: **no**
  `lgs localnet start` / `demo-localnet-fresh.sh` on the chain path; still starts two local
  `logoscore` hosts; CI default remains Step 17 local LEZ.
- Makefile targets `deploy-testnet`, `bootstrap-testnet`, `verify-step18` (names tentative).

#### Definition of done

- Documented public sequencer URL and LEZ revision aligned with wallet pins
  ([`feature-branch-pins.md`](../../feature-branch-pins.md)): local pin `62d9ba10` unchanged,
  testnet pin `cf3639d8` lives in a testnet-only flake input for the helper.
- `lez_payment_streams` deployed on that network via rc3 `wallet deploy-program`; demo uses
  committed or operator manifest with matching `program_id_hex` and bootstrapped vault/stream
  state.
- The temporary submit helper (`tools/lez-testnet-submit`) drives `chainAction` writes
  (`createStream`, `topUpStream`, `claim`) against testnet; its JSON contract matches
  `send_generic_public_transaction_json` so the `CHAIN` selector is the only module-side
  difference between backends.
- Dual-host paid Store success and inbound eligibility failure match Step 17 outcomes (same wire
  and modules; chain fixture on testnet).
- Operator can reproduce without wiping **chain** state; persistence rules documented (manifest +
  vault reuse; when to reset local `PERSIST_*` only).
- One-time deploy (`make deploy-testnet`) and one-time bootstrap (`make bootstrap-testnet`) are
  documented as distinct from the repeatable run (`make verify-step18`).
- Retirement procedure (Phase 7) is documented and requires no changes to core code; it is
  triggered when the testnet rebases past PR #491 / #510.

#### Non-regression guard

Before merging `feat/step18-public-testnet` to `master`:

- Run `make verify-step17` on `master` and on the branch. Both must pass with `CHAIN` unset
  and with `CHAIN=local`. Confirm no local-path script behavior changed when `CHAIN` is unset
  or `local`.
- Run `make verify-step18` on the branch against testnet. Confirm it does not start a local
  sequencer and does not call `make deploy` / `make setup` / `demo-localnet-fresh.sh`.
- Confirm `verify-step10a-dod.sh` and `verify-step10b-dod.sh` still assert `127.0.0.1:3040`
  and are not parameterized for testnet (they are the local-LEZ gate).
- Confirm no default pin in `scaffold.toml` or wallet flakes changed without an explicit
  testnet-pin decision recorded in [`feature-branch-pins.md`](../../feature-branch-pins.md).
  The testnet pin (`cf3639d8`) is `CHAIN=testnet`-only and lives in a testnet-only flake input
  for the helper; the local pin (`62d9ba10`) is unchanged.
- Confirm the helper is not compiled into the default `nix build` (it is gated behind
  `CHAIN=testnet` or a separate flake output). The local path must not gain a new dependency
  on the rc3 LEZ crates.
- Confirm the `submit_chain_action` selector's `CHAIN=local` branch is byte-for-byte the
  existing FFI call. Diff-review the dispatch point: the only local-path change is reading the
  env var.
- Confirm the local wallet binary still exposes `wallet_ffi_send_generic_public_transaction`,
  `wallet_ffi_program_deployment`, and `sign_public_payload` (used by `payment_streams_module`
  chain writes and Step 11d deploy). The helper must not alter the local FFI surface.

Not in scope: public internet Store provider (Step 23); replacing Step 17 local CI gate;
automatic testnet faucet unless the network ships a supported funding API; Basecamp UI
(Step 21); demo coordinator module (Step 25).

Follow-on: Step 19 (LIP on-chain), Step 20 (developer journey on testnet v0.2 uses this step);
optional Step 23 (hosted provider).
