# Step 13 — provider-side eligibility verification

Status: complete for integration DoD (feature in tree, `./scripts/verify-step13-dod.sh`,
`make verify-step13`). Logoscore prepare → verify cross-test needs a fresh or recently seeded
localnet when stream `0` is not fully accrued (same recovery story as Step 12).

Provider inbound verifier for Store eligibility bytes. Normative design: [plan/completed/step-13-normative.md](plan/completed/step-13-normative.md).
Index: [integration-index.md](../integration-index.md).

Prerequisites: Step 12 user path green (`./scripts/verify-step12-dod.sh`), Steps 10a–11b chain
fixture, Step 11c signing, installed `payment_streams_module` `.lgx`, guest ELF
(`PAYMENT_STREAMS_GUEST_BIN`). Demo policy numbers and N8 wire tooling live in
[`step12-user-eligibility.md`](step12-user-eligibility.md).

Runtime loop: [`logos-runtime-guide.md`](logos-runtime-guide.md) Part 3 (Step 13 subsection).
Recovery: [`demo-localnet-recovery.md`](demo-localnet-recovery.md).

## Method

`verifyEligibilityForStoreQuery(proofBytes, canonicalRequestBytes, requesterPeerId)`

Encoding matches Step 12: lowercase hex for `proofBytes` (serialized `EligibilityProof`) and
`canonicalRequestBytes` (full N8 canonical payload). `requesterPeerId` is logged only; it does
not affect eligibility predicates in the MVP demo.

## JSON response

- OK: `{"status":"ok","eligibility":"OK"}`
- Verdict failure: `{"status":"error","eligibility":"PARAMS_REJECTED|PROOF_INVALID|STREAM_NOT_ACTIVE","message":"…"}`
- Caller error: `{"status":"error","message":"…"}` without `eligibility`

Step 13 uses `eligibility` for LIP-155 verdict strings. Step 12 prepare errors use `code`
(`NO_ELIGIBLE_VAULT`, `STREAM_DEPLETED`, etc.) — do not overload those enums on the verifier.

## Provider persistence

Extends Step 12 `payment_streams_state.json` at the logoscore `instancePersistencePath`:

- `schema_version` `2` with `provider_acceptances` (migration from v1 on load).
- User-side `negotiations` / session keys unchanged; provider rows keyed by
  `(vault_id, provider_id_hex)`.

Wipe module persist when starting a new demo session unless you intentionally continue one
negotiation arc. See recovery doc.

## Demo provider payee

Inbound verify binds `VaultProof.provider_id` and on-chain stream payee to
[`fixtures/localnet.json`](../fixtures/localnet.json) `provider_account_id` (default manifest
path via `FIXTURE_MANIFEST`, same as Step 12). `registerProviderMapping` remains user-outbound
routing only; it is not the provider self-id for verify.

## Verify

Offline and structural checks (no logoscore):

```bash
VERIFY_LOGOSCORE=0 ./scripts/verify-step13-dod.sh
# or
make verify-step13
```

Checks FFI `parse_eligibility_proof_bytes`, installed plugin, and `lm methods` for
`verifyEligibilityForStoreQuery`.

Logoscore cross-test (sequencer on `127.0.0.1:3040`, wallet config/storage, guest ELF):

```bash
export PAYMENT_STREAMS_GUEST_BIN="$PWD/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
export PERSIST_DIR="$PWD/.scaffold/step13-persist-$(date +%s)"
VERIFY_LOGOSCORE=1 ./scripts/verify-step13-dod.sh
```

Script flow: register provider mapping, optional `topUpStream`, sync, `prepareEligibilityForStoreQuery`,
then `verifyEligibilityForStoreQuery` with the same `bytes_hex` and N8 wire; tampered canonical
must yield `PROOF_INVALID`.

Strict cross-test (honest `stream_proof` on seeded stream `0`, not SKIP on depleted):

```bash
./scripts/demo-localnet-fresh.sh
export PAYMENT_STREAMS_GUEST_BIN="$PWD/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
export PERSIST_DIR="$PWD/.scaffold/step13-persist-$(date +%s)"
VERIFY_LOGOSCORE=1 ./scripts/verify-step13-dod.sh
```

Run fresh reset immediately before verify when stream `0` has been exercised by earlier smokes.
Aged localnet with `unaccrued_lo: 0` on stream `0` cannot pass provider `OK` even if Step 12
prepare returned `STREAM_DEPLETED` or skipped.

Optional faster re-seed when you already trust the chain layout:

```bash
SKIP_VERIFY=1 ./scripts/demo-localnet-fresh.sh
VERIFY_LOGOSCORE=1 ./scripts/verify-step13-dod.sh
```

## Troubleshooting

| Symptom | Likely cause | Action |
| --- | --- | --- |
| Cross-test SKIP (stream depleted) | Stream `0` fully accrued on long-lived localnet | `./scripts/demo-localnet-fresh.sh`, new `PERSIST_DIR`, re-run verify |
| `verify` → `STREAM_NOT_ACTIVE` / depleted after OK prepare | Chain accrued between calls on depleted fixture | Fresh seed; run prepare and verify in one logoscore session without extra delay |
| `UNKNOWN_PROVIDER` / base58 failed | Wallet not open or LEZ module not loaded | `logoscore load-module logos_execution_zone`, `open` wallet before PS calls |
| `proofBytes … must be non-empty` | Prepare failed; no `bytes_hex` | Fix prepare path first (`REQUIRE_STREAM_PROOF=1` Step 12 helper) |
| Provider id mismatch | Manifest `provider_account_id` stale vs chain | Re-seed; align `FIXTURE_MANIFEST` with [`fixtures/localnet.json`](../fixtures/localnet.json) |
| `logos_execution_zone client unavailable` | Wallet not opened on this logoscore instance | `logoscore call logos_execution_zone open …` |

`PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF` is emergency-only (Step 12); provider verify still
requires active folded stream state for `OK` on the `stream_proof` arm.

## Handoff

Step 14 (Store tag `30`) and Step 15 (`liblogosdelivery` hooks) are complete on
`logos-delivery` branch `feat/payment-streams-store-eligibility`.
Step 16 wires `delivery_module` to call this method on inbound Store queries. Step 17 two-host E2E builds on
Steps 12–14 plus delivery integration.
