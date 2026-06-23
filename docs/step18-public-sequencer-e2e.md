# Step 18 — public sequencer E2E (local Store and relay)

Operator runbook for `CHAIN=testnet`. Local dual-host layout matches
[step17-e2e-local.md](step17-e2e-local.md). CI default remains `make verify-step17`.

## Dual-pin policy

| Role | LEZ pin | Artifact |
| --- | --- | --- |
| Local E2E and testnet reads/sign | `62d9ba10` (510) | `logos_execution_zone` .lgx, `scaffold.toml`, payment-streams FFI |
| Testnet chain writes (chainAction) | `cf3639d8` (rc3) | `lez-testnet-submit`, `wallet deploy-program` for `deploy-testnet` |

`wallet check-health` with the 510 CLI against testnet is expected to fail (builtin program id
mismatch). Do not use it as a testnet gate. Run
`./scripts/verify-step18-testnet-read-smoke.sh` when the public RPC is up (PASS, not skip)
before Part B bootstrap.

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
| `LEZ_TESTNET_SUBMIT` | `lez-testnet-submit` on PATH | optional override to helper binary |
| `PAYMENT_STREAMS_GUEST_BIN` | guest ELF path | same; passed to helper when `program_elf_hex` is empty |

Build the helper (not part of default `nix build` at repo root):

```bash
cd tools/lez-testnet-submit && cargo build --release
# or: nix build ./tools/lez-testnet-submit
export PATH="$PWD/tools/lez-testnet-submit/target/release:$PATH"
```

## One-time bootstrap (Part B)

1. `make deploy-testnet` — rc3 `wallet deploy-program` against testnet; record `program_id_hex`
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
- Read smoke fails after RPC is up: dual-pin read path blocked; fix before chain writes
- Helper not found: build `lez-testnet-submit` and set `LEZ_TESTNET_SUBMIT` or PATH
- Stale `program_id_hex`: re-run deploy or align manifest with on-chain guest
- Faucet limits: retry Piñata claim; bootstrap funding is independent of deploy
