# Step 11b — module chain writes and status

Wire `payment_streams_module` lifecycle writes to the patched Step 10b wallet (PR 19
`send_generic_public_transaction`).

Prerequisites: Step 10a fixture, Step 10b wallet plugin, load order wallet then payment
streams, wallet `open` with storage that includes the fixture owner (copy from
`.scaffold/wallet/storage.json` after seed — see E2E below).

Decisions: integration plan [N10](../reference/integration-decisions.md#n10-step-11b-module-writes-decisions).

Related: [Step 11a reads](archive/steps/module-chain-reads-runbook.md), [Step 10b wallet](archive/steps/wallet-runtime-runbook.md).

## Environment

| Variable | Purpose |
| --- | --- |
| `FIXTURE_MANIFEST` | Default `fixtures/localnet.json` |
| `PAYMENT_STREAMS_GUEST_BIN` | Guest ELF path (default 10a docker `.bin`); must be set on the logoscore daemon process |
| `REPO` | Repo root (fixture paths, guest ELF default) |
| `MODULES` | `lgpm` + `logoscore -m` install dir |
| `WALLET_SEED_STORAGE` | Optional; E2E copies this into the e2e wallet dir (default `.scaffold/wallet/storage.json`) |

When `PAYMENT_STREAMS_GUEST_BIN` is set, `payment_streams_module` omits the guest program ELF
from cross-module RPC (the blob is too large for logoscore IPC). The patched wallet loads the
ELF from this variable inside `send_generic_public_transaction` and attaches the authenticated
transfer dependency when none is supplied.

Cross-module submit uses `send_generic_public_transaction_json` on the wallet (single JSON
argument) because QList-shaped IPC from the Universal module to Legacy wallet is unreliable.

Wallet patches live under
`logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/`
(applied in the wrapper flake `postPatch`):

- `wallet-qt-guest-elf-from-env.patch` — `PAYMENT_STREAMS_GUEST_BIN`
- `wallet-qt-sign-public-payload.patch` — Step 11c
- `wallet-qt-send-generic-public-transaction-json.patch` — Step 11b JSON submit

Pin and rebuild details: [`archive/steps/wallet-510-runbook.md`](archive/steps/wallet-510-runbook.md),
[`feature-branch-pins.md`](feature-branch-pins.md).

Run logoscore from repo root so relative paths resolve.

## Public LogosAPI surface (Universal)

Exported methods come from `payment_streams_module_impl.h` via codegen ([N11](../reference/integration-decisions.md#n11-universal-module-public-api)).
Writes and status use one router instead of separate invokables per operation:

| Method | Purpose |
| --- | --- |
| `readVaultConfigDecoded` | Step 11a |
| `readVaultHoldingDecoded` | Step 11a |
| `readStreamConfigDecoded` | Step 11a |
| `readClockDecoded` | Step 11a |
| `readClock10Decoded` | Step 11a |
| `chainAction` | All writes and status queries (see below) |

There are no separate public `initializeVault`, `deposit`, or `getVaultStatus` invokables.
Use `logoscore call payment_streams_module chainAction <operation> '<json>'`.

`accountIdHexFromBase58` is not exported (call `logos_execution_zone.account_id_from_base58`
directly if needed).

### `chainAction(operation, paramsJson)`

`paramsJson` is a compact JSON object. Keys vary by operation:

| operation | JSON keys |
| --- | --- |
| `initializeVault` | `signer`, `vault_id` |
| `deposit` | `signer`, `vault_id`, `amount_lo`, `amount_hi` |
| `withdraw` | `signer`, `vault_id`, `amount_lo`, `amount_hi`, optional `withdraw_to` |
| `createStream` | `signer`, `vault_id`, `stream_id`, `provider`, `rate`, `allocation_lo`, `allocation_hi` |
| `pauseStream` / `resumeStream` | `signer`, `vault_id`, `stream_id` |
| `topUpStream` | `signer`, `vault_id`, `stream_id`, `increase_lo`, `increase_hi` |
| `closeStream` | `signer`, `vault_id`, `stream_id`, optional `authority` |
| `claim` | `owner`, `provider`, `vault_id`, `stream_id` |
| `getVaultStatus` | `owner`, `vault_id` |
| `getStreamStatus` | `owner`, `vault_id`, `stream_id` |

Write operations return submit-level JSON on success:
`{ "status":"ok", "success": true, "tx_hash": "…", "wallet": {…} }`.
The module does not wait for inclusion; callers sync via wallet `sync_to_block` and/or poll
status through `chainAction`.

Status helpers derive vault/stream PDAs from fixture `program_id_hex`, owner base58, and ids
(same program id as writes), then reuse wallet `get_account_public` and FFI decode/fold.

## E2E lifecycle

`./scripts/archive/step11b-logoscore-e2e.sh` (invoked from `./scripts/archive/verify-step11b-dod.sh`):

- Copies seeded storage from `WALLET_SEED_STORAGE` into `.scaffold/wallet-logoscore-e2e/`
- Uses manifest `owner_account_id` and `provider_account_id` (base58)
- Default `vault_id = 1`, `stream_id = 0` (demo vault `0` remains for Step 11a decode)
- Sets `PAYMENT_STREAMS_GUEST_BIN` on the daemon
- Calls `logos_execution_zone sync_to_block` when sequencer height is reachable, then sleeps
- Runs INIT → DEPOSIT → CREATE → PAUSE → RESUME → TOPUP → CLAIM via `chainAction`
- Polls `getVaultStatus` / `getStreamStatus` via `chainAction` with retries

Verify treats status as SKIP (not FAIL) when the chain returns `account data missing` for
derived PDAs after all submits succeeded; fixture vault `0` reads in Step 11a remain the
sanity check for `get_account_public` + decode.

## Definition of done

```bash
./scripts/archive/verify-step11b-dod.sh
```

Offline checks: `lm methods` on the PS plugin lists `chainAction`; wallet plugin strings
include `PAYMENT_STREAMS_GUEST_BIN` (use `rg -F` on the `.so`, not `strings | rg -q`).

Skip live chain:

```bash
VERIFY_LOGOSCORE=0 ./scripts/archive/verify-step11b-dod.sh
```
