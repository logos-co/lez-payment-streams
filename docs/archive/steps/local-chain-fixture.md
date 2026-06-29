# Step 10a — local chain fixture

Reproducible localnet with deployed `lez_payment_streams`, a funded public owner,
and on-chain demo vault `vault_id = 0` plus stream `stream_id = 0`.
PDAs in the manifest match `lez-payment-streams-core` / FFI derivation (not SPEL CLI PDA helpers).

Decisions: integration plan [N9](../reference/integration-decisions.md#n9-step-10a-local-chain-fixture-decisions).
Progress and sequencer follow-up: [`archive/steps/local-chain-fixture-handoff.md`](archive/steps/local-chain-fixture-handoff.md).
Scaffold RPC detail: [`archive/steps/scaffold-rpc-findings.md`](archive/steps/scaffold-rpc-findings.md).

## Prerequisites

- `lgs` and `wallet` on PATH after `lgs setup` in this repo (LEZ pin in `scaffold.toml` aligned with PR 491).
- RISC0 guest toolchain (`make build`).
- Gitignored runtime: `.scaffold/` (sequencer + wallet state). Committed: `scaffold.toml`, `spel.toml`.
- Export `LEE_WALLET_HOME_DIR` to the wallet directory (contains `wallet_config.json` and `storage.json`).
  The seed script sets it to `.scaffold/wallet`. Legacy `NSSA_WALLET_HOME_DIR` is copied into `LEE_WALLET_HOME_DIR` for the Rust seed binary only.

### Wallet storage after a LEZ pin bump

491 wallet uses encrypted `storage.json` (not the pre-491 NSSA layout). Use `.scaffold/wallet` only for Step 10a (`scaffold.toml` `[wallet] home_dir`).

Re-init (backs up old storage, creates 491 storage, clears fixture owner state):

```bash
chmod +x scripts/archive/reinit-scaffold-wallet.sh
./scripts/archive/reinit-scaffold-wallet.sh
```

Optional: set `SCAFFOLD_WALLET_SETUP_PASSWORD` before running (default is a local-dev placeholder). Save the recovery phrase printed on first init if you change the password flow manually.

Do not commit `storage.json`.

## One-shot seed

From repo root:

```bash
chmod +x scripts/seed-localnet-fixture.sh
./scripts/seed-localnet-fixture.sh
```

The script:

1. Ensures localnet is running (`lgs localnet start`).
2. Runs `make build`, `make idl`, `make deploy`.
3. Creates or reuses owner via `make setup` (`.lez_payment_streams-state`).
4. Funds owner with `lgs wallet topup`.
5. Submits `initialize_vault`, `deposit`, and `create_stream` via `seed_localnet_fixture`
   (core `Instruction` encoding — SPEL CLI currently fails `VaultId` IDL serialization).
6. Writes `fixtures/localnet.json` (gitignored).

Step 17b (repeat runs): [`step-17b-localnet-snapshot-restore.md`](plan/completed/step-17b-localnet-snapshot-restore.md)
splits fund vs stream — `prefund-onchain`, `create-stream-onchain` (stream params only), and
`seed-onchain` on `seed_localnet_fixture`. Operators use `make prepare-localnet`
(vault-only restore) plus per-run create in E2E or verify scripts, or `make full-reset-localnet` to rebuild
`.scaffold/snapshots/funded/`.

Idempotent resume: if vault `0` exists but stream `0` does not, re-run the seed script without
`SEED_FORCE` (deposit + `create_stream` only).

`SEED_FORCE=1` disables skip-if-initialized and retries including `initialize_vault`; that
fails when vault `0` already exists. Use a chain reset (below) instead of force after a partial run.

## Manual steps (equivalent)

```bash
export LEE_WALLET_HOME_DIR="$PWD/.scaffold/wallet"
lgs init && lgs setup && lgs localnet start
make build idl deploy setup
lgs wallet topup --address "Public/$(grep SIGNER_ID= .lez_payment_streams-state | cut -d= -f2)"
cargo run --bin seed_localnet_fixture -- seed-onchain \
  --program-bin methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin \
  --owner "$(grep SIGNER_ID= .lez_payment_streams-state | cut -d= -f2)" \
  --provider "<provider-base58>"
```

Manifest only (PDAs from program binary + ids):

```bash
cargo run --bin seed_localnet_fixture -- write-manifest \
  --program-bin methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin \
  --owner "<owner-base58>" --provider "<provider-base58>" \
  --output fixtures/localnet.json
```

## Verify definition of done

1. `curl -s http://127.0.0.1:3040` or `lgs localnet status` — sequencer up.
2. `lgs wallet -- check-health` — wallet + programs OK.
3. `make program-id` matches `program_id_hex` in `fixtures/localnet.json`.
4. JSON-RPC `getAccount` on manifest vault config, vault holding, and stream config PDAs
   returns non-empty `data` (see step1 findings for raw base58 id format).
5. clock id in manifest (demo now uses Clock01): `4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWNU`.

## Troubleshooting

### Owner and signing key (mixed wallet home or stale state)

The one-shot seed script does not require picking `--owner` by hand. It uses the same
`LEE_WALLET_HOME_DIR` (`.scaffold/wallet`) for `make setup`, topup, and `seed_localnet_fixture`.
`make setup` creates a public account in that wallet and records `SIGNER_ID` in
`.lez_payment_streams-state`; topup and on-chain seed use that id.

You only need to align owner manually when something is out of sync, for example:

- deploy or seed used a different `LEE_WALLET_HOME_DIR` (such as `~/.lee/wallet`) than setup/topup;
- `.lez_payment_streams-state` still lists a `SIGNER_ID` from an old wallet before re-init;
- you run `seed-onchain` yourself with `--owner` that is not in `wallet account list`.

Fix: run `./scripts/archive/reinit-scaffold-wallet.sh` (clears fixture owner state), or remove
`.lez_payment_streams-state` and re-run the seed script. For manual `seed-onchain`, set
`--owner` to a base58 id from `wallet account list` under the same `LEE_WALLET_HOME_DIR`
you use for signing.

### Localnet ownership (foreign sequencer on 3040)

This is required only when the listener on `127.0.0.1:3040` is not the sequencer scaffold
expects for this repo. `lgs localnet status` may show `ownership: foreign` or
`ready: false`. `wallet check-health` may panic or fail with authenticated-transfer or
other builtin program id mismatches. Deploy or seed may then fail (for example
`InvalidSignature`) even with a valid `.scaffold/wallet`.

If that happens: stop the process using port 3040, then from this repo run
`lgs localnet stop` (if applicable), `lgs localnet start`, and re-run the seed script.
Goal is a scaffold-tracked localnet on the LEZ pin in `scaffold.toml`.

If `lgs localnet status` shows this workspace owns the sequencer, check-health passes,
and `./scripts/seed-localnet-fixture.sh` completes with `fixtures/localnet.json`, no
localnet change is needed.

### lgs localnet start missing sequencer config path

If `lgs localnet start` fails to read
`.../repos/lez/<pin>/sequencer/service/configs/debug/sequencer_config.json`, the LEZ 491
tree places that file under `lez/sequencer/...`. Symlink once (example pin):

```bash
LEZ_ROOT="$HOME/.cache/logos-scaffold/repos/lez/a999563a2d27325ecada318746f1a0dc083d187f"
mkdir -p "$LEZ_ROOT/sequencer/service/configs/debug"
ln -sfn "$LEZ_ROOT/lez/sequencer/service/configs/debug/sequencer_config.json" \
  "$LEZ_ROOT/sequencer/service/configs/debug/sequencer_config.json"
```

Then re-run `lgs localnet start`.

### NSSA vs LEE public PDA prefix

LEZ 491 validates public PDAs with `/LEE/v0.2/AccountId/PDA/`. Host fixture derivation uses
`lee_core`; the guest uses vendored [`vendor/spel-framework-core`](../vendor/spel-framework-core)
(patched `compute_pda`). Rebuild the guest after changing that patch (`make build`). Upstream
SPEL-on-LEE cleanup: integration plan N9.

### Deposit or seed failures on 491

Troubleshooting (enum encoding, pinata balance vs demo deposit, poller vs execution): see
[`archive/steps/local-chain-fixture-handoff.md`](archive/steps/local-chain-fixture-handoff.md) (When verify fails).

### Stale `fixtures/localnet.json`

After `make build`, ImageID and all PDAs change. DoD item 2 fails if the manifest was written
for an older binary. Delete `fixtures/localnet.json` and complete a full seed, or run
`write-manifest` with the current `.bin` only after confirming on-chain state.

Verify DoD locally:

```bash
chmod +x scripts/archive/verify-step10a-dod.sh
./scripts/archive/verify-step10a-dod.sh
```

## Seed binary and workspace `Cargo.lock`

`seed_localnet_fixture` links LEZ 491 crates `wallet`, `lee`, and `common` as libraries
(signing, public tx construction, RPC submit/confirm). Payment streams does not use AMM, token,
or other builtin program logic; those crates appear in the root `Cargo.lock` because LEZ declares
them as dependencies of the monolithic `wallet` / `lee` client libraries, not because Step 10a
needs them.

The workspace also resolves two LEZ git identities (`nssa` tag `v0.1.2` for core/tests and
491 `rev` for the seed), which duplicates much of the LEZ graph in one lockfile.

Acceptable for Step 10a; optional later improvements (after fixture DoD is green):

- Shell out to the `wallet` CLI for submit/confirm where the CLI exposes enough control, and
  keep Rust for manifest PDA derivation and core `Instruction` encoding only.
- Depend on a minimal RPC + signing surface (`sequencer_service_rpc`, `lee_core`, …) instead of
  full `wallet`, if LEZ splits or documents a thinner operator API.
- Converge the repo on one LEZ `rev` when core/tests migrate off the `v0.1.2` tag, to drop
  duplicate LEZ entries in `Cargo.lock`.

## Persist vs reset

- Ledger data lives under `~/.cache/logos-scaffold/repos/lez/<scaffold.toml pin>/rocksdb/`
  (stop the sequencer before copying). Repo-local `.scaffold/state/` tracks scaffold bookkeeping.
- Step 17b: snapshot/restore the funded baseline (vault without stream) — see
  [step-17b-localnet-snapshot-restore.md](plan/completed/step-17b-localnet-snapshot-restore.md)
  and [archive/operator/localnet-recovery.md](archive/operator/localnet-recovery.md).
- Stop localnet without deleting `.scaffold/state/` to keep chain data between ad-hoc sessions.
- Full reset (pinata + prefund + snapshot): `make full-reset-localnet`. Legacy wipe of `.scaffold/state/` alone does not reset
  the LEZ RocksDB ledger.

## Step 11b note

Demo vault `0` is for Step 11a decode tests. Module-driven lifecycle in 11b should use
`vault_id = 1` or a reset chain (see `reserved_for_step_11b` in the manifest).

## Step 10b (wallet in logoscore)

After `./scripts/archive/verify-step10a-dod.sh` exits 0, install the patched wallet `.lgx` and run
[`archive/steps/wallet-runtime-runbook.md`](archive/steps/wallet-runtime-runbook.md) / `./scripts/archive/verify-step10b-dod.sh`.
