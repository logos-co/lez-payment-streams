# Step 18 — public sequencer E2E (local Store and relay)

Operator runbook for `CHAIN=testnet`. Local dual-host layout matches
[step17-e2e-local.md](step17-e2e-local.md). CI default remains `make verify-step17`.

## LEZ pin (single operational)

Operational wallet, module `.lgx`, local E2E, and public testnet use one LEZ revision:

| Field | Value |
| --- | --- |
| Tag | `v0.2.0-rc5` |
| Git rev | `27360cb7d6ccb2bfbcca7d171bab8a3938490264` |
| Where | `scaffold.toml`, `nix/payment-streams-ffi.nix`, wallet flakes, `tools/lez-testnet-submit` |

Rust `program_tests` and the guest ELF share the operational rc5 LEZ pin ([N16](reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06), [Step 24b](plan/completed/step-24b-rc5-rust-lee-unify.md)).

Public testnet at `https://testnet.lez.logos.co/` uses lez jsonrpsee RPC (`getLastBlockId`,
`sendTransaction`, `getAccount`, …). LEE v0.3 public message hashing applies on chain writes.

Historical note: dual-pin (510 local + rc3 testnet writes) was Step 18 WIP; superseded by Step 18b (rc5 on `master`).

Manifest policy: committed `fixtures/testnet.json.example` (chain constants + org
`program_id_hex`); gitignored `fixtures/testnet.json` per operator after
`make bootstrap-testnet`. Per-operator owner/provider/vault/stream ids.

Run `./scripts/archive/verify-step18-testnet-read-smoke.sh` (PASS, not skip-only) before Part B bootstrap.
`wallet check-health` against testnet with rc5 CLI and unified wallet storage is a valid gate.

## Part A vs Part B

Part A (no live testnet required): helper, `CHAIN` selector, fixtures template, runbook, Makefile
wiring. Local `make verify-step17` must stay green.

Part B (public RPC required): read smoke PASS, `make bootstrap-testnet`, `make verify-step18`.
Org guest deploy is already on chain (see step plan Verified org deploy log); operators only
re-run `make deploy-testnet` when the guest ELF or ImageID changes.

Explorer: transaction `1787368626484789a2976a2aa8631d2b5b39c415c0a74b5a345474d1415f79b1`
(block 3284) — `https://explorer.testnet.lez.logos.co/transaction/1787368626484789a2976a2aa8631d2b5b39c415c0a74b5a345474d1415f79b1`

## Prerequisites (Part B)

- Internet egress to `https://testnet.lez.logos.co/`
- rc5 `wallet` CLI (via `lgs setup` or scaffold cache) and testnet wallet storage
- `fixtures/testnet.json` (from `make bootstrap-testnet`)
- `WALLET_CONFIG` / `WALLET_STORAGE` under `.scaffold/e2e/testnet-wallet/` for module and CLI

Create or import keys with rc5 `wallet` after `lgs setup`; retire split `LEZ_TESTNET_WALLET_*` env vars if present in old notes.

## Environment

| Variable | Default | Testnet |
| --- | --- | --- |
| `CHAIN` | `local` | `testnet` |
| `FIXTURE_MANIFEST` | `fixtures/localnet.json` | `fixtures/testnet.json` |

Copy `fixtures/testnet-wallet_config.example.json` to
`.scaffold/e2e/testnet-wallet/wallet_config.json` before Part B when needed.

| `LEZ_TESTNET_SUBMIT` | `lez-testnet-submit` on PATH | optional override to helper binary |
| `PAYMENT_STREAMS_GUEST_BIN` | guest ELF path | passed to helper when `program_elf_hex` is empty |
| `TESTNET_SKIP_PINATA` | unset | `1` reuses manifest owner; owner must have non-zero balance |

Build the helper (not part of default `nix build` at repo root):

```bash
cd tools/lez-testnet-submit && cargo build --release
export PATH="$PWD/target/release:$PATH"
```

## One-time bootstrap (Part B)

1. Optional: `make deploy-testnet` — rc5 `wallet deploy-program`; idempotent if org guest is deployed
2. `make bootstrap-testnet` — vault/stream via helper or module; writes `fixtures/testnet.json`

First-time funding: unset `TESTNET_SKIP_PINATA` or fund owner manually. Repeat runs may use
`TESTNET_SKIP_PINATA=1` with an funded owner.

## Repeatable demo (Part B)

Default gate: `make verify-step18` sets `E2E_PHASE=core` (paid Store + missing-proof path),
`PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF=0`, and runs an in-orchestrator testnet
`topUpStream` when unaccrued allocation is low. Optional standalone preflight:
`TESTNET_SKIP_PREFLIGHT_TOPUP=0 ./scripts/archive/testnet-preflight-topup.sh` (same top-up via ephemeral
logoscore; default skip because the dual-host orchestrator top-ups after daemons start).

### Depleted-stream bypass (demo only — remove when top-up is reliable)

`PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF=1` lets the user module mint proofs when unaccrued
is zero. It is **not** production semantics. Do not use for Step 20 journey docs unless explicitly
documented as a testnet repeatability escape hatch. Follow-up: rebootstrap or vault deposit when
`topUpStream` cannot increase unaccrued (fixture liquidity).

```bash
export CHAIN=testnet
export FIXTURE_MANIFEST=fixtures/testnet.json
export WALLET_CONFIG="$PWD/.scaffold/e2e/testnet-wallet/wallet_config.json"
export WALLET_STORAGE="$PWD/.scaffold/e2e/testnet-wallet/storage.json"
make verify-step18
```

Does not start local LEZ, `make deploy`, `make setup`, or `demo-localnet-fresh.sh`.

Stream depletion on testnet is real; the orchestrator may top up via `chainAction` (same as Step 17
late-stream path).

## Temporary helper

`tools/lez-testnet-submit submit-public-tx` accepts the same JSON as
`send_generic_public_transaction_json` and prints wallet-shaped stdout for the module.

Retirement (Phase 9): when `make verify-step18` passes with module `chainAction` on testnet without
the helper path, remove the helper and narrow `CHAIN=testnet` C++ dispatch.

## Persistence

Do not wipe testnet chain state between runs. Reset only local `PERSIST_USER` / `PERSIST_PROVIDER`
when debugging off-chain eligibility state.

After changing operational LEZ pin or wallet storage format, delete local `fixtures/testnet.json`
and re-run `make bootstrap-testnet`.

## Failure triage (Phase 3 order)

1. Read smoke + `wallet check-health` + owner balance on chain
2. `make bootstrap-testnet` → vault holding balance and stream config accounts
3. `prepareEligibility` / module chain reads against manifest ids
4. Dual-host Store path (relay, filter, paid Store) — same split as Step 17
5. Stream accrual / depletion — default `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF=0`; orchestrator
   top-up (`chainAction` / `scripts/archive/testnet-preflight-topup.sh`); rebootstrap if vault holding
   cannot fund top-up

Do not re-open signing unless Phase 1 testnet smoke regresses.

- Sequencer unreachable: fix network; read smoke fails when RPC is down (merge gate)
- Read smoke fails on CLOCK_10: align `clock_10_account_id` with `fixtures/testnet.json.example`
- Helper not found: build `lez-testnet-submit` and set `LEZ_TESTNET_SUBMIT` or PATH
- Stale `program_id_hex`: align manifest with org deploy on chain
- `TESTNET_SKIP_PINATA=1` with zero balance: fund owner or run Piñata once
