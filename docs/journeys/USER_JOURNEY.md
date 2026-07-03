### What the user achieves

A user creates and manages payment streams on the Logos Execution Zone,
enabling continuous payments to service providers with per-second granularity.

### Why it matters

Payment streams replace discrete invoices with continuous value flow,
enabling real-time compensation for ongoing services.
This journey demonstrates direct operation of the LIP-155 protocol through the Logos Core module interface on a single host (`MODE=module`).

### Key components

* `payment_streams_module` — Universal Logos Core module exposing LIP-155 vault and stream lifecycle
* `logos_execution_zone` — Wallet module for chain interaction and transaction signing
* `lez-payment-streams` — On-chain SPEL guest program implementing LIP-155 semantics
* `logoscore` — Logos Core CLI for module loading and method invocation

### Repository

https://github.com/logos-co/lez-payment-streams

### Runtime target

The same entrypoint runs on localnet and on public TestNet v0.2:
`scripts/e2e.sh` with `MODE=module` and `CHAIN=local` or `CHAIN=testnet`.
Localnet is single-host and faster for iteration; testnet is the primary verification target.

### Prerequisites

Verification setup: lez-payment-streams repository README (https://github.com/logos-co/lez-payment-streams#prerequisites).

### Commands and expected outputs

Lifecycle: vault init, deposit, create stream, optional top-up, accrual, close stream, claim.
`scripts/module-e2e.sh` runs the phases; `scripts/e2e.sh` dispatches by `MODE=module` and `CHAIN`.

#### Localnet verification

```bash
make verify-module-local
```

Equivalent:

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

Optional top-up phase:

```bash
MODULE_E2E_TOPUP=1 MODE=module CHAIN=local ./scripts/e2e.sh local run
```

#### Testnet verification

One-time bootstrap:

```bash
make bootstrap-testnet-module
```

Run:

```bash
make verify-module-testnet
```

Equivalent:

```bash
VAULT_ID=5 STREAM_ID=0 MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run
```

#### Expected output

Exit code 0.
Console narrative ends with `E2E COMPLETE: All phases succeeded`.
Default scenario:

```jsonl
{"phase":"wallet_open","ok":true}
{"phase":"auth_init_owner","ok":true}
{"phase":"auth_init_provider","ok":true}
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

### Expected result

Exit code 0.
Every phase in `.scaffold/e2e/artifacts/module-e2e-*.log` reports `"ok":true`,
including `deposit_balance`, `accrual`, `claim_balance`, `close_state`, and `module_e2e_complete`.
With `MODULE_E2E_TOPUP=1`, also `topup_stream` and `topup_allocation`.

### Configuration details

Environment variables:

* `PAYMENT_STREAMS_GUEST_BIN`: Path to compiled guest ELF
* `VAULT_ID`: Vault identifier (default: 0)
* `STREAM_ID`: Stream identifier (default: 0)
* `DEPOSIT`: Initial deposit amount (default: 500 local, 30 testnet)
* `RATE`: Stream accrual rate per second (default: 1)
* `ALLOCATION`: Amount allocated to stream (default: 400 local, 20 testnet)
* `MIN_ACCRUED`: Tokens that must accrue before claim (default: `RATE * 3` local, 1 testnet)
* `TOPUP_INCREASE`: Tokens added during the top-up phase when enabled (default: 1)
* `MODULE_E2E_TOPUP`: Set to `1` to run `topUpStream` between create and accrual
* `MODULE_E2E_SKIP_CLOSE`: Set to `1` to skip close and claim on testnet
* `FIXTURE_MANIFEST`: Testnet fixture path (default `fixtures/testnet-module.json`)

Module requirements:

Single-host configuration:
* `logos_execution_zone` — wallet and chain interface
* `payment_streams_module` — LIP-155 operations

### Failure modes and limits

| Symptom | Cause | Fix |
|---------|-------|-----|
| Verification phase `ok:false` | Read did not settle within poll budget | Re-run; check sequencer inclusion and poll env vars |
| `NO_ELIGIBLE_VAULT` | Vault missing or wrong id | Run vault init for `VAULT_ID` |
| `STREAM_DEPLETED` | Allocation exhausted | Top up or close and create a new stream |
| `claim` skipped, `"reason":"zero_accrued"` | No residual accrued at close | Raise `MIN_ACCRUED` or allow more accrual |
| AT init / verify fails | AT registration or fixture id | Re-run `scripts/auth-transfer-ensure.sh`; fix fixture provider id |
| Wallet open fails | Config or password | Check `wallet_config.json` path |

### GitHub handle

@s-tikhomirov

### Discord handle

sergei.tikhomirov

### Existing docs or specs

* LIP-155 (Payment Streams): https://lip.logos.co/anoncomms/raw/payment-streams.html

### Hardware requirements

See Prerequisites above.

### Estimated time to complete

Cold start: 20–40 minutes.
Subsequent localnet runs: about 3–6 minutes.
Subsequent testnet runs: about 5–10 minutes when inclusion is healthy.

### Security notes

* Test keys in fixtures — use on test networks only
* Wallet password protects local storage; private keys stay in the wallet module
* Provider claims accrued value on a closed stream per LIP-155 rules
