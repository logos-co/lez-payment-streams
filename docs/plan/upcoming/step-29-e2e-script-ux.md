# Step 29 — E2E Script UX Enhancement

Active-work packet for agents. Index: [index.md](../index.md).

## Goal

Add narrative console output to the two E2E scenario scripts so that a
developer or researcher across the Logos organization can follow what is
happening without reading source code. The two scenarios address different
audiences and remain in separate scripts.

Prerequisite: None (can develop against `CHAIN=local` immediately).

## Two scenarios

The E2E suite implements two distinct scenarios that share on-chain
infrastructure (vault, stream, accrual, claim) but serve different purposes:

| Aspect | User Journey | Developer Journey |
| --- | --- | --- |
| Mode | `MODE=module` | `MODE=store` |
| Script | `scripts/module-e2e.sh` (bash) | `scripts/e2e/run_local_e2e.py` (Python) |
| Scenario | Alice pays Bob via a payment stream | User proves stream eligibility for a paid Store query |
| Stream role | The product | Enabling infrastructure for proof |
| Store / delivery | Not present | Central |
| Eligibility proof | Not present | Central |
| Claim | Climactic phase | Bookkeeping at teardown |
| Audience question | What is a payment stream? | How do I gate Store access behind payment? |

The scripts stay separate. A shared formatting convention (same timestamp
format, header style, status glyphs) keeps visual consistency without coupling
scenario logic. `e2e.sh` dispatches to the right script based on `MODE` and
passes through the verbosity flag.

## Target audience

Developers and researchers across the Logos organization who work on other
parts of the stack (delivery, basecamp, core, modules) and are familiar with
blockchain and networking concepts broadly, but not with payment streams
specifics (vaults, streams, accrual, eligibility proofs).

The testnet-based demonstrations serve to show that payment streams
functionality interoperates successfully with the running testnet. The
narrative should include enough background on payment streams concepts for
this audience to follow the scenario.

## Verbosity levels

Three levels, controlled by a `--verbosity` flag (short form `-v` takes
`quiet|normal|verbose`):

| Level | Default for | Output |
| --- | --- | --- |
| `quiet` | CI / gates | JSON-lines to artifact file only. No console narrative. |
| `normal` | Operator runs | Phase headers, status markers (✓/✗), elapsed time, on-chain values. No concept explanations. |
| `verbose` | Demos | Full narrative with inline background on payment streams concepts. |

Console markers in `module-e2e.sh`: `→` upcoming step, `✓` success, `✗` failure,
`!` clarification after a failure. Failures never reuse success wording (e.g.
`vault_init failed: transaction not included on chain`, not `✗ Vault created`).

Default when run interactively (TTY): `verbose`. Default when piped or in CI:
`quiet`. The `e2e.sh` wrapper accepts `--verbosity` and passes it through to
whichever scenario script it dispatches to.

## On-chain values in the narrative

The narrative prints actual on-chain values at each phase so the viewer can
verify that rate, elapsed time, and accrued amount are consistent:

- Stream creation: `rate`, `allocation`, `stream_id`, `vault_id`.
- Accrual wait: expected accrual window, polled `unaccrued_lo`, elapsed
  chain time.
- Claim: claimed amount, provider's resulting balance.
- Close: stream final state (accrued, unspent allocation reclaimed, vault
  balance after reclaim).

Accrual is timestamp-based, derived from the on-chain `accrued_as_of`
field. On localnet the folded clock advances quickly; on testnet the
timestamps come from block information, so granularity is limited by block
time. The narrative acknowledges this: it prints "expecting ~N tokens based
on rate and elapsed chain time" and then reports the actual amount at claim.

## Chain portability

Both `CHAIN=local` and `CHAIN=testnet` share the same phase list and core
architecture. The narrative does not add or remove phases based on chain.
On localnet, "waiting for block confirmation" is deterministic and fast; on
testnet it is probabilistic and slower, but the concept is identical.

Phase headers are unnumbered (no "PHASE 4/6") because elapsed time and
retry counts vary by chain. A fixed count would mislead the viewer on
testnet.

## Failure output

When a phase fails after all retries, the narrative includes diagnostic
hints suggesting likely causes and next steps. Example: "Stream creation
failed — check sequencer height, wallet sync, and gas balance." The JSON-lines
artifact retains the raw error for programmatic diagnosis.

## Messaging is a black box

The developer journey publishes a payload to a content topic to seed the
Store with messages before the query phase. The narrative treats this as a
setup step ("Publishing test messages to Store...") without explaining the
Waku/delivery layer. The audience is assumed to be familiar with the
messaging stack, or the presenter explains it verbally during a live demo.

## Phase lists

### User Journey (`MODE=module`) — "Alice pays Bob via a payment stream"

```
[19:00:01] ============================================
[19:00:01] Payment Streams E2E: User Journey (LocalNet)
[19:00:01] Scenario: Alice creates a stream to Bob, funds accrue,
[19:00:01]          Bob claims accrued amount, Alice closes stream
[19:00:01] ============================================
[19:00:01]
[19:00:02] PHASE: Environment Setup
[19:00:02]   → Starting LEZ sequencer
[19:00:04]   ✓ Sequencer ready (block height: 1)
[19:00:04]   → Starting logoscore, loading modules
[19:00:07]   ✓ logoscore ready, modules loaded: logos_execution_zone, payment_streams_module
[19:00:07]
[19:00:07] PHASE: Vault Initialization
[19:00:07]   → Alice creates vault 0
[19:00:09]   ✓ Vault created: vault_id=0
[19:00:09]   → Depositing 100 tokens into vault
[19:00:11]   ✓ Deposit confirmed, vault balance: 100
[19:00:11]
[19:00:11] PHASE: Stream Creation
[19:00:11]   → Alice opens stream 0 to Bob
[19:00:11]     rate=10 tokens/sec, allocation=80 tokens, vault=0
[19:00:13]   ✓ Stream created: stream_id=0, status=ACTIVE
[19:00:13]
[19:00:13] PHASE: Stream Lifecycle
[19:00:13]   → Pausing stream 0
[19:00:15]   ✓ Stream paused
[19:00:15]   → Resuming stream 0
[19:00:17]   ✓ Stream resumed
[19:00:17]   → Topping up stream 0 by 1 token
[19:00:19]   ✓ Top-up confirmed, allocation now 81 tokens
[19:00:19]
[19:00:19] PHASE: Accrual
[19:00:19]   → Waiting for funds to accrue (rate=10 tokens/sec)
[19:00:19]   → Accrual is timestamp-based: derived from on-chain
[19:00:19]     accrued_as_of field. On testnet, granularity is limited
[19:00:19]     by block time.
[19:00:24]   ✓ Accrued: ~50 tokens after 5s of chain time
[19:00:24]
[19:00:24] PHASE: Claim
[19:00:24]   → Bob claims accrued funds from stream 0
[19:00:26]   ✓ Claim successful: Bob received 50 tokens
[19:00:26]   ✓ Provider balance: 50
[19:00:26]
[19:00:26] PHASE: Close
[19:00:26]   → Alice closes stream 0, reclaims unspent allocation
[19:00:28]   ✓ Stream closed
[19:00:28]   ✓ Stream status: accrued=50, unspent reclaimed=31, vault balance=81
[19:00:28]
[19:00:28] ============================================
[19:00:28] E2E COMPLETE: All phases succeeded
[19:00:28] Artifact: .scaffold/e2e/artifacts/module-e2e-*.log
[19:00:28] ============================================
```

### Developer Journey (`MODE=store`) — "User proves stream eligibility for paid Store query"

```
[19:05:01] ============================================
[19:05:01] Payment Streams E2E: Developer Journey (LocalNet)
[19:05:01] Scenario: User proves stream eligibility to Provider
[19:05:01]          for paid Store query (LIP-155, RFC 73 tag 30)
[19:05:01] ============================================
[19:05:01]
[19:05:02] PHASE: Environment Setup
[19:05:02]   → Starting LEZ sequencer
[19:05:04]   ✓ Sequencer ready (block height: 1)
[19:05:04]   → Starting provider logoscore, loading modules
[19:05:07]   ✓ Provider ready: delivery_module (Store enabled), payment_streams_module
[19:05:07]   → Starting user logoscore, loading modules
[19:05:10]   ✓ User ready: delivery_module (Store client), payment_streams_module
[19:05:10]   → Publishing test messages to Store...
[19:05:25]   ✓ Messages published and propagated
[19:05:25]
[19:05:25] PHASE: Stream Creation
[19:05:25]   → User creates payment stream to provider
[19:05:25]     rate=1 token/sec, allocation=200 tokens, vault=0
[19:05:27]   ✓ Stream created: stream_id=0, status=ACTIVE
[19:05:27]   → Waiting for stream to become fundable (need ≥64 unaccrued tokens)
[19:05:27]     Accrual is timestamp-based: derived from on-chain
[19:05:27]     accrued_as_of field. On testnet, granularity is limited
[19:05:27]     by block time.
[19:06:31]   ✓ Stream fundable: 64 unaccrued tokens after 64s of chain time
[19:06:31]
[19:06:31] PHASE: Eligibility Proof Generation
[19:06:31]   → User generates LIP-155 eligibility proof from active stream
[19:06:31]     Proof derives from on-chain stream state, valid for current block
[19:06:33]   ✓ Proof generated: 128 bytes, stream_id=0
[19:06:33]
[19:06:33] PHASE: Paid Store Query
[19:06:33]   → User sends Store query with eligibility proof attached
[19:06:35]   → Provider verifies proof against LEZ on-chain state
[19:06:35]   ✓ Proof valid: stream active, 64 tokens accrued
[19:06:35]   → Serving historical messages (paid query)
[19:06:35]   ✓ Store query returned 99 messages, status 200
[19:06:35]
[19:06:35] PHASE: Rejection Path
[19:06:35]   → User sends Store query without eligibility proof
[19:06:37]   ✗ Query rejected (no eligibility proof)
[19:06:37]   ✓ Provider correctly returned 0 messages
[19:06:37]
[19:06:37] PHASE: Teardown
[19:06:37]   → Closing stream 0
[19:06:39]   ✓ Stream closed
[19:06:39]   → Provider claims accrued funds
[19:06:41]   ✓ Claim successful: 64 tokens claimed, provider balance: 64
[19:06:41]   ✓ Vault liquidity verified: allocated=0, unallocated=800
[19:06:41]
[19:06:41] ============================================
[19:06:41] E2E COMPLETE: All phases succeeded
[19:06:41] Artifact: .scaffold/e2e/artifacts/e2e-*.log
[19:06:41] ============================================
```

## Implementation scope

### User Journey (`scripts/module-e2e.sh`)

- Add `closeStream` call after `claim` (currently missing).
- Replace fixed `sleep 5` before claim with on-chain accrual polling
  (poll `getStreamStatus` until `unaccrued_lo` exceeds a minimum threshold,
  similar to `wait_for_stream_fundable` in the Python orchestrator).
- Add narrative printing functions: `narrative_phase`, `narrative_step`,
  `narrative_ok`, `narrative_fail`.
- Add `--verbosity` flag parsing (`quiet|normal|verbose`).
- Extract and print on-chain values (rate, allocation, accrued, balances)
  from existing `chainAction` call responses.
- Print diagnostic hints on failure.

### Developer Journey (`scripts/e2e/run_local_e2e.py`)

- Add narrative printing alongside existing `log_artifact` calls. The
  JSON-lines artifact format stays unchanged; narrative is printed to
  stderr or stdout depending on verbosity.
- Add `--verbosity` argument to `argparse` in `main()`.
- Extract on-chain values from existing query responses
  (`vault_status_json`, `stream_config_on_chain`, `vault_liquidity_*`)
  and print them in the narrative.
- Print diagnostic hints on failure (enrich existing `E2EError` messages).

### Entrypoint (`scripts/e2e.sh`)

- Accept `--verbosity` flag, pass through to dispatched scenario script.
- Default: `verbose` if TTY, `quiet` if piped/CI.

## Design constraints

- JSON-lines artifact format is unchanged (non-regression for CI gates).
- Same phase list on `CHAIN=local` and `CHAIN=testnet`.
- Phase headers are unnumbered (count varies by chain due to retries and
  timing).
- Messaging/Waku layer is treated as a black box in the narrative.
- The two scenario scripts stay in their current languages (bash / Python).
- Shared formatting convention (timestamp format, glyphs, header style) but
  no shared scenario code.

## Deliverables

- [ ] User Journey (`scripts/module-e2e.sh`): add `closeStream`, accrual
  polling, narrative output, `--verbosity` flag.
- [ ] Developer Journey (`scripts/e2e/run_local_e2e.py`): add narrative
  output alongside `log_artifact`, `--verbosity` flag.
- [ ] Entrypoint (`scripts/e2e.sh`): accept and pass through `--verbosity`.
- [ ] Three verbosity levels implemented: `quiet`, `normal`, `verbose`.
- [ ] On-chain values (rate, allocation, accrued, balances) printed at
  relevant phases.
- [ ] Diagnostic hints on failure for both scenarios.
- [ ] Both `CHAIN=local` and `CHAIN=testnet` produce consistent phase lists.
- [ ] JSON-lines artifacts unchanged (non-regression).
- [ ] Documentation: brief guide on how to read E2E output.

## Definition of done

- [ ] `MODE=module CHAIN=local` produces full narrative with on-chain values.
- [ ] `MODE=store CHAIN=local` produces full narrative with eligibility flow.
- [ ] `--verbosity quiet` suppresses all console narrative, JSON-lines only.
- [ ] `--verbosity normal` shows phase headers + status + values, no concept
  explanations.
- [ ] `CHAIN=testnet` produces same phase list as `CHAIN=local`.
- [ ] CI gate (`make verify-store-local`, `make verify-module-local`) passes
  with default verbosity (quiet when piped).
- [ ] JSON-lines artifacts unchanged format.

## Related

- `scripts/module-e2e.sh` — user journey script to enhance
- `scripts/e2e/run_local_e2e.py` — developer journey orchestrator to enhance
- `scripts/e2e.sh` — entrypoint / dispatcher
- [step-20-developer-journey.md](step-20-developer-journey.md) — benefits from clear UX
- [step-22-ui-journey.md](step-22-ui-journey.md) — CLI journey benefits from clear UX
