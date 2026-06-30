# Step 18 — plan excerpt

> Superseded by Step 26 (TestNet v0.2.0 migration). The rc5 operational pin and
> PR 19 wallet-module ref recorded here were replaced by the v0.2.0 pin
> (`a58fbce…`) and the `main` (Universal) wallet module. Retained in
> `completed/` for step-map history.

Active-work packet for agents. Index: [index.md](../index.md).

#### Integration status

Public sequencer at `https://testnet.lez.logos.co/` uses lez jsonrpsee RPC. Org guest deploy
for the current tree is on chain (see Verified org deploy below). Step 18b (rc5 operational pin)
is merged to `master`. Step 18 Part B DoD: `make verify-step18-testnet-read-smoke`, `make bootstrap-testnet`,
`make verify-step18` with unified rc5 tooling ([N16](../../reference/integration-decisions.md#n16-step-18b-rc5-operational-pin-2026-06),
[`archive/steps/public-sequencer-store-runbook.md`](../../archive/steps/public-sequencer-store-runbook.md)). Step 20 testnet journey
rows may cite this step; local Step 17 uses the same operational pin for localnet.

### Step 18, Public sequencer E2E (local Store and relay)

Prerequisite: Step 17 definition of done satisfied on local LEZ
([`scripts/e2e.sh`](../../../scripts/e2e.sh) `local run`; archived `scripts/archive/demo-e2e-local.sh`, [N12](../../../reference/decisions-historical.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)).

Branch: `feat/step18-public-testnet` from `master`. All changes are additive and gated by a
`CHAIN` selector (`CHAIN=local` default, `CHAIN=testnet` for this step). The local-LEZ path
(`make verify-step17`) must keep passing unchanged on `master` throughout the work; see the
non-regression guard at the end of this packet.

Architectural context:

Step 17 uses a disposable local LEZ sequencer (`127.0.0.1:3040`) and two `logoscore`
processes on one machine. Relay, GossipSub, and Store traffic stay on localhost (disjoint
`portsShift`, user `staticnode` dial to provider multiaddr from
[`E2E_PROVIDER_AD`](../../archive/steps/local-store-dual-host-runbook.md#provider-service-advertisement-off-band-mimic)).

Step 18 keeps that same dual-host P2P layout. Only LEZ chain access moves to the org
public testnet sequencer (testnet v0.2 target). The public endpoint is documented in
`logos-docs/docs/lez/get-started/quickstart-for-the-logos-execution-zone-wallet.md` as
`https://testnet.lez.logos.co/` (explorer `https://explorer.testnet.lez.logos.co/`). That doc
asserts the endpoint exists but does not document the `wallet_config.json` schema, the
`sequencer_addr` field, the testnet LEZ revision, whether `wallet pinata claim` works against
it, or how to deploy a LEE guest program there. Operator setup is documented in this packet
and in `docs/archive/steps/public-sequencer-store-runbook.md` (Phase 4).

Do not apply local reset-first policy from
[`archive/operator/localnet-recovery.md`](../../archive/operator/localnet-recovery.md). See testnet persistence in
[`archive/steps/user-eligibility-runbook.md`](../../archive/steps/user-eligibility-runbook.md) (Persistence across runs).

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
binary targets local LEZ fixture flows. Testnet (2) is rc5 `wallet deploy-program` (or skip when
org guest is already deployed); testnet (3) is `make bootstrap-testnet` (helper or module submits +
manifest write) with no localnet preamble. The lifecycle stages match local; paths differ only by
`CHAIN` and sequencer URL.

Program deploy on testnet is network-one-time: rc5 `wallet deploy-program` against the public
sequencer; the deployed program is callable by any wallet on that chain. The `program_id_hex`
is shared across operators for a given guest ELF, not per-operator.

#### Public testnet (operator facts)

- Sequencer URL: `https://testnet.lez.logos.co/` (explorer
  `https://explorer.testnet.lez.logos.co/`).
- RPC: lez jsonrpsee — `getLastBlockId` (JSON `result` int), `getProgramIds`, `getAccount`,
  `sendTransaction`, and related methods used by the rc5 `wallet` CLI, module reads, and read smoke.
- Operational LEZ pin: **`v0.2.0-rc5`** (`27360cb7d6ccb2bfbcca7d171bab8a3938490264`) for local
  E2E, `logos_execution_zone` .lgx, testnet scripts, and `tools/lez-testnet-submit`. See
  [feature-branch-pins.md](../../reference/feature-branch-pins.md) and [N16](../../reference/integration-decisions.md#n16-step-18b-rc5-operational-pin-2026-06).
- Guest ELF (current tree): **576576** bytes; ImageID / `program_id_hex`
  `79b1dd5c441caede8f9f82c30de637aba465f94cc43817b1105c8c48c77d0fc9` (`make program-id`).
- Read gate: `make verify-step18-testnet-read-smoke` — rc5 module `open`, `sync_to_block`,
  `get_account_public` on testnet; PASS (not skip-only) before bootstrap.
- `wallet check-health` with rc5 CLI and testnet `sequencer_addr` is a valid smoke check when
  wallet home is `.scaffold/e2e/testnet-wallet/`.
- Manifest policy: committed `fixtures/testnet.json.example`; gitignored per-operator
  `fixtures/testnet.json` after bootstrap. The example `clock_10_account_id` value is the testnet
  Clock01 id (`4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWNU`): the demo reads the clock every
  block via Clock01 so the create-time fold stays small (see
  [step-24c-simplify-demo-flow.md](../completed/step-24c-simplify-demo-flow.md)). The field name
  is a legacy key; do not copy from `localnet.json`.
- Demo claim policy: on testnet the provider `claim` is optional. A run is green through create,
  fundable, paid Store query, and close; claim is not reliably confirming on testnet. See
  [archive/operator/testnet-claim-known-issue.md](../../archive/operator/testnet-claim-known-issue.md).

#### Verified org deploy log (2026-06-25)

Recorded on `feat/step18-public-testnet`. Shared on-chain guest id for the current guest tree
(org deploy; copy into manifests per policy).

Pre-check RPC (`getLastBlockId`, jsonrpsee POST to sequencer URL):

- HTTP status **200**
- `result` block id **3291** immediately before deploy (height moves with chain time)

Command (rc5 `wallet` after `lgs setup` or cargo build at pin `27360cb7…`; testnet wallet home
under `.scaffold/e2e/testnet-wallet` with `sequencer_addr` `https://testnet.lez.logos.co/`):

```bash
export LEE_WALLET_HOME_DIR=/path/to/lez-payment-streams/.scaffold/e2e/testnet-wallet
export NSSA_WALLET_HOME_DIR="$LEE_WALLET_HOME_DIR"
wallet deploy-program \
  methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin
```

Observed outcome (2026-06-25 org deploy; rc3-era CLI was used historically — same ELF and ImageID):

- Shell exit code **0**
- Wallet stdout and stderr **empty** on success (repeat invocation also exit 0; treat as
  idempotent once the program is on chain)
- Guest ELF size **576576** bytes
- `program_id_hex` (ImageID): `79b1dd5c441caede8f9f82c30de637aba465f94cc43817b1105c8c48c77d0fc9`
  (`make program-id` / `lez_payment_streams_cli program-id` on the same ELF path)
- Deploy transaction hash (`deploy_tx_hash`): `1787368626484789a2976a2aa8631d2b5b39c415c0a74b5a345474d1415f79b1`
  (included in block **3284**; `getTransaction` on the testnet sequencer returns the
  `ProgramDeployment` payload). The `wallet deploy-program` CLI may not print this hash.
- Block explorer (transaction): `https://explorer.testnet.lez.logos.co/transaction/1787368626484789a2976a2aa8631d2b5b39c415c0a74b5a345474d1415f79b1`
- Block explorer (block): `https://explorer.testnet.lez.logos.co/block/3284`

Finding the deployment in the explorer:

1. Open the transaction URL above, or paste `deploy_tx_hash` into the explorer search box.
2. To locate the block, search block id `3284` or open the block URL above; the deploy is the
   large `ProgramDeployment` transaction in that block (guest bytecode begins with RISC Zero
   `R0BF` magic).
3. The guest program is identified by `program_id_hex` (ImageID), not a separate explorer
   program page today; use the transaction view or `make program-id` for the id.

Implementation note: `make deploy-testnet` invokes rc5 `wallet deploy-program` via
`scripts/testnet-common.sh`.

#### Operational LEZ (rc5) and temporary submit helper

Step 18 uses the same operational LEZ pin as Step 17 and local E2E: **`v0.2.0-rc5`**
(`27360cb7d6ccb2bfbcca7d171bab8a3938490264`) in `scaffold.toml`, wallet flakes, module
`.lgx`, testnet scripts, and `tools/lez-testnet-submit`. Public testnet builtins and LEE v0.3
public message hashing match that revision ([N16](../../reference/integration-decisions.md#n16-step-18b-rc5-operational-pin-2026-06)).

Dual-pin (510 module reads + rc3 writes) was Step 18 WIP only; it is not supported. Do not
document or configure split pins for testnet.

For `CHAIN=testnet`, `payment_streams_module` may still spawn `lez-testnet-submit` for
`chainAction` public submits until Phase 9 retires the helper. The helper links rc5 `lee` /
`wallet` crates (same pin as the module), accepts the same JSON as
`send_generic_public_transaction_json`, and prints wallet-shaped stdout for
`parseWalletSubmitJson`. Program deploy uses rc5 `wallet deploy-program` (`make deploy-testnet`).

Rust `program_tests` in `lez-payment-streams-core` may remain on PR 510 (`62d9ba10`) for Step 24
harness work; that dep does not define the operational pin.

Phase 9 retirement: delete the helper and the `CHAIN=testnet` submit branch in C++ when module
`chainAction` on testnet works without the subprocess (no LEZ fork required).

Design discipline: the helper's input/output contract mirrors
`send_generic_public_transaction_json` so the `ffi` and `helper` branches are interchangeable
from `payment_streams_module`'s perspective. Concrete shapes are in the next section (derived
from `payment_streams_module_writes.cpp` `submitGenericPublic` / `parseWalletSubmitJson`).

#### Helper wire contract, binary discovery, testnet wallet layout

Input to `submit-public-tx` is the same JSON object the module already builds for
`send_generic_public_transaction_json` (do not add alternate field names):

| Field | Type | Role |
| --- | --- | --- |
| `account_ids` | array of 64-char hex account ids | Message account list |
| `signing_requirements` | array of booleans | Witness flags per account |
| `instruction_hex` | hex string | Serialized guest instruction bytes |
| `program_elf_hex` | hex string | Payment-streams guest ELF (may be empty when the wallet process preloads guest ELF; helper must accept empty and resolve ELF from `--program-elf` / env when needed) |
| `program_dependencies_hex` | array of hex strings | Builtin dependency ELFs from rc5 wallet when required |

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

Testnet wallet layout (single directory per logical role):

- Create/import keys with rc5 `wallet` against testnet (`LEE_WALLET_HOME_DIR` /
  `.scaffold/e2e/testnet-wallet/`).
- `wallet_config.json` with testnet `sequencer_addr`; same paths for `WALLET_CONFIG` /
  `WALLET_STORAGE` during `logos_execution_zone open` (reads, `sign_public_payload`,
  `sync_to_block`) and for helper submits.

Open policy choices (record in runbook when decided; not blockers for local demo):

- `fixtures/testnet.json.example` in git; operator-specific `fixtures/testnet.json` gitignored
  unless the org commits a shared non-secret manifest after one-time bootstrap.
- `clock_10_account_id` in the manifest must match the testnet Clock01 id used by the demo (see
  `fixtures/testnet.json.example`), not copied blindly from `fixtures/localnet.json`. The field
  name is a legacy key whose value now points at Clock01.

#### What stays local vs what moves to testnet

| Layer | Step 17 | Step 18 |
| --- | --- | --- |
| LEZ sequencer | `127.0.0.1:3040` (`lgs localnet`) | Public testnet URL (documented) |
| `logoscore` user + provider | Two local daemons | Same: two local daemons |
| Delivery P2P / Store | Local TCP ports, provider archives SQLite | Same topology; multiaddrs remain loopback in the default script |
| Eligibility hooks | Local provider verifier + user provider registration | Same module behavior; chain reads and writes against testnet via rc5 wallet + helper when `CHAIN=testnet` |
| Program on chain | Deploy per local seed (every fresh reset) | Deploy once on testnet; stable `program_id_hex` in fixture |
| Vault/stream bootstrap | `seed-onchain` / `prefund-onchain` + `create-stream-onchain` per fresh seed | One-time `bootstrap-testnet`; reused across runs |
| Chain reset | `make full-reset-localnet` wipes local ledger | Never reset testnet chain; only reset local `PERSIST_*` |

#### Configuration that must match (sync checklist)

All of the following must refer to the same public network and deployed program.
Mismatches produce deploy failures, `STREAM_NOT_ACTIVE`, or verify rejections with no Store
symptoms.

| Setting | Where | Purpose |
| --- | --- | --- |
| `sequencer_addr` | Each host `wallet_config.json` used at `logos_execution_zone open` | Wallet RPC: reads, sync, `sign_public_payload`; testnet submits via helper or FFI per `CHAIN` |
| `sequencer_url` | `fixtures/testnet.json` (`FIXTURE_MANIFEST`) | Operator docs, scripts, verify parity with wallet |
| LEZ revision | [`feature-branch-pins.md`](../../reference/feature-branch-pins.md): operational pin **`v0.2.0-rc5`** (`27360cb7…`) for scaffold, module, scripts, helper | Single pin for local Step 17 and testnet; harness `program_tests` may lag on 510 ([N16](../../reference/integration-decisions.md#n16-step-18b-rc5-operational-pin-2026-06)) |
| `program_id_hex` | `fixtures/testnet.json` | Module PDA derivation and `chainAction` program binding ([N10](../../reference/integration-decisions.md#n10-step-11b-module-writes-decisions)) |
| Owner / provider account ids, vault/stream ids, derived PDAs | `fixtures/testnet.json` | Prepare/verify and on-chain stream state |
| `clock_10_account_id` | `fixtures/testnet.json` | Testnet clock account (do not copy local-only id without validation) |
| Guest program for submits | `PAYMENT_STREAMS_GUEST_BIN` (local FFI path) or helper's `--program-elf` (testnet path) | Writes (`createStream`, `claim`) against testnet deployed ELF |
| Provider libp2p identity | `E2E_PROVIDER_AD` (written by script) | User `registerProviderMapping` + `storeQuery` target — still local provider peer in Step 18 |
| Provider LEZ payee | `provider_account_id` in manifest | Must match bytes in proofs and on-chain `StreamConfig.provider` ([N5](../../reference/integration-decisions.md#n5-provider-identity-mapping)) |
| Module install | `MODULES_USER`, `MODULES_PROVIDER` | Same forked `.lgx` set as Step 17 ([feature-branch-pins.md](../../reference/feature-branch-pins.md)) |
| Off-chain eligibility state | `--persistence-path` per host (`PERSIST_USER`, `PERSIST_PROVIDER`) | Session keys, mappings, provider acceptances ([N4](../../reference/integration-decisions.md#n4-persistence-policy)); separate from chain persistence |

Wallet and manifest: [N10 fixture and config](../../reference/integration-decisions.md#n10-step-11b-module-writes-decisions).
Delivery `createNode` defaults: [archive/steps/local-store-dual-host-runbook.md](../../archive/steps/local-store-dual-host-runbook.md#delivery-createnode-defaults) (local ports unchanged).

##### Branch status

Part A (phases 1–5) and Part B scaffolding are on `master` / `feat/step18-public-testnet`.
Org guest deploy on public testnet is complete (2026-06-25). Part B DoD: read smoke PASS,
`make bootstrap-testnet`, `make verify-step18` with unified rc5 tooling (verified locally after
Step 18b merge).

- `CHAIN` env (`local` default, `testnet` for public sequencer path). When `CHAIN` is unset or
  `local`, behavior must remain byte-for-byte Step 17.
- Phases 6–9 below (Part B when read smoke PASS).

#### Implementation order

Work proceeds in phases on `feat/step18-public-testnet`. After each phase in Part A, the
local-LEZ regression gate must still be green (see non-regression guard below). Part B needs
the public testnet RPC reachable; run `make verify-step18-testnet-read-smoke` (PASS,
not skip) before Part B chain writes.

#### Part A — no live public testnet required

##### Phase 1 — Build the temporary submit helper

1. `tools/lez-testnet-submit` under `tools/`: standalone CLI linking rc5 `lee` / `wallet` at the
    operational pin (same as `scaffold.toml`). Not a separate testnet-only LEZ fork.
2. The helper must implement exactly the operations `payment_streams_module` needs for
    `chainAction` writes. Single subcommand `submit-public-tx`:
    - Input (stdin or `--arg` file): JSON per table in Helper wire contract (`account_ids`,
      `signing_requirements`, `instruction_hex`, `program_elf_hex`, `program_dependencies_hex`).
    - Wallet paths: `--wallet-config`, `--wallet-storage` (same dirs as `logos_execution_zone open`).
    - Behavior: open wallet at rc5, resolve nonces, load signing keys, run the public submit
      pattern (`Message::try_new` → `WitnessSet::for_message` → `PublicTransaction::new` →
      `send_transaction`) over the jsonrpsee sequencer client (`sendTransaction`).
    - Output: wallet-shaped JSON on stdout for `parseWalletSubmitJson`; non-zero exit on failure.
    - No `deploy-program` subcommand (rc5 `wallet deploy-program` covers one-time deploy).
3. The helper does not touch `wallet_ffi`, `payment_streams_ffi`, or the C bridge. When
    `CHAIN=testnet`, `payment_streams_module` spawns it (`LEZ_TESTNET_SUBMIT` or `PATH`). No
    LogosAPI variant. Local `CHAIN=local` unchanged (FFI only).
4. Optional: unit-test the helper against a local rc5 sequencer (trivial public tx submit).
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
    `WALLET_CONFIG` / `WALLET_STORAGE` (single rc5 wallet tree under `.scaffold/e2e/testnet-wallet/`).
10. Ship `fixtures/testnet.json.example` in git (no secrets; includes org `program_id_hex`
    for the current guest). Default: operators copy to gitignored `fixtures/testnet.json` or set
    `FIXTURE_MANIFEST`. An org-wide committed manifest is optional policy; use testnet
    `clock_10_account_id` from the example or runbook, not from localnet.
11. The existing `verify-step10b-dod.sh` `127.0.0.1:3040` assertion is the local-LEZ gate and
    stays unchanged. It is not run for `CHAIN=testnet`.

##### Phase 4 — Runbook and pins

12. Write `docs/archive/steps/public-sequencer-store-runbook.md` covering:
    - Prerequisites (internet egress, testnet wallet, funded account).
    - Part A vs Part B phase order and read smoke before chain bootstrap.
    - One-time bootstrap: `make deploy-testnet`, `make bootstrap-testnet` (Part B).
    - Repeatable run: `make verify-step18` (Part B, after bootstrap).
    - The temporary helper: what it is, why it exists, and Phase 9 retirement (module FFI only).
    - Persistence rules (do not wipe chain state; only reset local `PERSIST_*`).
    - Failure triage for testnet-specific cases (unreachable sequencer, stale program id,
      faucet rate limits, stream depletion, read smoke failing when the sequencer is up).
13. Update [`feature-branch-pins.md`](../../reference/feature-branch-pins.md) with operational rc5 pin and
    Phase 9 helper retirement pointer.

##### Phase 5 — Demo and Makefile wiring (local CI unchanged)

14. Teach [`scripts/e2e.sh`](../../../scripts/e2e.sh) (`local run` / `testnet run`; implemented) to read `CHAIN`
    and pick `FIXTURE_MANIFEST` / `WALLET_CONFIG` accordingly. When `CHAIN=testnet`:
    - Skip `lgs localnet start`, localnet prepare (`make prepare-localnet` /
      `make full-reset-localnet`), and snapshot restore.
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

17. Read gate: run `make verify-step18-testnet-read-smoke` until PASS
    (`getLastBlockId`, open/sync/`get_account_public` on CLOCK_10). Re-run if the sequencer was
    unreachable before bootstrap. Do not rely on Part B chain I/O if the latest smoke was skip
    or fail.
18. `make deploy-testnet`:
    - Reads testnet wallet paths (`scripts/testnet-common.sh`, rc5 `wallet_config` + storage).
    - Runs rc5 `wallet deploy-program $(PROGRAM_BIN)` against the testnet sequencer (not the
      helper).
    - Records the resulting `program_id_hex` (must equal `make program-id` output).
    - Idempotent handling for duplicate deploy documented in script.
    - Org deploy complete 2026-06-25 for guest ELF 576576 B and ImageID `79b1dd5c…` (see
      Verified org deploy log).
19. Do not wire `deploy-testnet` into `make verify-step18`. It is a one-time bootstrap step.

##### Phase 7 — Testnet fixture bootstrap (one-time, or rare)

20. `make bootstrap-testnet` (requires org program on chain). Runs vault/stream fixture via
    rc5 helper + unified wallet env; writes `fixtures/testnet.json`. See script and
    `examples/src/bin/bootstrap_testnet_fixture.rs`.
21. `bootstrap-testnet` must be idempotent for partial state, not assume a fresh chain. It does
    not require a running `logoscore`. Before each sub-step, read chain state via rc5 `wallet`
    CLI and/or sequencer JSON-RPC `getAccount` on derived PDAs from `program_id_hex` + manifest
    ids (same bytes the module decodes after `get_account_public`).
    - If `vault_id` is already in the manifest and vault accounts exist on chain, skip
      `initialize_vault`.
    - Before `create_stream`, inspect stream config / folded balance; if depleted,
      `topUpStream`; if healthy, no-op; if missing, `createStream`.
    - `deposit` only if vault balance is below the bootstrap target.
22. Kept separate from Phase 6 because funding (Piñata or faucet) may need retries and is
    independent of program deployment.

##### Phase 8 — Repeatable verify-step18

23. `make verify-step18` (wired at `ca1ba7f`; requires bootstrap complete). Intended flow:
    read smoke gate, dual-host demo with `CHAIN=testnet`. Must not start a local sequencer or
    call `make deploy` / `make setup` / localnet full reset (`make full-reset-localnet`).

#### Deferred (not Part A or B implementation on the feature branch)

##### Phase 9 — Retirement (triggered by testnet catch-up)

24. Documented procedure only, triggered when `make verify-step18` passes with module
    `chainAction` on testnet without the helper subprocess:
    - Delete `tools/lez-testnet-submit` (or its successor location).
    - Remove the `CHAIN=testnet` branch from `submit_chain_action`; the FFI call becomes the
      sole path.
    - Re-run `make verify-step18` to confirm the FFI path works against testnet.
25. No changes to `payment_streams_module` instruction encoding, `payment_streams_ffi`, the C
    bridge, `instruction_abi.rs`, or any core code are required at retirement. This is the
    key property the helper approach was chosen for.

#### Deliver

- Temporary submit helper `tools/lez-testnet-submit`: standalone Rust CLI using rc5 public-tx
  submit for `chainAction` when the module dispatches to the helper. Never compiled or invoked
  when `CHAIN` is unset or `local`.
- `CHAIN` selector in `payment_streams_module` at a single `submit_chain_action` dispatch
  point: `CHAIN=testnet` shells out to the helper; `CHAIN=local` (default) calls
  `send_generic_public_transaction_json` over the FFI bridge unchanged.
- Committed `fixtures/testnet.json.example` (no secrets; may include stable `program_id_hex`
  after org-wide deploy). Operators copy to gitignored `fixtures/testnet.json` or set
  `FIXTURE_MANIFEST`.
- Testnet bootstrap runbook (`docs/archive/steps/public-sequencer-store-runbook.md`): one-time program deploy
  (rc5 `wallet deploy-program`), one-time vault/stream bootstrap (via helper), repeatable demo
  run; document one-time vs repeatable steps explicitly; document Phase 9 helper retirement.
- Testnet wallet template(s): `sequencer_addr` aligned with manifest `sequencer_url` (per-host
  copies under `.scaffold/e2e/` or documented paths).
- `CHAIN=testnet` via [`scripts/e2e.sh`](../../../scripts/e2e.sh) `testnet run` (Step 17 orchestrator): no
  `lgs localnet start` / `make full-reset-localnet` on the chain path; still starts two local
  `logoscore` hosts; CI default remains Step 17 local LEZ.
- Makefile targets `deploy-testnet`, `bootstrap-testnet`, `verify-step18` (names tentative).

#### Definition of done

- Documented public sequencer URL and single operational LEZ pin rc5
  ([`feature-branch-pins.md`](../../reference/feature-branch-pins.md), [N16](../../reference/integration-decisions.md#n16-step-18b-rc5-operational-pin-2026-06)).
- `lez_payment_streams` deployed on public testnet via rc5 `wallet deploy-program` (org deploy
  2026-06-25); demo uses committed or operator manifest with matching `program_id_hex` and
  bootstrapped vault/stream state.
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
- Retirement procedure (Phase 9) is documented and requires no changes to core code; triggered
  when testnet E2E no longer needs the helper subprocess.

#### Non-regression guard

Before merging `feat/step18-public-testnet` to `master`:

- Run `make verify-step17` on `master` and on the branch. Both must pass with `CHAIN` unset
  and with `CHAIN=local`. Confirm no local-path script behavior changed when `CHAIN` is unset
  or `local`.
- Run `make verify-step18` on the branch against testnet once bootstrap succeeds. Confirm it
  does not start a local sequencer and does not call `make deploy` / `make setup` /
  `make full-reset-localnet`. Until bootstrap and verify pass, local `make verify-step17` remains
  the merge gate for guest correctness.
- Confirm `verify-step10a-dod.sh` and `verify-step10b-dod.sh` still assert `127.0.0.1:3040`
  and are not parameterized for testnet (they are the local-LEZ gate).
- Confirm operational LEZ pin is rc5 everywhere documented in
  [`feature-branch-pins.md`](../../reference/feature-branch-pins.md) ([N16](../../reference/integration-decisions.md#n16-step-18b-rc5-operational-pin-2026-06)).
- Confirm the helper is not compiled into the default `nix build` (separate `cargo build` under
  `tools/lez-testnet-submit` or flake output). The local path must not require the helper when
  `CHAIN=local`.
- Confirm the `submit_chain_action` selector's `CHAIN=local` branch is byte-for-byte the
  existing FFI call. Diff-review the dispatch point: the only local-path change is reading the
  env var.
- Confirm the local wallet binary still exposes `wallet_ffi_send_generic_public_transaction`,
  `wallet_ffi_program_deployment`, and `sign_public_payload` (used by `payment_streams_module`
  chain writes and Step 11d deploy). The helper must not alter the local FFI surface.

Not in scope: public internet Store provider (Step 23); replacing Step 17 local CI gate;
automatic testnet faucet unless the network ships a supported funding API; Basecamp UI
(Step 21, parked). Demo orchestration stays external scripts ([N17](../../reference/integration-decisions.md#n17-demo-orchestration-stays-external-script-2026-06); Step 25 won't fix).

Follow-on: Step 19 (LIP on-chain, done). Step 20 testnet journey rows use this step when
unblocked; local journey does not require Step 18. Optional Step 23 (hosted provider).
