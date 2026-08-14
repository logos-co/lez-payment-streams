# Verification matrix (mode × network)

Canonical entry: [`verify/e2e.sh`](../../verify/e2e.sh).
The first argument (`local` or `testnet`) sets the network.
Each `local run` / `testnet run` performs prepare, run, and teardown unless `SKIP_TEARDOWN=1`.

`MODE` values: [naming-conventions.md](names.md).
Orchestrated recipes: [reproduce/store.md](../reproduce/store.md).
Manual protocol path: [reproduce/module.md](../reproduce/module.md).
Testing handles: [root README Testing](../../README.md#testing).

## Cold start (first time on a machine)

Optional one-time setup before the commands below.
`local run` calls prepare, which builds modules unless `SKIP_BUILD=1`.

1. Host: [Nix](https://nixos.org/download/) with flakes enabled.
   Rust + RISC Zero toolchain for the guest ELF (`cargo risczero build --manifest-path program/methods/guest/Cargo.toml`, or `make build`).
2. Logos scaffold CLI `lgs` on `PATH` (often `~/.cargo/bin` after installing `lgs`).
3. Run verification inside a shell that provides `logoscore` and `lgpm`:

```bash
nix shell --accept-flake-config \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-package-manager \
  --command bash
```

4. From the repo root (still in that shell, plus `lgs` on `PATH`):

```bash
lgs init      # if .scaffold/ is missing
lgs setup     # if scaffold.toml / layout is missing
```

5. Store integration: clone `logos-delivery-module` at `../logos-delivery-module` (or set `DELIVERY_MODULE_ROOT`) on the branch in [feature-branch-pins.md](pins.md).
   E2E does not clone it.
   Build fails if the directory is missing.
   A `../logos-delivery` sibling is optional (local `liblogosdelivery` overlay).
   Nix fetches the locked delivery input when building the module.
   `MODE=module` skips delivery checkouts.
6. First local run builds `.lgx` via Nix, starts localnet, and installs modules.
   Later runs can use `SKIP_BUILD=1` when `.scaffold/e2e/*/modules` are already populated.

`e2e.sh` sets `PAYMENT_STREAMS_GUEST_BIN` to the guest path under `program/methods/guest/target/...` when the file exists.
Build the guest before Store prepare if seed/fixture steps fail.

Recovery: [reference/localnet-recovery.md](localnet-recovery.md).

## Terminology

- `MODE=module` — module verification.
  Single-host path through `payment_streams_module` `chainAction` (vault, stream, claim).
- `MODE=store` (default) — Store integration verification.
  Dual-host paid Store query with LIP-155 eligibility proof.
  Orchestrator [`verify/store/run_e2e.py`](../../verify/store/run_e2e.py).

## The matrix

|  | Localnet (`./verify/e2e.sh local …`) | Testnet (`./verify/e2e.sh testnet …`) |
| --- | --- | --- |
| Module (`MODE=module`) | Required | Required |
| Store (`MODE=store`) | Required | Required |

## Support

Required: both modes on both networks.
Localnet needs no fixture.
Clone and verify on your machine.
Testnet needs `verify/fixtures/testnet.json` (one-time `make bootstrap-testnet`).
Module-only users can use `verify/fixtures/testnet-module.json` (one-time `make bootstrap-testnet-module`).
Store runs use a fresh vault per run.
Set `VAULT_ID` to pin a vault id, or `E2E_REUSE_BASELINE_VAULT=1` for the vault-0 lifecycle path.

Claim is required on both networks for the module.
Store claim is strict by default (`E2E_CLAIM_OPTIONAL=0`).
Artifact phase is `claim`.
Privacy testnet gates use `E2E_CLAIM_OPTIONAL=0` with real proving (`RISC0_DEV_MODE=0`).

## On-chain confirmation principle

`chainAction` submits are asynchronous.
Wallet success means the transaction was accepted locally.
When a later step reads state that a `chainAction` wrote, the harness confirms on-chain status directly.

Two checks:

- `wait_for_sequencer_tx` ([await_tx.py](../../verify/lib/await_tx.py)) polls sequencer `getTransaction` until the tx appears.
  Override the wall-clock budget with `E2E_TX_ONCHAIN_WAIT_S` (default 110s).
- A state poll then reads the account the next step depends on (`getVaultStatus`, `readStreamConfigDecoded`, `getAccount`, and similar) until the expected state is visible.

`await_chain_action_inclusion` always polls the sequencer on localnet.
Non-local chains where `getTransaction` lags may set `E2E_ALLOW_FIRE_AND_FORGET=1`.
The downstream state poll remains the gate.
Applies to `MODE=store` ([e2e/run_e2e.py](../../verify/store/run_e2e.py)) and `MODE=module` ([module-e2e.sh](../../verify/module-e2e.sh)).

Manual walkthrough: wait until `last_block` is past the submit height, then read status.
See [reproduce/module.md](../reproduce/module.md#on-chain-confirmation).

## ImageID skew is fail-fast

Testnet cells pin `PAYMENT_STREAMS_PROGRAM_ID_HEX` and `PAYMENT_STREAMS_GUEST_BIN` to the fixture ImageID and a gitignored `.scaffold/program-bins/lez_payment_streams-<id8>.bin` copy created by `make bootstrap-testnet` / `make bootstrap-testnet-module` (and `TESTNET_DEPLOY=1`).
A rebuilt guest whose `spel inspect` ImageID does not match the fixture dies in seconds at prepare/run.
Do not wait for `getTransaction` to stay null.

`PAYMENT_STREAMS_PROGRAM_ID_HEX` is unset on local cells so identity comes from the live ELF.
Neither that variable nor a testnet `FIXTURE_MANIFEST` may leak across cells.

Local snapshot save records the vault `program_owner` from the sequencer, not the ELF on disk.
`snapshot validate` compares that id to the live ledger and to the current guest; a new guest after snapshot fails closed into prefund.

## Commands

Per-cell prepare, bootstrap, verbosity, and expected artifacts: [reproduce/store.md](../reproduce/store.md).

Make aliases: `verify-module-local`, `verify-module-testnet`, `verify-module-local-provider-close`, `verify-module-local-provider-close-privacy`, `verify-module-local-close-negatives`, `verify-store-local`, `verify-store-testnet`.

Maintainer-only: `make verify-store-local-lifecycle` or [`verify/store/store-lifecycle.sh`](../../verify/store/store-lifecycle.sh).

## Notes

- Store local prepare restores a funded snapshot (identity + policy, no program vault) and writes `verify/fixtures/localnet.json` from owner/provider markers.
  The orchestrator scans for a fresh vault id and ensures it (init + deposit) before stream creation.
  `E2E_REUSE_BASELINE_VAULT=1` selects vault-0 reuse (`verify-store-local-lifecycle`).
- Module flow ensures localnet is up and skips `delivery_module` build.
- Module testnet uses `VAULT_ID` to pin a fresh vault.
- Artifacts: `.scaffold/e2e/artifacts/` JSON-lines logs.
  Layout: [naming-conventions.md#scaffold-layout](names.md#scaffold-layout).
  Module: `module-e2e-*.log` (`vault_init`, `deposit`, `create_stream`, `claim`, …).
  Store: `e2e-*.log` (`store_query_success`, `store_query_missing_proof`, `claim`, …).
  Privacy overlays and env flags: [reproduce/store.md](../reproduce/store.md#privacy-overlays).
  Privacy cells keep the same private owner/provider within one run and derive fresh
  private accounts between cells (reuse only the public funder). Burned ids live in
  `.scaffold/e2e/burned-private-ids.json`. After an ImageID cut, existing private notes
  are treated as burned so a later cell cannot shield a spent nullifier.
