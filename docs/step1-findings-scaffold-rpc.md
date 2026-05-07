# Step 1 findings scaffold rpc

## Scope

This document records concrete findings from Step 1 discovery work
for scaffold localnet lifecycle and sequencer account reads.

## Environment and repos

Canonical scaffold repository is `https://github.com/logos-co/scaffold`.
Local directory name is not part of behavior.
Remote URL and pinned commit control behavior.

Discovery runs were performed from a dedicated workspace directory
separate from the `lez-payment-streams` repository.
This keeps scaffold operational state local to that run workspace.

## Sequencer lifecycle

The following command flow is sufficient to launch and verify localnet:

`lgs init`
`lgs setup`
`lgs localnet start`
`lgs localnet status`
`lgs wallet -- check-health`

Shutdown flow:

`lgs localnet stop`
`lgs localnet status`

Confirmed endpoint for sequencer RPC:
`http://127.0.0.1:3040`

## Input format contracts

Wallet CLI and JSON-RPC use different account-id input formats.

Wallet CLI expects account references with privacy prefix:
`Public/<base58-id>` or `Private/<base58-id>`.

Sequencer JSON-RPC `getAccount` expects raw base58 account id only:
`<base58-id>`.

Passing raw id to wallet `account get` fails with:
unsupported privacy kind.

Passing prefixed id to JSON-RPC is invalid for `getAccount`.

## Public and private account visibility

Observed behavior in this environment:

`wallet account get` on a private account reference
returns initialized private account state.

`getAccount` with the corresponding raw private id
returns a zeroed default-like account object.

`getAccount` on an initialized public account id
returns populated state consistent with wallet semantics.

This is treated as a visibility difference between wallet private context
and public sequencer account reads.

## Public account initialization requirement

Preconfigured public accounts may be uninitialized.
For meaningful `getAccount` validation, initialize and fund first:

`lgs wallet topup --address Public/<base58-id>`

Scaffold performs preflight and, when needed, runs:

`wallet auth-transfer init`
then `wallet pinata claim`.

After topup, `wallet account get` and `getAccount` return consistent values
for balance and nonce on the same public account id.

## Account read response shape

`getAccount` response shape confirmed:

- `program_owner` as `[u32; 8]`
- `balance` as integer
- `data` as byte array-like field
- `nonce` as integer

Wallet output encodes `program_owner` as base58 text.
RPC output encodes the same value as integer array words.

## Known working example

Working public account reference:
`Public/8UUCxCrkZAiP8A6g6rQAVMmk6bVxfurKqYi8aFxfEZqf`

Derived raw id used in RPC:
`8UUCxCrkZAiP8A6g6rQAVMmk6bVxfurKqYi8aFxfEZqf`

Observed post-topup state:

- balance `150`
- nonce `1`
- account owned by authenticated transfer program

## Deploy verification

Program deployment succeeded from `lez-payment-streams` via scaffold.

Observed deploy JSON:

- program: `lez_payment_streams`
- program_id: `0b9349a24ceccf031fd2e06af23722e086dd2a8fec388e4d179619045ffb377d`
- status: `submitted`

## Idl verification

IDL generation command succeeded:

`cargo run -p lez_payment_streams-examples --bin generate_idl`

Generated IDL contains top-level account types:

- `VaultConfig`
- `VaultHolding`
- `StreamConfig`

## Pda derivation results

Derived in current environment:

- vault_config PDA: `5iyXRxdaXH9xZHbNvPG3ZjxqQgtY5nebRX6xPxnNxu8i`
- vault_holding PDA: `6CXxQd4HxPu8KFqE1AHq5PXkVqRRgzd4KFpsBnmyWxJ2`

Derived stream_config PDA with only arg seed path:

- `AprUVPgJAmRkcuEew34n4hg1bD1zSbxbvMmjrTAyQUEf`

Attempt to provide explicit account-seed input for stream config:

`pda stream_config --vault-config-account <vault_config_id> --seed-arg <stream_id>`

fails with:

`Seed '<base58>' is 44 bytes, max 32`

This indicates a current SPEL CLI limitation for account-seeded PDA derivation
in this pinned version.

## Vault and clock account snapshots

Vault config snapshot:

- program_owner all zeros
- balance `0`
- data length `0`
- nonce `0`

Vault holding snapshot:

- program_owner all zeros
- balance `0`
- data length `0`
- nonce `0`

These default values are expected before running `initialize_vault`.

Clock account selected for checks:

- `CLOCK_10` id:
  `4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWSs`

Clock snapshot:

- non-zero program_owner
- balance `0`
- data length `16`
- nonce `0`

## Inspect limitations

`spel inspect` network mode failed in this environment due to wallet storage
compatibility and root initialization expectations.

Observed failures:

- default wallet path parse error in `~/.nssa/wallet/storage.json`
- project wallet path root setup error under `.scaffold/state/wallet`

Working fallback for Step 1:

- fetch account bytes via `getAccount`
- decode with `spel inspect ... --data <hex>`

For uninitialized vault accounts, `data` is empty and decode is not meaningful.

## Doctor summary

Latest `lgs doctor` summary:

- PASS `21`
- WARN `1`
- FAIL `0`

Remaining warning:

- `repo lez` working tree dirty

Pin alignment remains correct for LEZ and SPEL.
Sequencer reachability and wallet health checks pass.

## Deferred item

Clean rerun in a clean workspace is deferred.
Current findings are valid with the explicit dirty-tree caveat above.

## Sequencer stop and optional cleanup

Clean stop from the workspace that started localnet:

`lgs localnet stop`
`lgs localnet status`

No mandatory cleanup is required after a normal run.
Keeping `.scaffold/state/` and `.scaffold/logs/` is expected.

If a fresh workspace state is needed:

- stop localnet first
- remove `.scaffold/state/`
- keep `.scaffold/logs/` if diagnostics should be preserved

This performs a full local workspace state reset for subsequent runs.
