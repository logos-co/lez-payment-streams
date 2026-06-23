# Step 18 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

#### Integration status (2026-06)

Paused on `master`: public testnet Part B is blocked on guest program deploy size (~576576 B
ELF vs ~511800 B public testnet tx cap). Org priority is the fully local demo (Step 17,
`make verify-step17`). Implementation WIP remains on `feat/step18-public-testnet`; do not
block Step 20 or Step 25 on Step 18 completion. Resume this packet when deploy succeeds or
testnet policy changes.

### Step 18, Public sequencer E2E (local Store and relay)

Prerequisite: Step 17 definition of done satisfied on local LEZ
(`scripts/demo-e2e-local.sh`, [N12](../../reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)).

Branch: `feat/step18-public-testnet` from `master`. All changes are additive and gated by a
`CHAIN` selector (`CHAIN=local` default, `CHAIN=testnet` for this step). The local-LEZ path
(`make verify-step17`) must keep passing unchanged on `master` throughout the work; see the
non-regression guard at the end of this packet.

Architectural context:

Step 17 uses a disposable local LEZ sequencer (`127.0.0.1:3040`) and two `logoscore`
processes on one machine. Relay, GossipSub, and Store traffic stay on localhost (disjoint
`portsShift`, user `staticnode` dial to provider multiaddr from
[`E2E_PROVIDER_AD`](../../step17-e2e-local.md#provider-service-advertisement-off-band-mimic)).

Step 18 keeps that same dual-host P2P layout. Only LEZ chain access moves to the org
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
internet) is Step 23 (optional). Step 18 does not require it.

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

On `CHAIN=local`, the existing `seed_localnet_fixture.rs` subcommands implement (2)+(3) against
`127.0.0.1:3040` (`write-manifest`, `prefund-onchain`, `create-stream-onchain`), coupled with
localnet start / snapshot restore as today.

On `CHAIN=testnet`, do not run `seed_localnet_fixture` against the public sequencer: that
binary links pin `62d9ba10` / `lee` and is the local fixture tool only. Testnet (2) is rc3
`wallet deploy-program`; testnet (3) is `make bootstrap-testnet` (helper submits + manifest write)
with no localnet preamble. The lifecycle stages match local; the implementation differs.

Program deploy on testnet is network-one-time: `wallet deploy-program` against the public
testnet succeeds and the deployed program is immediately callable by a second wallet on the
same chain (confirmed during reconnaissance). The `program_id_hex` is therefore shared across
operators, not per-operator.

#### Reconnaissance results (2026-06-23)

- Sequencer liveness: `https://testnet.lez.logos.co/` is live; `getLastBlockId` returns
  block `66256` (JSON-RPC `result` int, same shape `run_local_e2e.py:142-147` parses).
- Compatible LEZ pin: `v0.2.0-rc3` = `cf3639d8252040d13b3d4e933feb19b42c76e14a`. Confirmed
  via `wallet check-health` (exit 0) and tag probes. Piñata funding confirmed: balance `150`
  after `wallet pinata claim`.
- Current pin `62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60` (PR 510 merge) is not compatible
  with the public testnet: `check-health` panics with "Local ID for authenticated transfer
  program is different from remote" (builtin `AUTHENTICATED_TRANSFER_ID` mismatch,
  `lee/state_machine/src/program.rs:114-119`). `62d9ba10` is a descendant of `cf3639d8`
  (merge-base is `cf3639d8` itself), so the divergence is forward from rc3.
- Guest ImageID (for `fixtures/testnet.json`, independent of LEZ builtin pin):
  `f9f4147cb438738a64487ea766afe1d1a6fa5542b402cf3f567fbd813a84cc0b`.

Dual-pin read path (510 `logos_execution_zone` + testnet `sequencer_addr`, without
`check-health`): not re-verified here while the public RPC was down (502). When testnet is
reachable again, run `./scripts/verify-step18-testnet-read-smoke.sh` before relying on
`verify-step18`; exit 0 skip means unreachable, a full PASS closes this gate. See dual-pin
section below.

#### Builtin program IDs and dual-pin semantics (2026-06-23)

Step 18 keeps the Step 17 module stack on pin `62d9ba10` for local E2E and adds testnet
chain access without retargeting `scaffold.toml` or the default `logos_execution_zone` `.lgx`.
That is not a contradiction if we separate what “LEZ compatibility” means for builtins vs
for the payment-streams guest.

On LEZ, system programs (authenticated transfer, token, AMM, privacy circuit, …) are identified
by a ProgramId: the RISC Zero ImageID of the guest ELF baked into the sequencer and wallet at
build time (`Program::new(bytecode).id()` / `artifacts/program_methods/*.bin`). The public
testnet sequencer at rc3 exposes those ids via JSON-RPC `getProgramIds`. The wallet CLI
`check-health` compares remote ids to the ids compiled into the local wallet; the first
failure on pin `62d9ba10` against testnet is authenticated transfer
(`lez/wallet/src/cli/mod.rs` `Command::CheckHealth`), then token, privacy circuit, and AMM the
same way. That is the recon panic text about “Local ID for authenticated transfer program is
different from remote” — not a user account address and not the payment-streams guest id.

ImageIDs derived from `artifacts/program_methods/*.bin` at each pin (same method as
`make program-id`):

| Builtin | rc3 `cf3639d8` (testnet) | `62d9ba10` (local pin) |
| --- | --- | --- |
| `authenticated_transfer` | `a96e088942d7fc09afc7b1db5221558c67f772ac8130d04df1c086dc07ab8b7b` | `d9a19237236822b1f8100576ebd19a19f74178f99e284c983a4ac44acbd5b472` |
| `token` | `6d1ec77d426db847e2a37eb964b78d7870b89f17fc7f2537c0e50046bd8a8150` | `c4584a559312f876bbde4248b1daf95f6fc895a42171734d3ffd32940c0adf24` |
| `amm` | `a2cb551b201f93227167cdd38a0c081c2b771cd4fdc95ed950167132b2e39fbe` | `5d75823a711b071a6da5685c84300a4d4e2fcbada25b95889ee11d88c84b6791` |

The deployed payment-streams guest on testnet uses its own ImageID (`f9f4147c…` above). Fixture
`program_id_hex` must match that guest, not the authenticated-transfer builtin.

What still uses pin `62d9ba10` on the testnet demo path:

- `logos_execution_zone` loaded by both local `logoscore` hosts (unchanged `.lgx` as Step 17).
- `open`, `sign_public_payload`, and chain reads used by `payment_streams_module`
  (`get_account_public` → sequencer `get_account`; `sync_to_block` scans private txs only and
  does not run `check-health`). These paths do not assert builtin ProgramId parity at open.
- Do not treat `wallet check-health` as a gate for `CHAIN=testnet`; expect it to fail while
  local pin and testnet sequencer differ. Local LEZ gates (`verify-step10b-dod.sh`, etc.) keep
  using `127.0.0.1:3040` with pin `62d9ba10`.

What must use rc3 (`cf3639d8`) on the testnet path:

- One-time `wallet deploy-program` (rc3 CLI binary from the testnet flake input).
- Per-run `chainAction` public submits (`createStream`, `topUpStream`, `claim`, bootstrap
  vault/stream ops): rc3 lacks the generic-transaction FFI (below), and even with 510 FFI the
  module attaches 510 `authenticated_transfer_elf` bytes in `program_dependencies_hex` when
  `includeTransferDep` is set (`payment_streams_module_writes.cpp` →
  `logos_execution_zone.authenticated_transfer_elf`). Testnet execution expects rc3 builtin
  ELFs/ids. Hence the rc3-linked submit helper for writes only.

Summary: testnet `sequencer_addr` in `wallet_config.json` points the 510 wallet module at the
public sequencer for RPC reads and signing; chain writes that embed builtin program bytecode go
through rc3 tooling. When the public testnet upgrades to a revision that matches pin `62d9ba10`,
`check-health` with the local wallet against testnet should pass and Phase 9 can drop the helper.

#### FFI surface gap and resolution

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

We cannot simply bump `scaffold.toml` to `cf3639d8` — the local-LEZ path
(`make verify-step17`) and the payment-streams module would lose their chain-write and
program-deploy FFI. The testnet pin `cf3639d8` is used only for the submit helper and the
rc3 `wallet` CLI (`deploy-testnet`); the in-process `logos_execution_zone` module stays on
`62d9ba10` for both `CHAIN=local` and `CHAIN=testnet` (see dual-pin section above).

Step 18 resolves this gap with a temporary Rust submit helper, not a LEZ fork/backport.
The rc3 wallet already constructs and submits generic `PublicTransaction`s. The AMM facade
(`wallet/src/program_facades/amm.rs:115-122` at `cf3639d8`) and the Piñata facade
(`wallet/src/pinata_interactions.rs:26-28`) both run the same four-step pattern:
`Message::try_new` → `WitnessSet::for_message` → `PublicTransaction::new` →
`sequencer_client.send_transaction(NSSATransaction::Public(tx))`. These primitives all exist
at `cf3639d8`; only the single generic entry point is missing. PR #491's
`wallet_ffi_send_generic_public_transaction` body is mechanically that same four steps plus FFI
marshalling, so the helper reproduces an existing in-repo pattern at the same commit.

Program deployment needs no helper: rc3 already ships `wallet deploy-program <elf>` as a CLI
subcommand (`wallet/src/cli/mod.rs` `Command::DeployProgram`), which wraps
`ProgramDeploymentTransaction::new` + `send_transaction`. The testnet path shells out to that.

A LEZ fork/backport was rejected because PR #491 was authored after the
`wallet-ffi → lez/wallet-ffi` / `nssa → lee` restructure (commit `50be7458`), so its diffs target
paths that do not exist at rc3, which means maintaining a fork of an entire upstream repo with
merge-conflict risk on every sync. The helper's retirement cost is minimal: when the testnet
runs a LEZ revision containing PR #491 and #510, delete the helper, delete the
`CHAIN=testnet` submit branch, and restore the FFI path. No changes to
`payment_streams_module`, `payment_streams_ffi`, `instruction_abi.rs`, or any core code. The
helper is reached through a `CHAIN` selector; flipping it back to `ffi` is a one-line or config
change.

Design discipline: the helper's input/output contract must mirror what
`send_generic_public_transaction_json` already expects, so the `ffi` and `helper` branches are
interchangeable from `payment_streams_module`'s perspective. Concrete shapes are in the next
section (derived from `payment_streams_module_writes.cpp` `submitGenericPublic` /
`parseWalletSubmitJson`).

#### Helper wire contract, binary discovery, testnet wallet layout

Input to `submit-public-tx` is the same JSON object the module already builds for
`send_generic_public_transaction_json` (do not add alternate field names):

| Field | Type | Role |
| --- | --- | --- |
| `account_ids` | array of 64-char hex account ids | Message account list |
| `signing_requirements` | array of booleans | Witness flags per account |
| `instruction_hex` | hex string | Serialized guest instruction bytes |
| `program_elf_hex` | hex string | Payment-streams guest ELF (may be empty when the wallet process preloads guest ELF; helper must accept empty and resolve ELF from `--program-elf` / env when needed) |
| `program_dependencies_hex` | array of hex strings | Builtin dependency ELFs (e.g. rc3 authenticated transfer when required) |

The module passes wallet config/storage paths as CLI flags (or env documented in the runbook),
not inside this JSON.

Output on stdout: JSON in the wallet submit shape consumed by `parseWalletSubmitJson`
(`success`, `tx_hash`, and `error` on failure) — not the module's outer `{ status, wallet, … }`
wrapper. The testnet branch parses helper stdout like the wallet string returned from
`send_generic_public_transaction_json`, then reuses the same wrap logic as the local path.
On failure: non-zero exit and error text on stderr.

Binary discovery when `CHAIN=testnet`:

- Override: `LEZ_TESTNET_SUBMIT` (absolute path to the helper).
- Default: `lez-testnet-submit` on `PATH` (nix flake output or `tools/` install from the
  testnet-only build). The module subprocess runs only when `CHAIN=testnet`; demo scripts must
  export `CHAIN=testnet` for both `logoscore` hosts so in-process `chainAction` hits the helper.

Testnet wallet layout (single directory per logical role, shared across pins):

- Create/import keys once (rc3 `wallet` CLI against testnet is the supported bootstrap path if
  storage format must match the helper's `WalletCore::new_update_chain`).
- `wallet_config.json` with testnet `sequencer_addr`; same paths for `WALLET_CONFIG` /
  `WALLET_STORAGE` during `logos_execution_zone open` (510 module: reads, `sign_public_payload`,
  `sync_to_block`) and for helper submits (rc3: signing keys + nonces). If rc3 cannot open
  storage created by the 510 module, document the rc3-first creation flow in the Step 18 runbook
  and adjust bootstrap scripts (validate in Phase 1 or Phase 7).

Open policy choices (record in runbook when decided; not blockers for local demo):

- `fixtures/testnet.json.example` in git; operator-specific `fixtures/testnet.json` gitignored
  unless the org commits a shared non-secret manifest after one-time bootstrap.
- `clock_10_account_id` in the manifest must come from testnet recon, not copied from
  `fixtures/localnet.json`.

#### What stays local vs what moves to testnet

| Layer | Step 17 | Step 18 |
| --- | --- | --- |
| LEZ sequencer | `127.0.0.1:3040` (`lgs localnet`) | Public testnet URL (documented) |
| `logoscore` user + provider | Two local daemons | Same: two local daemons |
| Delivery P2P / Store | Local TCP ports, provider archives SQLite | Same topology; multiaddrs remain loopback in the default script |
| Eligibility hooks | Local provider verifier + user provider registration | Same module behavior; chain reads via 510 wallet RPC to testnet; `chainAction` writes via rc3 helper |
| Program on chain | Deploy per local seed (every fresh reset) | Deploy once on testnet; stable `program_id_hex` in fixture |
| Vault/stream bootstrap | `seed-onchain` / `prefund-onchain` + `create-stream-onchain` per fresh seed | One-time `bootstrap-testnet`; reused across runs |
| Chain reset | `demo-localnet-fresh.sh` / `FULL_RESET=1` wipes ledger | Never reset testnet chain; only reset local `PERSIST_*` |

#### Configuration that must match (sync checklist)

All of the following must refer to the same public network and deployed program.
Mismatches produce deploy failures, `STREAM_NOT_ACTIVE`, or verify rejections with no Store
symptoms.

| Setting | Where | Purpose |
| --- | --- | --- |
| `sequencer_addr` | Each host `wallet_config.json` used at `logos_execution_zone open` | Wallet RPC: reads, sync, `sign_public_payload` (not `chainAction` submits; those use rc3 helper) |
| `sequencer_url` | `fixtures/testnet.json` (`FIXTURE_MANIFEST`) | Operator docs, scripts, verify parity with wallet |
| LEZ revision | [`feature-branch-pins.md`](../../feature-branch-pins.md): local pin `62d9ba10` (default `.lgx` + `scaffold.toml`), testnet pin `cf3639d8` only for helper CLI + `deploy-testnet` wallet | Sequencer builtins are rc3; 510 module for reads/signing; rc3 for submits and deploy (see dual-pin section above). `wallet check-health` is not a testnet gate |
| `program_id_hex` | `fixtures/testnet.json` | Module PDA derivation and `chainAction` program binding ([N10](../../reference/decisions-and-notes.md#n10-step-11b-module-writes-decisions)) |
| Owner / provider account ids, vault/stream ids, derived PDAs | `fixtures/testnet.json` | Prepare/verify and on-chain stream state |
| `clock_10_account_id` | `fixtures/testnet.json` | Testnet clock account (do not copy local-only id without validation) |
| Guest program for submits | `PAYMENT_STREAMS_GUEST_BIN` (local FFI path) or helper's `--program-elf` (testnet path) | Writes (`createStream`, `claim`) against testnet deployed ELF |
| Provider libp2p identity | `E2E_PROVIDER_AD` (written by script) | User `registerProviderMapping` + `storeQuery` target — still local provider peer in Step 18 |
| Provider LEZ payee | `provider_account_id` in manifest | Must match bytes in proofs and on-chain `StreamConfig.provider` ([N5](../../reference/decisions-and-notes.md#n5-provider-identity-mapping)) |
| Module install | `MODULES_USER`, `MODULES_PROVIDER` | Same forked `.lgx` set as Step 17 ([feature-branch-pins.md](../../feature-branch-pins.md)) |
| Off-chain eligibility state | `--persistence-path` per host (`PERSIST_USER`, `PERSIST_PROVIDER`) | Session keys, mappings, provider acceptances ([N4](../../reference/decisions-and-notes.md#n4-persistence-policy)); separate from chain persistence |

Wallet and manifest: [N10 fixture and config](../../reference/decisions-and-notes.md#n10-step-11b-module-writes-decisions).
Delivery `createNode` defaults: [step17-e2e-local.md](../../step17-e2e-local.md#delivery-createnode-defaults) (local ports unchanged).

##### Preliminaries

Reconnaissance complete (2026-06-23):

- Sequencer liveness: `https://testnet.lez.logos.co/` responds to `getLastBlockId`
  (block `66256`). No code change needed in `run_local_e2e.py` — it already parses the
  `result` int shape.
- Compatible LEZ pin for testnet sequencer + rc3 tooling: `cf3639d8` (`v0.2.0-rc3`);
  `wallet check-health` with rc3 wallet against testnet exits 0.
- Pin `62d9ba10` vs testnet: `check-health` fails (builtin id mismatch); dual-pin semantics
  documented above. Read-path smoke test still pending (recon bullet).
- Piñata funding on testnet: `wallet pinata claim` succeeds, balance `150`.
- Program deploy: rc3 `wallet deploy-program` works for one-time deploy; per-run
  `chainAction` writes need the temporary helper (no generic-tx FFI at rc3).

Implementation pending on `feat/step18-public-testnet` (may be partial on branch; verify in git):

- Branch from `master`, `CHAIN` env (`local` default, `testnet` for public sequencer path).
  When `CHAIN` is unset or `local`, behavior must remain byte-for-byte Step 17.
- Phases 1–9 below (Part A first, then Part B when RPC is up).

#### Implementation order

Work proceeds in phases on `feat/step18-public-testnet`. After each phase in Part A, the
local-LEZ regression gate must still be green (see non-regression guard below). Part B needs
the public testnet RPC reachable; run `./scripts/verify-step18-testnet-read-smoke.sh` (PASS,
not skip) before Part B chain writes.

#### Part A — no live public testnet required

##### Phase 1 — Build the temporary submit helper

1. Add a small Rust binary crate `lez-testnet-submit` under `tools/` (name tentative). It is
    a standalone CLI, not a `wallet_ffi` extension. It links against the rc3 LEZ crates
    (`nssa`, `wallet`) pinned at `cf3639d8` via a testnet-only Nix flake input, completely
    separate from the local-path `62d9ba10` pin.
2. The helper must implement exactly the operations `payment_streams_module` needs for
    `chainAction` writes. Single subcommand `submit-public-tx`:
    - Input (stdin or `--arg` file): JSON per table in Helper wire contract (`account_ids`,
      `signing_requirements`, `instruction_hex`, `program_elf_hex`, `program_dependencies_hex`).
    - Wallet paths: `--wallet-config`, `--wallet-storage` (same dirs as `logos_execution_zone open`).
    - Behavior: open wallet at rc3, resolve nonces, load signing keys, run the rc3 four-step
      public submit pattern (`Message::try_new` → `WitnessSet::for_message` →
      `PublicTransaction::new` → `send_transaction`).
    - Output: wallet-shaped JSON on stdout for `parseWalletSubmitJson`; non-zero exit on failure.
    - No `deploy-program` subcommand (rc3 `wallet deploy-program` covers one-time deploy).
3. The helper does not touch `wallet_ffi`, `payment_streams_ffi`, or the C bridge. When
    `CHAIN=testnet`, `payment_streams_module` spawns it (`LEZ_TESTNET_SUBMIT` or `PATH`). No
    LogosAPI variant. Local `CHAIN=local` unchanged (FFI only).
4. Unit-test the helper against a local rc3 sequencer first (start rc3 locally, submit a
    trivial public tx, assert the hash is non-empty). This isolates helper correctness from
    testnet liveness.
5. Gate the helper behind `CHAIN=testnet`. When `CHAIN` is unset or `local`, the helper is
    never compiled into the default `nix build` and never invoked. The local path continues
    to call `wallet_ffi_send_generic_public_transaction` through the existing C bridge.

##### Phase 2 — Selector in `payment_streams_module`

6. Add a `CHAIN` selector at chain-write dispatch (env read inside the module process; demo
    exports `CHAIN=testnet` for both hosts). When `CHAIN=testnet`, shell out to
    `lez-testnet-submit submit-public-tx` with the same JSON as
    `send_generic_public_transaction_json`; when `CHAIN=local` or unset, FFI only. Instruction
    encoding (`instruction_abi.rs`, `instruction_wire.rs`) stays shared.
7. The selector is a single dispatch point, not a scatter of conditionals. Concretely: one
    function `submit_chain_action(action_json)` that branches on `CHAIN`. Everything upstream
    of it (instruction encoding, PDA derivation, account-id resolution) is backend-agnostic and
    shared.
8. The `CHAIN=local` branch is byte-for-byte the current FFI call. Verify this with a diff
    review: the only new code on the local path is the selector reading the env var.

##### Phase 3 — Testnet operator scaffolding

9. Create a testnet wallet template under `.scaffold/e2e/testnet-wallet/wallet_config.json`
    with `sequencer_addr` set to the testnet URL. Document copy-to-gitignored paths for
    `WALLET_CONFIG` / `WALLET_STORAGE` (see testnet wallet layout — same dirs for 510 module
    and rc3 helper).
10. Ship `fixtures/testnet.json.example` in git (no secrets; include recon `program_id_hex`
    where known). Default: operators copy to gitignored `fixtures/testnet.json` or set
    `FIXTURE_MANIFEST`. An org-wide committed manifest is optional policy; record
    `clock_10_account_id` from testnet recon in the example or runbook, not from localnet.
11. The existing `verify-step10b-dod.sh` `127.0.0.1:3040` assertion is the local-LEZ gate and
    stays unchanged. It is not run for `CHAIN=testnet`.

##### Phase 4 — Runbook and pins

12. Write `docs/step18-public-sequencer-e2e.md` covering:
    - Prerequisites (internet egress, testnet wallet, funded account).
    - Part A vs Part B phase order and dual-pin read smoke before chain bootstrap.
    - One-time bootstrap: `make deploy-testnet`, `make bootstrap-testnet` (Part B).
    - Repeatable run: `make verify-step18` (Part B, after bootstrap).
    - The temporary helper: what it is, why it exists, and the retirement trigger (testnet
      runs a LEZ revision containing PR #491 and #510).
    - Persistence rules (do not wipe chain state; only reset local `PERSIST_*`).
    - Failure triage for testnet-specific cases (unreachable sequencer, stale program id,
      faucet rate limits, stream depletion, dual-pin read smoke failing after RPC is up).
13. Update [`feature-branch-pins.md`](../../feature-branch-pins.md) with:
    - The testnet LEZ revision (`cf3639d8`) for helper + `deploy-testnet` CLI.
    - The deployed `program_id_hex` (after Part B deploy).
    - A "retirement" section pointing at Phase 9 below.

##### Phase 5 — Demo and Makefile wiring (local CI unchanged)

14. Teach `scripts/demo-e2e-local.sh` (or a new `scripts/demo-e2e-testnet.sh`) to read `CHAIN`
    and pick `FIXTURE_MANIFEST` / `WALLET_CONFIG` accordingly. When `CHAIN=testnet`:
    - Skip `lgs localnet start`, `demo-localnet-fresh.sh`, `demo-localnet-prepare.sh`, and
      snapshot restore.
    - Skip `make deploy` / `make setup` (program is already deployed, accounts already
      funded).
    - Still run `nix build` + `lgpm install` for the three modules, and still start two local
      `logoscore` daemons for user and provider.
    - The `sequencer_url` in the manifest drives wallet sync; no code change needed in
      `run_local_e2e.py` (it already reads `manifest.get("sequencer_url", ...)`).
15. The late-stream-create / top-up path in `run_local_e2e.py` (`ensure_fresh_demo_stream`,
    `user_prepare_proof`) stays. It is a vault lifecycle op, not a program deploy, and is
    needed when the testnet stream is depleted. Document that testnet stream depletion is
    real (accrual runs between prepare and verify) and top-up is the expected remedy.
16. Add Makefile targets: `verify-step18-testnet-read-smoke` (wraps the script), placeholder
    `deploy-testnet` / `bootstrap-testnet` if not yet implemented, and `verify-step18` that
    will run the full testnet demo in Phase 8. Wiring may land in Part A; targets may skip or
    fail until Part B is complete. CI default remains `verify-step17`.

#### Part B — public testnet RPC required (read smoke PASS first)

##### Phase 6 — Read gate and program deploy (one-time)

17. Run `./scripts/verify-step18-testnet-read-smoke.sh` until PASS (not skip). Do not rely on
    Part B chain I/O until dual-pin reads are confirmed.
18. Add/finish `make deploy-testnet`:
    - Reads `WALLET_CONFIG` pointing at a testnet wallet config
      (`sequencer_addr = https://testnet.lez.logos.co/`).
    - Runs `wallet deploy-program $(PROGRAM_BIN)` (rc3 CLI, not the helper) against the
      testnet sequencer.
    - Records the resulting `program_id_hex` (should equal `make program-id` output).
    - Is idempotent for re-runs: probe the rc3 CLI's behavior on a duplicate deploy (run it
      twice against the same ELF on a throwaway testnet account first), capture the exact exit
      code and error string for the "already deployed" case, and have the Makefile target
      treat that specific outcome as success. Fail on any other error.
19. Do not wire `deploy-testnet` into `make verify-step18`. It is a one-time bootstrap step.

##### Phase 7 — Testnet fixture bootstrap (one-time, or rare)

20. Add/finish `make bootstrap-testnet` that runs `initialize_vault`, `deposit`, and
    `create_stream` against testnet. Vault/stream bootstrap uses the helper's
    `submit-public-tx` (these are `chainAction` writes: `initializeVault`, `deposit`,
    `createStream`) since rc3 has no CLI for them. Writes or updates `fixtures/testnet.json`
    with the resulting ids and `program_id_hex` from Phase 6.
21. `bootstrap-testnet` must be idempotent for partial state, not assume a fresh chain. It does
    not require a running `logoscore`. Before each sub-step, read chain state via rc3
    `wallet` CLI account/query commands against testnet and/or sequencer JSON-RPC
    `getAccount` on derived PDAs from `program_id_hex` + manifest ids (same bytes the module
    decodes after `get_account_public`). If a read path is missing at rc3, add a minimal
    `get-account-public` (or equivalent) subcommand to the helper rather than spawning the
    module stack.
    - If `vault_id` is already in the manifest and vault accounts exist on chain, skip
      `initialize_vault`.
    - Before `create_stream`, inspect stream config / folded balance; if depleted,
      `topUpStream`; if healthy, no-op; if missing, `createStream`.
    - `deposit` only if vault balance is below the bootstrap target.
22. Kept separate from Phase 6 because funding (Piñata or faucet) may need retries and is
    independent of program deployment.

##### Phase 8 — Repeatable verify-step18

23. `make verify-step18` runs the full testnet path: `getLastBlockId` (fail fast if
    unreachable), read smoke (or rely on Phase 6 gate), then dual-host demo with
    `CHAIN=testnet`. Must not start a local sequencer or call `make deploy` / `make setup` /
    `demo-localnet-fresh.sh`. Required on the feature branch before merge (maintainer /
    network access).

#### Deferred (not Part A or B implementation on the feature branch)

##### Phase 9 — Retirement (triggered by testnet catch-up)

24. Documented procedure only, triggered when the public testnet runs a LEZ revision that
    contains PR #491 and #510 (detected by `wallet check-health` passing with the current
    local pin, or by confirming `wallet_ffi_send_generic_public_transaction` is present in the
    testnet wallet binary):
    - Delete `tools/lez-testnet-submit` (or its successor location).
    - Remove the `CHAIN=testnet` branch from `submit_chain_action`; the FFI call becomes the
      sole path.
    - Drop the testnet-only Nix flake input for the rc3 LEZ pin.
    - Re-point the testnet wallet pin to the new testnet revision in `feature-branch-pins.md`.
    - Re-run `make verify-step18` to confirm the FFI path works against the updated testnet.
25. No changes to `payment_streams_module` instruction encoding, `payment_streams_ffi`, the C
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
- `scripts/demo-e2e-testnet.sh` or `CHAIN=testnet` in the Step 17 orchestrator: no
  `lgs localnet start` / `demo-localnet-fresh.sh` on the chain path; still starts two local
  `logoscore` hosts; CI default remains Step 17 local LEZ.
- Makefile targets `deploy-testnet`, `bootstrap-testnet`, `verify-step18` (names tentative).

#### Definition of done

- Documented public sequencer URL and dual-pin policy
  ([`feature-branch-pins.md`](../../feature-branch-pins.md)): `scaffold.toml` and default
  `logos_execution_zone` stay on `62d9ba10`; rc3 `cf3639d8` only for helper + `deploy-testnet`
  CLI; testnet demo reads/sign via 510 module, submits via helper.
- `lez_payment_streams` deployed on that network via rc3 `wallet deploy-program`; demo uses
  committed or operator manifest with matching `program_id_hex` and bootstrapped vault/stream
  state.
- The temporary submit helper (`tools/lez-testnet-submit`) drives `chainAction` writes
  (`createStream`, `topUpStream`, `claim`) against testnet; its JSON contract matches
  `send_generic_public_transaction_json` so the `CHAIN` selector is the only module-side
  difference between backends.
- Dual-host paid Store success and inbound eligibility failure match Step 17 outcomes (same wire
  and modules; chain fixture on testnet).
- Operator can reproduce without wiping chain state; persistence rules documented (manifest +
  vault reuse; when to reset local `PERSIST_*` only).
- One-time deploy (`make deploy-testnet`) and one-time bootstrap (`make bootstrap-testnet`) are
  documented as distinct from the repeatable run (`make verify-step18`).
- Retirement procedure (Phase 9) is documented and requires no changes to core code; it is
  triggered when the testnet runs a LEZ revision containing PR #491 and #510.

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

Follow-on: Step 19 (LIP on-chain, done). Step 20 testnet journey rows use this step when
unblocked; local journey does not require Step 18. Optional Step 23 (hosted provider).
