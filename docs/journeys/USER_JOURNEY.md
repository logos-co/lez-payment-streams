# User Journey: Operate Payment Streams via Logos Core Module

## What the user achieves

A user creates and manages payment streams on the Logos Execution Zone,
enabling continuous payments to service providers with per-second granularity.

## Why it matters

Payment streams replace discrete invoices with continuous value flow,
enabling real-time compensation for ongoing services.
This journey demonstrates direct operation of the LIP-155 protocol through the Logos Core module interface on a single host (`MODE=module`).
It does not exercise Logos Delivery Store or eligibility proofs on queries.

## Key components

* `payment_streams_module` — Universal Logos Core module exposing LIP-155 vault and stream lifecycle
* `logos_execution_zone` — Wallet module for chain interaction and transaction signing
* `lez-payment-streams` — On-chain SPEL guest program implementing LIP-155 semantics
* `logoscore` — Logos Core CLI for module loading and method invocation

## Repository

https://github.com/logos-co/lez-payment-streams

## Runtime target

The same entrypoint runs on localnet and on public TestNet v0.2: `scripts/e2e.sh` with `MODE=module` and `CHAIN=local` or `CHAIN=testnet` (implemented by `scripts/module-e2e.sh`).
TestNet v0.2 is the default and primary verification target (`make verify-module-testnet`).
Localnet (`make verify-module-local`) is single-host and faster for iteration.

## Prerequisites

Verification setup: lez-payment-streams repository README (https://github.com/logos-co/lez-payment-streams#prerequisites).

## Commands and expected outputs

Lifecycle: vault init, deposit, create stream, optional top-up, accrual, close stream, claim.
`scripts/module-e2e.sh` runs the phases; `scripts/e2e.sh` dispatches by `MODE=module` and `CHAIN`.

### Run the end-to-end demo

```bash
make verify-module-local
```

Equivalent:

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

Optional top-up phase (`topUpStream`, longer run):

```bash
MODULE_E2E_TOPUP=1 MODE=module CHAIN=local ./scripts/e2e.sh local run
```

Narrative console output and a JSON-lines artifact at `.scaffold/e2e/artifacts/module-e2e-*.log`.
Verbosity defaults to `verbose` on a TTY and `quiet` when piped; override with `./scripts/e2e.sh --verbosity quiet|normal|verbose`.

### Testnet run

The repo pins the public testnet payment-streams program in `fixtures/testnet-module.json` (`program_id_hex`, `sequencer_url`, and shared chain fields).
E2E loads the manifest via `FIXTURE_MANIFEST`; you do not deploy the guest as part of this journey.

One-time wallet and operator fixture setup:

```bash
make bootstrap-testnet-module
```

That creates or refreshes `fixtures/testnet-module.json` with your testnet owner and provider ids while keeping the shared `program_id_hex`.
Keys live under `.scaffold/e2e/testnet-wallet/`.

```bash
make verify-module-testnet
```

Or explicitly (use an unused `VAULT_ID` on repeat runs):

```bash
VAULT_ID=5 STREAM_ID=0 MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run
```

### Expected output

Exit code 0.
Console narrative ends with `E2E COMPLETE: All phases succeeded`.
Default scenario (no top-up):

```jsonl
{"phase":"wallet_open","ok":true}
{"phase":"auth_init_owner","ok":true,"extra":{"account_id":"…","already_initialized":true,"via":"on_chain","verify":"image_id"}}
{"phase":"auth_init_provider","ok":true,"extra":{"account_id":"…","already_initialized":false,"via":"wallet_cli","verify":"image_id","tx_hash":"…"}}
{"phase":"vault_init","ok":true}
{"phase":"deposit","ok":true}
{"phase":"deposit_balance","ok":true}
{"phase":"create_stream","ok":true}
{"phase":"accrual","ok":true}
{"phase":"close_stream","ok":true}
{"phase":"close_state","ok":true}
{"phase":"claim","ok":true}
{"phase":"claim_balance","ok":true}
{"phase":"module_e2e_complete","ok":true}
```

With `MODULE_E2E_TOPUP=1`, expect `topup_stream` and `topup_allocation` after `create_stream`.

## Expected result

Exit code 0.
Every phase in `.scaffold/e2e/artifacts/module-e2e-*.log` reports `"ok":true`, including verification lines `deposit_balance`, `accrual`, `claim_balance`, `close_state`, and `module_e2e_complete`.
With `MODULE_E2E_TOPUP=1`, also `topup_stream` and `topup_allocation`.

## Configuration details

### Environment variables

* `PAYMENT_STREAMS_GUEST_BIN`: Path to compiled guest ELF
* `VAULT_ID`: Vault identifier (default: 0)
* `STREAM_ID`: Stream identifier (default: 0)
* `DEPOSIT`: Initial deposit amount (default: 500 local, 30 testnet)
* `RATE`: Stream accrual rate per second (default: 1)
* `ALLOCATION`: Amount allocated to stream (default: 400 local, 20 testnet)
* `MIN_ACCRUED`: Tokens that must accrue before claim (default: `RATE * 3` local, 1 testnet)
* `TOPUP_INCREASE`: Tokens added during the top-up phase when enabled (default: 1)
* `MODULE_E2E_TOPUP`: Set to `1` to run `topUpStream` between create and accrual (default `0`)
* `MODULE_E2E_SKIP_CLOSE`: Set to `1` to skip close and claim (shorter testnet run; use a new `STREAM_ID` on the next run)
* `LEE_WALLET_HOME_DIR`: Set by the script — local `.scaffold/module-e2e-wallet`, testnet `.scaffold/e2e/testnet-wallet`
* `INCLUSION_ATTEMPTS`, `INCLUSION_SLEEP`: Poll budget for `getTransaction` after each write
* `ACCRUAL_ATTEMPTS`, `ACCRUAL_POLL_SLEEP`: Poll budget for `getStreamStatus` accrual
* `FIXTURE_MANIFEST`: Testnet fixture path (default `fixtures/testnet-module.json`; carries `program_id_hex` and `sequencer_url`)

### Verbosity

Console output level via `./scripts/e2e.sh --verbosity quiet|normal|verbose` or `E2E_VERBOSITY`:

* `quiet` — JSON-lines artifact only (typical when piped)
* `normal` — phase headers, status markers, on-chain values
* `verbose` — adds concept explanations (TTY default)

Console markers (normal and verbose): `→` upcoming, `✓` success, `✗` failure, `!` hint.

### Testnet timing

Wall clock on testnet is mostly serial transaction inclusion (default poll budget 45×2s per write).
Plan for several minutes on a slow public sequencer.

### Demo assumptions

Single `logoscore` loads `logos_execution_zone` and `payment_streams_module`.
Owner and provider keys live in one wallet for this demo.
After each write the script polls on-chain state and records `*_balance` / `*_state` lines in the artifact.

Verification line meanings:

* `deposit_balance` — vault holding matches deposit
* `topup_allocation` — allocation increased by top-up amount (`MODULE_E2E_TOPUP=1`)
* `accrual` — `accrued_lo` above minimum derived from rate
* `claim_balance` — provider balance up, vault holding down by payout
* `close_state` — stream `Closed`, unaccrued reclaimed, balances recorded

## Failure modes and limits

| Symptom | Cause | Fix |
|---------|-------|-----|
| Verification phase `ok:false` | Read did not settle within poll budget | Re-run; check sequencer inclusion and poll env vars |
| `NO_ELIGIBLE_VAULT` | Vault missing or wrong id | Run vault init for `VAULT_ID` |
| `STREAM_DEPLETED` | Allocation exhausted | Top up or close and create a new stream |
| `claim` skipped, `"reason":"zero_accrued"` | No residual accrued at close | Raise `MIN_ACCRUED` or allow more accrual |
| AT init / verify fails | AT registration or fixture id | Re-run `scripts/auth-transfer-ensure.sh`; fix fixture provider id |
| Wallet open fails | Config or password | Check `wallet_config.json` path |

### Limits

* Stream ids are per-vault sequential integers

## GitHub handle

@FILL_IN

## Discord handle

FILL_IN

## Existing docs or specs

* LIP-155 (Payment Streams): https://lip.logos.co/anoncomms/raw/payment-streams.html
* lez-payment-streams repository: https://github.com/logos-co/lez-payment-streams
  (in-repo: `docs/payment-streams-module/`, `docs/reference/verification-matrix.md`, `docs/reference/integration-contracts.md`, `scripts/e2e.sh`, `scripts/module-e2e.sh`, `fixtures/testnet-module.json`)

## Estimated time to complete

* Cold start: 20–40 minutes
* Subsequent localnet runs: about 3–6 minutes (`make verify-module-local`)
* Subsequent testnet runs: about 5–10 minutes when inclusion is healthy (longer on a slow sequencer)

## Security notes

* Test keys in fixtures — use on test networks only
* Wallet password protects local storage; private keys stay in the wallet module
* Provider claims accrued value on a closed stream per LIP-155 rules
