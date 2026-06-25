# Step 18 — public sequencer E2E (local Store and relay)

Operator runbook for `CHAIN=testnet`. Local dual-host layout matches
[step17-e2e-local.md](step17-e2e-local.md). CI default remains `make verify-step17`.

## Dual-pin policy

| Role | LEZ pin | Artifact |
| --- | --- | --- |
| Local E2E and testnet reads/sign | `62d9ba10` (510) | `logos_execution_zone` .lgx, `scaffold.toml`, payment-streams FFI |
| Testnet chain writes (chainAction) | `cf3639d8` (rc3) | `lez-testnet-submit`, `deploy-testnet` via helper |

Public testnet sequencer RPC (2026-06): snake_case legacy methods (`get_last_block`, `send_tx`,
`get_account`). Local LEZ and pin-510 `logos_execution_zone` use jsonrpsee camelCase
(`getLastBlockId`, `sendTransaction`). `lez-testnet-submit` talks legacy RPC for deploy, submits,
and bootstrap reads. Full testnet E2E still opens the 510 wallet module for eligibility chain reads;
that path fails until testnet exposes jsonrpsee again or `logos_execution_zone` gains a legacy
sequencer client (track with infra).

Manifest policy: committed `fixtures/testnet.json.example` (chain constants + template); gitignored
`fixtures/testnet.json` per operator after `make bootstrap-testnet`. Shared on-chain guest
`program_id_hex`; per-operator owner/provider/vault/stream ids.

`wallet check-health` with the 510 CLI against testnet is expected to fail (builtin program id
mismatch). Do not use it as a testnet gate. Run
`./scripts/verify-step18-testnet-read-smoke.sh` when the public RPC is up (PASS on legacy checks,
not skip) before Part B bootstrap.

## Part A vs Part B

Part A (no live testnet required): helper, `CHAIN` selector, fixtures template, runbook, Makefile
wiring. Local `make verify-step17` must stay green.

Part B (public RPC required): read smoke PASS, `make deploy-testnet`, `make bootstrap-testnet`,
`make verify-step18`.

## Prerequisites (Part B)

- Internet egress to `https://testnet.lez.logos.co/`
- Testnet wallet storage with funded accounts (rc3 `wallet` CLI + Piñata or faucet)
- `fixtures/testnet.json` (copy from `fixtures/testnet.json.example` after bootstrap)
- `WALLET_CONFIG` / `WALLET_STORAGE` pointing at the same dirs for 510 module open and rc3 helper

Create or import keys with the rc3 wallet first if 510-created storage cannot be opened by rc3
(document outcome in your operator notes).

## Environment

| Variable | Default | Testnet |
| --- | --- | --- |
| `CHAIN` | `local` | `testnet` |
| `FIXTURE_MANIFEST` | `fixtures/localnet.json` | `fixtures/testnet.json` |
Copy `fixtures/testnet-wallet_config.example.json` to
`.scaffold/e2e/testnet-wallet/wallet_config.json` (and import or copy wallet storage) before
Part B. Template also under `.scaffold/e2e/testnet-wallet/wallet_config.json` when created locally.
| `LEZ_TESTNET_WALLET_CONFIG` / `LEZ_TESTNET_WALLET_STORAGE` | rc3 wallet paths for helper submits | required when 510 storage differs from rc3 |
| `LEZ_TESTNET_SUBMIT` | `lez-testnet-submit` on PATH | optional override to helper binary |
| `PAYMENT_STREAMS_GUEST_BIN` | guest ELF path | same; passed to helper when `program_elf_hex` is empty |

Build the helper (not part of default `nix build` at repo root):

```bash
cd tools/lez-testnet-submit && cargo build --release
# or: nix build ./tools/lez-testnet-submit
export PATH="$PWD/tools/lez-testnet-submit/target/release:$PATH"
```

## One-time bootstrap (Part B)

1. `make deploy-testnet` — `lez-testnet-submit deploy-program` (legacy `send_tx`); idempotent if
   the guest is already deployed; record `program_id_hex` in your gitignored manifest
2. `make bootstrap-testnet` — vault/stream via `lez-testnet-submit`; writes `fixtures/testnet.json`

## Repeatable demo (Part B)

```bash
export CHAIN=testnet
export FIXTURE_MANIFEST=fixtures/testnet.json
# WALLET_CONFIG / WALLET_STORAGE / PATH to helper
make verify-step18
```

Does not start local LEZ, `make deploy`, `make setup`, or `demo-localnet-fresh.sh`.

Stream depletion on testnet is real; the orchestrator may top up via `chainAction` (same as Step 17
late-stream path).

## Temporary helper

`tools/lez-testnet-submit submit-public-tx` accepts the same JSON as
`send_generic_public_transaction_json` and prints wallet-shaped stdout for the module.

Retirement: when public testnet runs LEZ containing PR #491 and #510 and `check-health` passes
with pin `62d9ba10`, remove the helper and the `CHAIN=testnet` branch (Phase 9 in the step plan).

## Persistence

Do not wipe testnet chain state between runs. Reset only local `PERSIST_USER` / `PERSIST_PROVIDER`
when debugging off-chain eligibility state.

## Failure triage

- Sequencer unreachable: fix network or wait; read smoke skips with exit 0 when RPC is down
- `getLastBlockId` / METHOD_NOT_FOUND: testnet expects `get_last_block`; use `scripts/testnet_rpc.py`
- Read smoke fails on CLOCK_10: update `clock_10_account_id` in manifest from testnet recon
- Full E2E fails at wallet open/sync on testnet: 510 module jsonrpsee vs legacy RPC (see Dual-pin)
- Helper not found: build `lez-testnet-submit` and set `LEZ_TESTNET_SUBMIT` or PATH
- Stale `program_id_hex`: align manifest with `make program-id` after deploy
- Deploy `Transaction too large`: guest ELF vs testnet tx size policy (target 600 KiB class limit)
- Faucet limits: retry Piñata claim; bootstrap funding is independent of deploy
