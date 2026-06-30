# Step 29 — plan excerpt

Active-work packet for agents. Index: [index.md](../index.md).

### Step 29, E2E Script UX Enhancement

Improve E2E script output to clearly explain each phase to technically
competent users who may be unfamiliar with payment streams mechanics.

Prerequisite: None (can develop against `CHAIN=local` immediately).

#### Background

Current E2E scripts (`scripts/e2e.sh`, `scripts/e2e/run_local_e2e.py`) produce
JSON-lines artifacts but minimal console output explaining what is happening.
This step adds narrative output suitable for demonstration and education.

#### Target audience

- Technically competent (can read CLI output, understands basic blockchain concepts)
- Unfamiliar with payment streams specifics (vaults, streams, accrual, eligibility proofs)
- Needs clear scenario explanation without reading source code

#### Output format requirements

Each E2E run should produce console output like:

```
[14:32:01] ============================================
[14:32:01] Payment Streams E2E: User Journey (LocalNet)
[14:32:01] Scenario: Alice creates a stream to Bob, funds accrue,
[14:32:01]          Bob claims accrued amount after 30 seconds
[14:32:01] ============================================
[14:32:01]
[14:32:02] PHASE 1/6: Environment Setup
[14:32:02]   → Starting local LEZ sequencer at 127.0.0.1:3040
[14:32:04]   ✓ Sequencer ready (block height: 1)
[14:32:04]
[14:32:04] PHASE 2/6: Vault Initialization
[14:32:04]   → Alice creates vault for payment streams
[14:32:06]   ✓ Vault created: vault_id=abc123...
[14:32:06]   → Depositing 1000 tokens into vault
[14:32:08]   ✓ Deposit confirmed, vault balance: 1000
[14:32:08]
[14:32:08] PHASE 3/6: Stream Creation
[14:32:08]   → Alice opens stream to Bob at 1 token/second
[14:32:10]   ✓ Stream created: stream_id=xyz789...
[14:32:10]   → Stream is ACTIVE, funds accruing to Bob
[14:32:10]
[14:32:10] PHASE 4/6: Accrual Period
[14:32:10]   → Waiting 30 seconds for funds to accrue...
[14:32:40]   ✓ Estimated accrued: 30 tokens (some variance expected)
[14:32:40]
[14:32:40] PHASE 5/6: Claim
[14:32:40]   → Bob claims accrued funds from stream
[14:32:42]   ✓ Claim successful, Bob received ~30 tokens
[14:32:42]
[14:32:42] PHASE 6/6: Cleanup
[14:32:42]   → Alice closes stream, reclaims remaining funds
[14:32:44]   ✓ Stream closed, vault balance reclaimed
[14:32:44]
[14:32:44] ============================================
[14:32:44] E2E COMPLETE: All phases succeeded
[14:32:44] Artifact: .scaffold/e2e/artifacts/module-e2e-*.log
[14:32:44] ============================================
```

#### Deliver

- [ ] Define output format specification (timestamps, phase headers, progress indicators)
- [ ] Implement in `scripts/e2e/run_local_e2e.py` (Python orchestrator)
- [ ] Implement in `scripts/e2e.sh` (bash wrapper)
- [ ] Add `--verbose` / `--quiet` flags (default: verbose)
- [ ] Document scenario being executed (e.g., "Alice pays Bob via stream")
- [ ] Explain payment streams concepts inline (accrual, vault, claim)
- [ ] Maintain JSON-lines artifacts for programmatic use

#### Developer Journey specific additions

For `MODE=store`, additionally explain:

```
[14:35:10] PHASE X: Eligibility Proof Generation
[14:35:10]   → User requests Store query with payment proof
[14:35:10]   → Generating LIP-155 eligibility proof from active stream
[14:35:12]   ✓ Proof generated: 128 bytes, valid for current block
[14:35:12]   → Attaching proof to Store request (RFC 73 tag 30)
[14:35:12]
[14:35:12] PHASE Y: Provider Verification
[14:35:12]   → Provider receives Store request with eligibility proof
[14:35:12]   → Verifying proof against LEZ on-chain state
[14:35:14]   ✓ Proof valid: stream active, 47 tokens accrued
[14:35:14]   → Serving historical messages (paid query)
```

#### Definition of done

- [ ] Output format specification documented
- [ ] User Journey (`MODE=module`) shows clear phase-by-phase narrative
- [ ] Developer Journey (`MODE=store`) shows eligibility proof flow narrative
- [ ] Both `CHAIN=local` and `CHAIN=testnet` produce consistent output
- [ ] `--quiet` flag available for CI/automation (JSON-lines only)
- [ ] Non-regression: JSON-line artifacts unchanged format
- [ ] README or documentation explains how to read E2E output

#### Parallel work

This step can proceed independently of Steps 26-28:
- Develop against `CHAIN=local` (no testnet dependency)
- Merge to master before testnet migration completes
- Steps 26-28 benefit from improved UX immediately

#### Related

- `scripts/e2e/run_local_e2e.py` — main orchestrator to enhance
- `scripts/e2e.sh` — entrypoint to enhance
- [step-20-developer-journey.md](step-20-developer-journey.md) — benefits from clear UX
- [step-22-ui-journey.md](step-22-ui-journey.md) — CLI journey benefits from clear UX
