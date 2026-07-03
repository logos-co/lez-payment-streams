# User Journey: Operate Payment Streams via Logos Core Module

## What the user achieves

A user creates and manages payment streams on the Logos Execution Zone, enabling continuous micropayments to service providers with per-second granularity.

## Why it matters

Payment streams replace discrete invoices with continuous value flow, enabling real-time compensation for ongoing services. This journey demonstrates direct operation of the LIP-155 protocol through the Logos Core module interface, independent of any specific service integration.

## Key components

* `payment_streams_module` — Universal Logos Core module exposing LIP-155 vault and stream lifecycle
* `logos_execution_zone` — Wallet module for chain interaction and transaction signing
* `lez-payment-streams` — On-chain SPEL guest program implementing LIP-155 semantics
* `logoscore` — Logos Core CLI for module loading and method invocation

## Repository

https://github.com/logos-co/lez-payment-streams (FILL IN: exact repository URL)

## Runtime target

local LEZ (module only, single-host). The journey is verified end-to-end on
localnet with the phase list below. On TestNet v0.2, run `make deploy-testnet`
once after a sequencer relaunch, then `CHAIN=testnet ./scripts/module-e2e.sh`.
The script reuses owner and provider from `fixtures/testnet-module.json` or
`fixtures/testnet.json` (keys in `.scaffold/e2e/testnet-wallet/`), runs shared
`ps_auth_transfer_ensure` ([scripts/lib/auth_transfer.sh](scripts/lib/auth_transfer.sh)
via [scripts/auth-transfer-ensure.sh](scripts/auth-transfer-ensure.sh)) so both
accounts are registered under the `authenticated_transfer` program ImageID, then
funds via wallet pinata when balances are low, then runs the full lifecycle
(close, then claim). Replace a broken fixture provider id if AT verify fails.
See [Step 28](docs/plan/completed/step-28-user-journey-testnet.md) and Step 32.

## Prerequisites

* OS: Linux (Ubuntu 22.04+) or macOS 14+
* Hardware: 2 GB RAM, ~5 GB free disk
* Tools: Nix with flakes enabled; Rust toolchain with RISC Zero (for guest builds)
* Network: Internet access for Nix flakes

Cold start: [docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine](docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine)

## Commands and expected outputs

The journey exercises the complete payment stream lifecycle: create vault,
deposit funds, open a stream to a provider, wait for value to accrue, close the
stream, then have the provider claim residual accrued on the closed stream.
An optional **top-up** phase (`topUpStream`) runs between stream
creation and accrual when enabled (see `MODULE_E2E_TOPUP` below). The chain runs
the phases through `scripts/module-e2e.sh`, dispatched by `scripts/e2e.sh`.

Every balance- and state-changing step is verified by reading real on-chain
state back, not by reporting the script's input values. After each write the
script polls `getVaultStatus` / `getStreamStatus` (via the module's `chainAction`
read ops) or the sequencer's `getAccount` until the expected change is reflected,
then records it in the narrative and the JSON-lines artifact. This is what makes
the demo a faithful demonstration of on-chain behavior.

### Run the end-to-end demo

The tester runs a single command. It prints a narrative console trace (phase
headers, status markers, and on-chain values) and writes a JSON-lines artifact
to `.scaffold/e2e/artifacts/module-e2e-*.log`. Verbosity defaults to `verbose` on
a TTY and `quiet` when piped; override with `--verbosity quiet|normal|verbose`.

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

To include the optional `topUpStream` phase (longer run):

```bash
MODULE_E2E_TOPUP=1 MODE=module CHAIN=local ./scripts/e2e.sh local run
```

The localnet path creates fresh isolated owner/provider accounts under
`.scaffold/module-e2e-wallet/` and funds them via `lgs wallet topup`, looping the
faucet until the owner holds enough to cover the deposit plus gas. For this
demo, Alice (owner) and Bob (provider) are two accounts generated in the same
local wallet and driven by a single `logoscore` instance. In a real two-party
setup Bob would run his own wallet, hold his own provider key, and Alice would
share the vault id and stream id with him out of band so he knows where to
claim. The demo collapses both roles into one wallet to keep the scenario
single-host.

### Testnet run

One-time after a public sequencer relaunch, deploy the guest program:

```bash
make deploy-testnet
```

Ensure `fixtures/testnet-module.json` exists (`make bootstrap-testnet-module`) or
fall back to `fixtures/testnet.json` with `owner_account_id` and
`provider_account_id` set. Keys live under `.scaffold/e2e/testnet-wallet/`.
Before chain writes, `scripts/module-e2e.sh` calls `ps_auth_transfer_ensure`
(strict on-chain `program_owner` check against the AT ImageID; init via wallet
`auth-transfer init` when missing) and pinata-funds accounts when needed.
`./scripts/e2e.sh` recomputes `FIXTURE_MANIFEST`, `WALLET_CONFIG`, and
`WALLET_STORAGE` from `CHAIN` so testnet runs never pick up local fixture paths.

Use an unused `VAULT_ID` (and `STREAM_ID=0`) on repeat runs:

```bash
make verify-module-testnet
```

Or explicitly:

```bash
VAULT_ID=5 STREAM_ID=0 MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run
```

Testnet defaults are smaller than localnet (`DEPOSIT=30`, `ALLOCATION=20`,
`MIN_ACCRUED=1`) to keep wall clock down; see configuration below. Expect
several minutes dominated by transaction inclusion, not accrual polling.

### Expected output

Exit code 0. Console narrative ends with `E2E COMPLETE: All phases succeeded`.
Artifact `.scaffold/e2e/artifacts/module-e2e-*.log` (default scenario, no top-up):

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

With `MODULE_E2E_TOPUP=1`, the artifact also includes `topup_stream` and
`topup_allocation` after `create_stream`.

The `*_balance`, `*_allocation`, `*_state` lines are on-chain verification lines
recorded by reading state back after the corresponding write settled:

* `deposit_balance` — `getVaultStatus` vault holding balance equals the deposit.
* `topup_allocation` — (only when `MODULE_E2E_TOPUP=1`) `getStreamStatus`
  allocation increased by exactly the top-up amount.
* `accrual` — polled via `getStreamStatus` until `accrued_lo` exceeds a minimum
  derived from the rate (no fixed sleep).
* `claim_balance` — the provider's `getAccount` balance increased and the vault
  holding balance decreased by the same payout.
* `close_state` — `getStreamStatus` state is `Closed` with `unaccrued` reclaimed
  to the vault, plus the final vault holding balance and `total_allocated`.

The write-phase lines (`deposit`, `create_stream`, …) record that the transaction
was accepted; the verification lines record that the expected on-chain change
actually settled. A verification line may report `ok:false` if the read did not
settle within the poll budget — the write itself still succeeded, and the hint
points to re-reading or checking sequencer inclusion.

## Success command

Localnet:

```bash
make verify-module-local
```

Testnet (pick an unused vault id):

```bash
VAULT_ID=<unused> make verify-module-testnet
```

## Expected result

Exit code 0. JSON-lines artifact at `.scaffold/e2e/artifacts/module-e2e-*.log`
with all write and verification phases for the selected scenario reporting
`"ok":true`, including `deposit_balance`, `accrual`, `claim_balance`,
`close_state`, and ending with `module_e2e_complete`. When
`MODULE_E2E_TOPUP=1`, also expect `topup_stream` and `topup_allocation`.

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
* `MODULE_E2E_TOPUP`: Set to `1` to run `topUpStream` between create and accrual;
  default `0` (skipped) for a shorter demo
* `MODULE_E2E_SKIP_CLOSE`: Set to `1` to skip settlement (**close and claim**;
  saves two testnet txs). Do not claim on an active stream when skipped.
* `LEE_WALLET_HOME_DIR`: Set by the script — local module runs use
  `.scaffold/module-e2e-wallet`; testnet module runs use
  `.scaffold/e2e/testnet-wallet` (same bundle as Store testnet per Step 32 D1).
* `INCLUSION_ATTEMPTS`, `INCLUSION_SLEEP`: Poll budget for `getTransaction` after each write
* `ACCRUAL_ATTEMPTS`, `ACCRUAL_POLL_SLEEP`: Poll budget for `getStreamStatus` accrual
* `FIXTURE_MANIFEST`: Testnet fixture path (default: `fixtures/testnet-module.json`)

### Verbosity

Console output level, set via `./scripts/e2e.sh --verbosity quiet|normal|verbose`
or the `E2E_VERBOSITY` environment variable:

* `quiet` — JSON-lines to artifact only, no console narrative (CI default when piped)
* `normal` — phase headers, status markers, on-chain values
* `verbose` — full narrative with inline payment-streams concept explanations (TTY default)

Console markers (normal and verbose):

* `→` — step about to run
* `✓` — step succeeded
* `✗` — step failed
* `!` — clarification or warning (usually after a failure)

### Module requirements

Single-host configuration:
* `logos_execution_zone` — wallet and chain interface
* `payment_streams_module` — LIP-155 operations

No `delivery_module` needed (this is the module-only flow).

### Testnet run duration

Wall clock on `CHAIN=testnet` is dominated by **serial transaction inclusion**, not
the accrual wait. Each write waits for `getTransaction` with a poll budget
(default 45×2s on testnet). Five required payment-stream writes after AT ensure
(vault init, deposit, create stream, **close**, **claim**) can sum to many minutes
when the public sequencer includes blocks
irregularly (often on the order of **15–60 seconds** between heights; sometimes longer).

Accrual for the demo only needs **`MIN_ACCRUED` tokens** (default **1** on testnet at
`RATE=1`). That needs chain time to advance at least once after stream creation,
which is usually one block, not three seconds of wall clock.

To shorten a testnet demo further without dropping claim:

* Keep defaults (`DEPOSIT=30`, `ALLOCATION=20`, `MODULE_E2E_TOPUP=0`).
* Pick an unused `VAULT_ID` / `STREAM_ID` per run, or close the stream when reusing ids.
* Optional: `MODULE_E2E_SKIP_CLOSE=1` (skips close and claim; next run needs a new `STREAM_ID`).

Localnet keeps larger defaults (`DEPOSIT=500`, `ALLOCATION=400`, `MIN_ACCRUED=RATE*3`) for regression parity.

## Failure modes and limits

| Symptom | Cause | Fix |
|---------|-------|-----|
| Verification phase `ok:false` (`deposit_balance`, `topup_allocation`, `claim_balance`, `close_state`, …) | Write accepted but on-chain read did not settle within poll budget | Re-run the `getVaultStatus` / `getStreamStatus` / `getAccount` read; check sequencer inclusion and wallet sync |
| `NO_ELIGIBLE_VAULT` | Vault not initialized or wrong ID | Run `initializeVault` first |
| `STREAM_DEPLETED` | Stream ran out of allocated funds | `topUpStream` or `closeStream` then create new |
| `claim` skipped or zero payout | Stream closed with no residual accrued | Expected when `MIN_ACCRUED` was not met before close; increase accrual wait or `MIN_ACCRUED` |
| AT init / verify fails | Account not under `authenticated_transfer` ImageID | Re-run ensure: `scripts/auth-transfer-ensure.sh --owner … --provider …`; rotate broken provider in fixture |
| Wallet open fails | Wrong password or missing config | Check `wallet_config.json` path |

### Limits

* `claim` is required and verified. `claim` takes the stream `owner` explicitly
  (via the `chainAction` `owner` field) so the `vault_config` PDA is derived from
  the real stream creator, not a static fixture value; see
  [step-27](docs/plan/completed/step-27-claim-fix-verification.md) and
  [step-28](docs/plan/completed/step-28-user-journey-testnet.md).
* Pause/resume are not part of this journey. On localnet the chain clock
  advances much faster than wall-time, which can deplete a short-lived stream
  before a pause could land, so the demo omits those operations.
* On-chain verification reads (`getVaultStatus`, `getStreamStatus`,
  `getAccount`) may return "account data missing" immediately after a write
  because the module submits async and the wallet view has not synced yet. The
  script polls through this; a verification phase reports `ok:false` only if the
  expected change did not settle within the poll budget, while the write phase
  itself still reports `ok:true`. See
  [docs/archive/steps/module-chain-writes-runbook.md](docs/archive/steps/module-chain-writes-runbook.md).
* Settlement order is **close then claim**: unaccrued allocation returns to the
  vault at close; residual **accrued** on the closed stream is what the provider
  claims next. The `close_state` line reports accrued/unaccrued after close; claim
  is skipped with `"reason":"zero_accrued"` when nothing remains to pay out.
* Stream IDs are per-vault sequential integers
* The TestNet v0.2 path reuses fixture owner and provider; use an unused
  `VAULT_ID` on repeat runs. After a testnet relaunch, run `make deploy-testnet`.

## GitHub handle

@FILL_IN

## Discord handle

FILL_IN

## Existing docs or specs

* **LIP-155 (Payment Streams)**: https://lip.logos.co/anoncomms/raw/payment-streams.html
* **Payment streams module**: [docs/payment-streams-module/README.md](docs/payment-streams-module/)
* **Module E2E script**: [scripts/module-e2e.sh](scripts/module-e2e.sh)
* **AT ensure (shared)**: [scripts/lib/auth_transfer.sh](scripts/lib/auth_transfer.sh), [scripts/auth-transfer-ensure.sh](scripts/auth-transfer-ensure.sh)
* **Entrypoint / dispatcher**: [scripts/e2e.sh](scripts/e2e.sh)
* **Verification matrix**: [docs/reference/verification-matrix.md](docs/reference/verification-matrix.md)

## Estimated time to complete

* Cold start: 20–40 minutes
* Subsequent localnet runs: about 3–6 minutes (`make verify-module-local`)
* Subsequent testnet runs: about 5–10 minutes when inclusion is healthy (`make verify-module-testnet` with a fresh `VAULT_ID`; can be longer on a slow sequencer)

## Security notes

* Test keys in fixtures — never reuse for production
* Wallet password protects local storage; private keys never leave wallet module
* Provider can claim without owner approval (designed behavior) but cannot withdraw more than accrued
