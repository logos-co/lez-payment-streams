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
The script keeps the funded fixture owner, creates a fresh provider each run,
runs `authenticated_transfer` init (via the v0.2 wallet CLI on testnet), funds
both accounts with pinata, then executes the full lifecycle including claim.
See [Step 28](docs/plan/completed/step-28-user-journey-testnet.md).

## Prerequisites

* OS: Linux (Ubuntu 22.04+) or macOS 14+
* Hardware: 2 GB RAM, ~5 GB free disk
* Tools: Nix with flakes enabled; Rust toolchain with RISC Zero (for guest builds)
* Network: Internet access for Nix flakes

Cold start: [docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine](docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine)

## Commands and expected outputs

The journey exercises the complete payment stream lifecycle: create vault,
deposit funds, open a stream to a provider, top up the stream, wait for value
to accrue, have the provider claim accrued funds, then close the stream and
reclaim the unspent allocation. The chain runs the phases through
`scripts/module-e2e.sh`, dispatched by `scripts/e2e.sh`.

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

The localnet path creates fresh isolated owner/provider accounts under
`.scaffold/module-e2e-wallet/` and funds them via `lgs wallet topup`, looping the
faucet until the owner holds enough to cover the deposit plus gas. For this
demo, Alice (owner) and Bob (provider) are two accounts generated in the same
local wallet and driven by a single `logoscore` instance. In a real two-party
setup Bob would run his own wallet, hold his own provider key, and Alice would
share the vault id and stream id with him out of band so he knows where to
claim. The demo collapses both roles into one wallet to keep the scenario
single-host.

### Expected output

Exit code 0. Console narrative ends with `E2E COMPLETE: All phases succeeded`.
Artifact `.scaffold/e2e/artifacts/module-e2e-*.log`:

```jsonl
{"phase":"wallet_open","ok":true}
{"phase":"vault_init","ok":true}
{"phase":"deposit","ok":true}
{"phase":"deposit_balance","ok":true}
{"phase":"create_stream","ok":true}
{"phase":"topup_stream","ok":true}
{"phase":"topup_allocation","ok":true}
{"phase":"accrual","ok":true}
{"phase":"claim","ok":true}
{"phase":"claim_balance","ok":true}
{"phase":"close_stream","ok":true}
{"phase":"close_state","ok":true}
{"phase":"module_e2e_complete","ok":true}
```

The `*_balance`, `*_allocation`, `*_state` lines are on-chain verification lines
recorded by reading state back after the corresponding write settled:

* `deposit_balance` — `getVaultStatus` vault holding balance equals the deposit.
* `topup_allocation` — `getStreamStatus` allocation increased by exactly the
  top-up amount.
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

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

Make convenience alias: `make verify-module-local`.

## Expected result

Exit code 0. JSON-lines artifact at `.scaffold/e2e/artifacts/module-e2e-*.log`
with all write and verification phases reporting `"ok":true`, including
`deposit_balance`, `topup_allocation`, `accrual`, `claim_balance`,
`close_state`, and ending with `module_e2e_complete`.

## Configuration details

### Environment variables

* `PAYMENT_STREAMS_GUEST_BIN`: Path to compiled guest ELF
* `VAULT_ID`: Vault identifier (default: 0)
* `STREAM_ID`: Stream identifier (default: 0)
* `DEPOSIT`: Initial deposit amount (default: 500)
* `RATE`: Stream accrual rate per second (default: 1)
* `ALLOCATION`: Amount allocated to stream (default: 400)
* `TOPUP_INCREASE`: Tokens added during the top-up phase (default: 1)
* `FIXTURE_MANIFEST`: Testnet fixture path (default: `fixtures/testnet-module.json`)

### Verbosity

Console output level, set via `./scripts/e2e.sh --verbosity quiet|normal|verbose`
or the `E2E_VERBOSITY` environment variable:

* `quiet` — JSON-lines to artifact only, no console narrative (CI default when piped)
* `normal` — phase headers, status markers, on-chain values
* `verbose` — full narrative with inline payment-streams concept explanations (TTY default)

### Module requirements

Single-host configuration:
* `logos_execution_zone` — wallet and chain interface
* `payment_streams_module` — LIP-155 operations

No `delivery_module` needed (this is the module-only flow).

## Failure modes and limits

| Symptom | Cause | Fix |
|---------|-------|-----|
| Verification phase `ok:false` (`deposit_balance`, `topup_allocation`, `claim_balance`, `close_state`, …) | Write accepted but on-chain read did not settle within poll budget | Re-run the `getVaultStatus` / `getStreamStatus` / `getAccount` read; check sequencer inclusion and wallet sync |
| `NO_ELIGIBLE_VAULT` | Vault not initialized or wrong ID | Run `initializeVault` first |
| `STREAM_DEPLETED` | Stream ran out of allocated funds | `topUpStream` or `closeStream` then create new |
| `claim` returns 0 | No time elapsed for accrual | Wait longer between create and claim |
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
* Accrual between claim and close keeps accumulating: the closed stream may
  retain a residual accrued balance that stays allocated until a later claim.
  The close narrative reports the real residual rather than assuming zero.
* Stream IDs are per-vault sequential integers
* The TestNet v0.2 path reuses the same script and phase list but is not
  verified here: the shared testnet sequencer was stalled during validation
  (block height near-static, submitted transactions never included). Re-run the
  testnet path once the sequencer is healthy.

## GitHub handle

@FILL_IN

## Discord handle

FILL_IN

## Existing docs or specs

* **LIP-155 (Payment Streams)**: https://lip.logos.co/anoncomms/raw/payment-streams.html
* **Payment streams module**: [docs/payment-streams-module/README.md](docs/payment-streams-module/)
* **Module E2E script**: [scripts/module-e2e.sh](scripts/module-e2e.sh)
* **Entrypoint / dispatcher**: [scripts/e2e.sh](scripts/e2e.sh)
* **Verification matrix**: [docs/reference/verification-matrix.md](docs/reference/verification-matrix.md)

## Estimated time to complete

* Cold start: 20–40 minutes
* Subsequent runs: 2–5 minutes

## Security notes

* Test keys in fixtures — never reuse for production
* Wallet password protects local storage; private keys never leave wallet module
* Provider can claim without owner approval (designed behavior) but cannot withdraw more than accrued
