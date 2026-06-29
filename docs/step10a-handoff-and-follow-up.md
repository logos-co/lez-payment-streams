# Step 10a — progress handoff and follow-up

Status: fixture tooling and LEZ guest alignment are in tree. Step 10a DoD is green when
`./scripts/archive/verify-step10a-dod.sh` exits 0 after a full seed on the pinned LEZ localnet (see
[`step10a-local-chain-fixture.md`](step10a-local-chain-fixture.md)). Current LEZ pin: `scaffold.toml`
(matches [PR 510](https://github.com/logos-blockchain/logos-execution-zone/pull/510) merge after Step 11d).

## What is in tree

- Operator path: `scaffold.toml`, `spel.toml`, `scripts/seed-localnet-fixture.sh`,
  `scripts/archive/verify-step10a-dod.sh`, `scripts/archive/reinit-scaffold-wallet.sh`,
  `examples/src/bin/seed_localnet_fixture.rs`, `fixtures/localnet.json.example`.
- Runbook: [`step10a-local-chain-fixture.md`](step10a-local-chain-fixture.md).
- Public PDA prefix (LEE vs NSSA): vendored
  [`vendor/spel-framework-core`](../vendor/spel-framework-core) (`lee_core::AccountId::for_public_pda`
  in `compute_pda`); guest `[patch]` in root and `methods/guest/Cargo.toml`.
- Deposit chained call: guest serializes LEZ
  `authenticated_transfer_core::Instruction::Transfer { amount }` (not NSSA bare `u128`).
- In-process program tests: NSSA [`V03State`] harness tests are `#[ignore]` while the guest
  targets the pinned LEZ tree; other `cargo test -p lez-payment-streams-core --lib` tests still run.

Long-term cleanup when SPEL targets LEE: integration plan
[N9 SPEL-on-LEE cleanup](../reference/decisions-and-notes.md#n9-step-10a-local-chain-fixture-decisions).

## When verify fails

| DoD check | Typical cause |
| --- | --- |
| Program id in manifest | Stale `fixtures/localnet.json` after `make build` (new ImageID) |
| Vault / holding / stream PDAs empty | Partial seed, failed tx, or manifest PDAs from an old binary |

### Sequencer / execution

Search `.scaffold/logs/sequencer.log` for the failing tx hash from seed stdout.

- `MismatchedPdaClaim` on `initialize_vault`: guest not rebuilt after PDA vendor change, or
  wrong program binary deployed.
- `invalid value: integer N, expected variant index` on deposit: old guest (bare `u128` chained
  call); rebuild and redeploy.
- `Sender has insufficient balance` (authenticated_transfer): increase pinata rounds or lower
  `SEED_DEPOSIT_AMOUNT` / `SEED_STREAM_ALLOCATION` in seed/prefund scripts (defaults deposit
  `1000`, allocation `200`, rate `1`). `SEED_WALLET_TOPUP_ROUNDS * 150` must cover
  `SEED_DEPOSIT_AMOUNT`.
- `Transaction not found in preconfigured amount of blocks`: tx often never included (check log
  for `ProgramExecutionFailed` / skip); poller timeout is not proof the guest encoding is wrong.

### Operator state

- After every guest rebuild: redeploy, delete or regenerate `fixtures/localnet.json`, re-seed.
- Partial seed: vault `0` exists, stream `0` does not — re-run seed without `SEED_FORCE`.
- `SEED_FORCE=1`: retries `initialize_vault` and fails if vault `0` already exists; prefer reset
  or partial resume.

### Clean reset + re-verify

Preferred one-shot entry for demos:

```bash
make full-reset-localnet
```

Wallet storage parse errors or deploy failures:

```bash
REINIT_WALLET=1 make full-reset-localnet
```

Manual equivalent:

```bash
lgs localnet stop
rm -rf .scaffold/state/
rm -f fixtures/localnet.json .lez_payment_streams-state .lez_payment_streams-fixture-provider

export LEE_WALLET_HOME_DIR="$PWD/.scaffold/wallet"
lgs localnet start   # or ./scripts/seed-localnet-fixture.sh from repo root
make build idl deploy
./scripts/seed-localnet-fixture.sh
./scripts/archive/verify-step10a-dod.sh
```

See also [`demo-localnet-recovery.md`](demo-localnet-recovery.md).

Foreign localnet on `:3040`, wallet home drift, and sequencer config symlink: runbook
[Troubleshooting](step10a-local-chain-fixture.md#troubleshooting).

## Version bumps do not drop the patches

Published SPEL / `nssa_core` tags remain NSSA-prefix PDAs and NSSA guest conventions; the host
and 491 localnet pin (`a999563…`) use LEE. Do not remove `vendor/spel-framework-core` or the
deposit enum encoding on a dependency bump alone — see N9 SPEL-on-LEE cleanup in the integration
plan.

## Next step

After `./scripts/archive/verify-step10a-dod.sh` exits 0, follow [`step10b-wallet-runtime.md`](step10b-wallet-runtime.md)
and run `./scripts/archive/verify-step10b-dod.sh`. When Step 10b DoD is green, proceed to integration plan
Step 11a. Do not commit `fixtures/localnet.json` (gitignored).
