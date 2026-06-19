# Step 12 — user-side eligibility

Status: complete for integration DoD (feature in tree, `./scripts/verify-step12-dod.sh`,
strict localnet path with `REQUIRE_STREAM_PROOF=1`). Step 13 provider verify is complete
separately ([`step13-provider-eligibility.md`](step13-provider-eligibility.md)). Step 16
`delivery_module` bridge is the next integration step ([N12](reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18));
not a blocker for Step 12 DoD.

Session keys, persisted negotiation state, and Store `EligibilityProof` bytes (LIP-155
`stream_proposal` / `stream_proof` arms) for the paid Store demo. Step 12 in
[`integration-index.md`](../integration-index.md).

Runbook for operators and implementors. Behavior matches the plan.

Prerequisites: Step 11c green (`./scripts/verify-step11c-dod.sh`), Step 11a reads, Step 11b
`chainAction` for manual `createStream`, Step 10b wallet with `sign_public_payload`.

Related: [N4 persistence](../reference/decisions-and-notes.md#n4-persistence-policy),
[N5 provider mapping](../reference/decisions-and-notes.md#n5-provider-identity-mapping),
[N8 canonical Store bytes](../reference/decisions-and-notes.md#n8-canonical-store-request-bytes-format),
[Step 11b writes](step11b-chain-writes.md).

Runtime loop: [`logos-runtime-guide.md`](logos-runtime-guide.md) Part 3.

## Demo goal

Show the user path for payment-stream Store eligibility on one local LEZ + two logical roles
(user host, provider host in Step 17):

1. Map provider libp2p `PeerId` to LEZ payee account id.
2. First Store query → `EligibilityProof` with `stream_proposal` (vault owner signs; session key committed).
3. Provider accepts; user opens stream on-chain via `chainAction` `createStream` (not automatic).
4. Next query → `EligibilityProof` with `stream_proof` over the same canonical Store request bytes.

Step 16 routes prepare output into outbound Store via `delivery_module`; Step 17 exercises
the full two-host path ([D2](../reference/decisions-and-notes.md#d2-delivery-module-hook-design),
[N12](../reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)).
Step 12 can be tested with logoscore + FFI only.

## Environment

Same as Step 11b where chain I/O is involved:

| Variable | Purpose |
| --- | --- |
| `FIXTURE_MANIFEST` | Default `fixtures/localnet.json` |
| `MODULES` | `lgpm` + `logoscore -m` install dir |
| `WALLET_CONFIG` / `WALLET_STORAGE` | Required for proposal path: `logos_execution_zone open` before `prepareEligibilityForStoreQuery` (`sign_public_payload`) |
| `PAYMENT_STREAMS_GUEST_BIN` | On daemon when exercising `createStream` after a proposal |
| `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF` | Demo-only escape hatch when stream is depleted; prefer `./scripts/demo-localnet-fresh.sh` instead |

## User vault (demo)

Eligibility uses one vault for the MVP, loaded from the fixture on first use (N10 fixture
config). `vault_id` comes from the manifest (local default `0` after Step 10a seed).

| Field | Source | Demo note |
| --- | --- | --- |
| `owner_account_id` | manifest | Signs `VaultProof` via `sign_public_payload` |
| `vault_id` | manifest `vault_id` | Reuse the seeded / testnet vault; do not require a separate vault `1` for Step 12 |
| `program_id_hex` | manifest | PDA derivation (already used by `chainAction`) |
| `provider_account_id` | manifest | Bytes in `VaultProof.provider_id` / on-chain `StreamConfig.provider` |

### Persistence across runs

Local demos: see [`demo-localnet-recovery.md`](demo-localnet-recovery.md). Chain and manifest
are disposable; wipe `payment_streams_state.json` (fresh `--persistence-path`) each demo unless
you continue one session. Blank slate: `./scripts/demo-localnet-fresh.sh`.

On-chain state (vault holding, existing streams) persists for the life of that network:

- Local: while `.scaffold/` sequencer state is intact and Step 10a seed is not reset.
- Public testnet: deploy program once, `initialize_vault` + `deposit` once per demo owner;
  reuse the same vault across runs and hosts via an operator manifest (same shape as
  `fixtures/localnet.json`, different `sequencer_url` and ids).

Off-chain Step 12 state (session keys, pending proposals, peer mapping, local `stream_id`
inventory) lives under `instancePersistencePath` (N4). For local smoke, use a new path per run;
do not reuse a stale dir after chain reset.

### Stream id (same vault, new negotiation)

Step 10a may already have `stream_id: 0` ACTIVE on vault `0`. That is enough for
proof-path smoke and `listMyStreams`, but it skips the proposal → `createStream` narrative.

For the full demo arc on the same vault, allocate the next free `stream_id` in module
inventory (e.g. `1` when `0` is taken), issue a `StreamProposal`, then manual `chainAction`
`createStream` with matching params, then `StreamProof`. Step 11b’s default `vault_id: 1`
remains for isolated write-lifecycle tests only; Step 12 and Step 17 default to manifest
`vault_id`, not vault `1`.

Stream id allocation (new proposal): use on-chain `VaultConfig.next_stream_id` from the
manifest vault’s config PDA (Step 11a read) when starting a new `(vault_id, provider_id)`
negotiation. Persist that id for the pending proposal. Local inventory records ids after
`chainAction` `createStream` succeeds; `rediscoverStreams` scans `stream_id` 0, 1, … on chain
and merges into inventory. Do not reuse an id that still has an `ACTIVE` stream on chain for the
same vault; proof path uses the established `stream_id` from persistence or chain, not a new id.

`NO_ELIGIBLE_VAULT`: module reads vault holding + config for that vault; proposed
`allocation` must not exceed unallocated balance (Step 3a / plan Step 12 errors).

MVP demo assumes a single user vault from the manifest. Multiple vaults per owner are out of
scope for Step 12 / Step 17; a later `configureVault` or manifest list is optional follow-on
only if the demo script changes (inventory is already keyed by `vault_id`).

## Default stream terms

Normative demo numbers for Step 12, Step 13 local verifier, and `verify-step12-dod.sh`.
Align with [`fixtures/localnet.json`](../fixtures/localnet.json) and
[`seed_localnet_fixture.rs`](../examples/src/bin/seed_localnet_fixture.rs). Each local pinata
claim is about 150 tokens; the seed script runs `SEED_WALLET_TOPUP_ROUNDS` (default 16) before
on-chain deposit (`TOPUP_ROUNDS * 150` must cover `SEED_DEPOSIT_AMOUNT`). Defaults are deposit
`2400`, stream `0` allocation `1800`, rate `1` (demo `min_rate` floor). Rough unaccrued runway at
rate `1` is on the order of `allocation` seconds (≈30 min) until eligibility sees
`STREAM_DEPLETED`; the 30 min window keeps a fresh seed plus an E2E run depletion-free, and the
`600`-token unallocated headroom (`2400-1800`) lets the E2E top-up loop refill mid-run.

Demo provider policy (localnet):

| Field | Value |
| --- | --- |
| `min_rate` | `1` |
| `min_allocation` | `1` |
| `max_create_stream_deadline_delay` | `3600` |

`StreamParams` by path (`service_id` always `/vac/waku/store-query/3.0.0`):

| Path | `stream_id` | `rate` | `allocation` | `create_stream_deadline` |
| --- | --- | --- | --- | --- |
| Proof-only on Step 10a seed | `0` (on-chain) | `1` | `1800` (on-chain) | n/a |
| Full arc on same seeded vault | next free (e.g. `1`) | `10` | `15` | `clock10_timestamp + 600` |
| Fresh vault (testnet one-shot) | `0` | `1` | `80` | `clock10_timestamp + 600` |

After Step 10a seed, vault holding matches `demo_deposit_amount` (default `2400`) and
`total_allocated` is `1800`, so unallocated is `600`; the full-arc allocation `15` stays within solvency.

`createStream` via `chainAction` must use the same `vault_id`, `stream_id`, provider, rate,
and allocation as the persisted proposal.

## Module methods (logoscore)

Same conventions as Step 11a/11b ([`step11a-chain-reads.md`](step11a-chain-reads.md),
[`step11b-chain-writes.md`](step11b-chain-writes.md)): every method returns one compact JSON
`QString`; successes use `"status":"ok"`, failures `"status":"error"` plus `"message"`. Step 12
eligibility errors also include `"code"` (machine-readable enum from the plan).

Logoscore passes each argument as a separate string (Universal `QVariant` → `QString`), same as
`readVaultConfigDecoded <base58>` and `chainAction <op> '<json>'`.

### LogosAPI encoding

| Kind | Encoding |
| --- | --- |
| LEZ account id (owner, provider) | Base58 string, trimmed (fixture ids; same keys as `chainAction` `signer` / `provider`) |
| libp2p `PeerId` | Opaque UTF-8 string (no hex conversion) |
| `vault_id`, `stream_id` | Decimal integer in logoscore argv (e.g. `0`, `1`); parsed as unsigned 64-bit |
| Binary blobs (canonical request, proposal, proof, account `data`) | Lowercase hex, even length, no `0x` prefix (wallet `get_account_public` `data`, `sign_public_payload` digest/result, decode `owner_hex` / `provider_hex`) |
| `chainAction` params | Unchanged compact JSON object per Step 11b |

| Method | Arguments (logoscore order) | Success JSON (extra fields) |
| --- | --- | --- |
| `registerProviderMapping` | `provider_peer_id`, `provider_account_id_base58` | `"status":"ok"` only |
| `prepareEligibilityForStoreQuery` | `canonical_request_hex`, `provider_peer_id` | `"kind"`: `"stream_proposal"` or `"stream_proof"`; `"bytes_hex"`: serialized protobuf `EligibilityProof` ([D1](../reference/decisions-and-notes.md#d1-store-wire-format)); optional `"stream_id"`, `"vault_id"` for demo scripts |
| `listMyStreams` | `vault_id` | `"streams"`: array of per-stream objects (inventory + folded status fields) |
| `rediscoverStreams` | `vault_id` | `"streams"`: array; `"discovered_count"` |

`canonical_request_hex` is the full N8 `canonical_payload` (32-byte domain prefix + Borsh
`CanonicalStoreRequest`), not the SHA-256 digest. The module hashes internally for
`sign_public_payload` / session signing.

Example (values illustrative):

```bash
logoscore call payment_streams_module registerProviderMapping \
  '12D3KooWExamplePeerId' 'A4qQ6pXk4LcojwJsBa3qTRb4KjM17eufHajrRpU3chTY'

logoscore call payment_streams_module prepareEligibilityForStoreQuery \
  '<pinned N8 test vector hex>' '12D3KooWExamplePeerId'
```

Success:

```json
{"status":"ok","kind":"stream_proposal","bytes_hex":"…","stream_id":1,"vault_id":0}
```

Eligibility error:

```json
{"status":"error","code":"NO_ELIGIBLE_VAULT","message":"…"}
```

| Method | Purpose |
| --- | --- |
| `registerProviderMapping` | `PeerId` → LEZ provider account (N5) |
| `prepareEligibilityForStoreQuery` | N8 canonical request + `providerPeerId` → Store `EligibilityProof` bytes |
| `listMyStreams` | Local inventory + folded chain status for `vaultId` |
| `rediscoverStreams` | Recovery scan `stream_id` 0, 1, … on chain |

`listMyStreams` is not a cache-only read: for each id in local inventory it derives the stream
PDA, calls `get_account_public`, decodes and folds at clock-10, and returns current folded
fields (same chain read pattern as the integration plan). `rediscoverStreams` extends inventory
from chain scan only; it does not replace per-entry refresh in `listMyStreams`.

Existing Step 11 methods remain (`read*`, `chainAction`).

### `registerProviderMapping`

Maps libp2p `provider_peer_id` (Store routing) to the LEZ stream payee. Two arguments only
(demo simplicity; a third generic `providerId` is deferred until a non-LEZ binding is needed).

`registerProviderMapping(provider_peer_id, provider_account_id_base58)`

The module converts base58 to 32 bytes and uses that everywhere LIP-155 and LEZ need
`provider_id`: `VaultProof.provider_id`, `createStream` provider binding, persistence and
pending-proposal keys `(vault_id, provider_id)` ([N5](../reference/decisions-and-notes.md#n5-provider-identity-mapping)).

| Name | Source |
| --- | --- |
| `provider_peer_id` | Host / Step 16; lookup key for `prepareEligibilityForStoreQuery` |
| LIP-155 `provider_id` | Derived from `provider_account_id_base58` (same octets as `StreamConfig.provider`) |

One peer maps to one LEZ payee for the MVP. Multiple payees per peer is a later API extension.

### `prepareEligibilityForStoreQuery`

Builds the incentivization envelope, not bare `StreamProposal` / `StreamProof` messages alone.
Per [D2](../reference/decisions-and-notes.md#d2-delivery-module-hook-design), Delivery treats
eligibility as opaque bytes on the provider/verifier hooks: copy `bytes_hex` decoded to
`StoreQueryRequest.eligibility_proof` (tag `30`) without parsing `stream_proposal` or
`stream_proof`. The streams module owns that protobuf shell and the nested LIP-155 messages
(Step 4).

| Output | `EligibilityProof` field set | Nested payload |
| --- | --- | --- |
| Proposal path | `stream_proposal` | Serialized `StreamProposal` (`VaultProof`, `StreamParams`, session `public_key`) |
| Proof path | `stream_proof` | Serialized `StreamProof` (session signature over N8 digest) |

Proposal path: generate session keypair, persist, build `VaultProof` (`sign_public_payload`
on owner), serialize inner message, wrap in `EligibilityProof`, return hex of full protobuf.

Proof path: read/fold `StreamConfig`, sign canonical digest with session key (FFI), wrap in
`EligibilityProof`.

JSON `"kind"` mirrors which arm is set (`stream_proposal` vs `stream_proof`) for demos; wire
bytes are always the full `EligibilityProof`. Step 13 `verifyEligibilityForStoreQuery` receives
the same opaque blob (Delivery does not unwrap).

Inner-only serialization remains valid inside the module and FFI tests; it does not cross the
Delivery hook.

Path selection (no extra error codes): if folded chain state shows an `ACTIVE` stream for
the `(vault_id, provider_id)` pair, return the `stream_proof` arm. Pending non-expired proposal →
`PROPOSAL_PENDING`. Pending past `create_stream_deadline` → evict, return `PROPOSAL_EXPIRED`
(that call does not mint a new proposal); the following call may issue a fresh `stream_proposal`.
Duplicate `createStream` for an existing `stream_id` is a chain/wallet failure, not a new module
code.

Errors (machine-readable `code` in JSON): `UNKNOWN_PROVIDER`, `NO_ELIGIBLE_VAULT`,
`PROPOSAL_PENDING`, `PROPOSAL_EXPIRED`, `STREAM_NOT_CONFIRMED`, `STREAM_DEPLETED`,
`STREAM_PAUSED`, `STREAM_CLOSED`, `WALLET_SIGNING_FAILED`, `CHAIN_READ_FAILED` (plan Step 12).

### Persistence (N4)

Demo policy ([N4](../reference/decisions-and-notes.md#n4-persistence-policy)):

| Item | Value |
| --- | --- |
| File | `payment_streams_state.json` at root of `instancePersistencePath` |
| Format | Single JSON object, `schema_version`: `1`, atomic write (temp + rename) |
| Failure | Log error; continue in-memory only |
| Eviction | No background timer. On load and on each `prepareEligibilityForStoreQuery` / `listMyStreams`, drop pending rows when clock-10 ≥ stored `create_stream_deadline` |
| Clock-10 fold | LEZ clock account timestamp is milliseconds; fold and deadline checks use `ms / 1000` (truncate). See [step-13-normative.md](plan/completed/step-13-normative.md) |
| Session keys | Plaintext `session_private_key_hex` / `session_public_key_hex` (lowercase hex) in JSON; treat instance dir as sensitive |

`provider_id_hex` in negotiations is lowercase hex of the 32-byte LEZ account id (same bytes as
`VaultProof.provider_id`), derived from `provider_account_id_base58` via
`logos_execution_zone.account_id_from_base58`.

Schema version `1` (informative shape):

```json
{
  "schema_version": 1,
  "peer_mappings": {
    "<provider_peer_id>": "<provider_account_id_base58>"
  },
  "negotiations": [
    {
      "vault_id": 0,
      "provider_id_hex": "<64-char hex>",
      "stream_id": 1,
      "status": "pending",
      "create_stream_deadline": 0,
      "session_private_key_hex": "<64-char hex>",
      "session_public_key_hex": "<64-char hex>"
    }
  ],
  "inventory": [
    { "vault_id": 0, "stream_id": 0 }
  ]
}
```

Additional folded snapshots are optional; omit until a consumer needs them.

### Session key generation

Generate and use session keys only through `lez-payment-streams-ffi` (same NSSA stack as
`payment_streams_ffi_sign_canonical_payload_digest` and verify helpers in Step 4). The Qt module
stores hex bytes and calls FFI to sign proofs; it does not implement NSSA in C++.

Vault owner signing stays on the wallet (`sign_public_payload`); session keys are separate from
LEZ account keys.

Step 12 adds keygen in the plan as
[`payment_streams_ffi_generate_session_keypair`](../reference/decisions-and-notes.md#ffi-session-keypair-step-12-deliverable)
(same `payment_streams_ffi_*` family as `payment_streams_ffi_sign_canonical_payload_digest`).
Implement generation in `lez-payment-streams-core`; C ABI in `proof_abi.rs` only (no separate
session ABI module). Refresh `cbindgen` / module C bridge as needed.

### Canonical request bytes (N8)

Step 12 demos and `./scripts/verify-step12-dod.sh` use the same field values via Rust.
Nim parity is covered in Step 15 (`test_store_eligibility_canonical.nim` on the delivery fork).
DoD scripts use a pinned Rust fixture, not a live Store query from `delivery_module`.
Field values match the integration plan N8 cross-language vector and
`store_eligibility_digest_matches_n8_reference_fixture` in
[`lez-payment-streams-core/src/off_chain/canonical.rs`](../lez-payment-streams-core/src/off_chain/canonical.rs).

Pinned wire hex (authoritative for demos and DoD scripts):

```bash
cargo run -q -p lez-payment-streams-core --bin n8_canonical_wire_hex
```

That binary prints lowercase hex of the full N8 `canonical_payload`:
`STORE_ELIGIBILITY_DOMAIN_PREFIX` (32 bytes) concatenated with Borsh(`CanonicalStoreRequest`).
For the reference fixture the wire is 177 bytes (354 hex characters).
`verify-step12-dod.sh` and `step12-topup-and-prepare.sh` invoke this tool.

Pass that full wire to `prepareEligibilityForStoreQuery` as `canonical_request_hex`.
Do not pass base64, the SHA-256 digest alone, or Borsh body without the domain prefix.
Body-only input (~145 bytes) fails in the module FFI path
(`store_eligibility_canonical_payload_digest_from_n8_wire` → `InvalidWireFrame`; sizing helpers
return non-success status such as `Malformed` / status 2).

Cross-check: `cargo run -p lez-payment-streams-core --bin n8_canonical_wire_hex` and the Step 15
Nim test on the delivery fork (see [N8](reference/decisions-and-notes.md#n8-canonical-store-request-bytes-format)).

## Intended demo sequence (logoscore)

Sketch only; see [LogosAPI encoding](#logosapi-encoding) for argument shapes.

```bash
export REPO="$PWD"
export MODULES="$HOME/Downloads/software/waku/lez-related/logos-cli/modules"
export WALLET_CONFIG="$REPO/.scaffold/wallet/wallet_config.json"
export WALLET_STORAGE="$REPO/.scaffold/wallet/storage.json"

# localnet + 10a fixture, tooling shell
logoscore -D -m "$MODULES" -q &
sleep 3
logoscore load-module logos_execution_zone
logoscore load-module payment_streams_module
logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE"

# 1) mapping — use provider PeerId from off-band advertisement (Step 17 E2E writes
#    .scaffold/e2e/provider-advertisement.json). Single-host Step 12 verify may use a
#    placeholder string such as step12-demo-provider-peer when only prepare is tested.
logoscore call payment_streams_module registerProviderMapping \
  '<provider_peer_id>' "$(python3 -c "import json; print(json.load(open('fixtures/localnet.json'))['provider_account_id'])")"

# 2) proposal — canonical_request_hex from pinned N8 vector in verify-step12-dod.sh
# logoscore call payment_streams_module prepareEligibilityForStoreQuery '<hex>' '<provider_peer_id>'

# 3) on-chain stream (manual) — vault_id from manifest; stream_id from proposal inventory
# logoscore call payment_streams_module chainAction createStream '{...}'

# 4) proof — same canonical_request_hex and provider_peer_id as step 2
# logoscore call payment_streams_module prepareEligibilityForStoreQuery '<hex>' '<provider_peer_id>'

logoscore stop
```

## Definition of done

Recovery policy: [`demo-localnet-recovery.md`](demo-localnet-recovery.md). If logoscore smoke
skips `stream_proof` due to `STREAM_DEPLETED`, run `./scripts/demo-localnet-fresh.sh` and retry
with `REQUIRE_STREAM_PROOF=1` (runs top-up then prepare).

```bash
./scripts/verify-step12-dod.sh
```

Checks (from plan Step 12):

- Valid `EligibilityProof` wrapper bytes for pinned inputs; nested proposal/proof passes FFI structural verify.
- `payment_streams_ffi_generate_session_keypair` covered by FFI unit test (sign/verify round-trip).
- Persistence survives module reload (same `instancePersistencePath`).
- `listMyStreams` matches folded status when chain state exists.
- Documented error codes for negative cases.
- Provider cross-test for the same bytes: [`step13-provider-eligibility.md`](step13-provider-eligibility.md)
  and `./scripts/verify-step13-dod.sh` (not a hard gate for Step 12-only CI).

Skip live chain:

```bash
VERIFY_LOGOSCORE=0 ./scripts/verify-step12-dod.sh
```

Strict `stream_proof` on localnet (after `./scripts/demo-localnet-fresh.sh` and wallet `.lgx` reinstall):

```bash
export PAYMENT_STREAMS_GUEST_BIN="$REPO/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
REQUIRE_STREAM_PROOF=1 ./scripts/verify-step12-dod.sh
```

Uses `./scripts/step12-topup-and-prepare.sh` internally (`topUpStream` then `prepareEligibilityForStoreQuery`).
Seeded stream `0` is often fully accrued on LEZ 510 clock units until top-up; that is expected for smoke mode.

If `lgs localnet start` fails with missing `sequencer/service/configs/debug/sequencer_config.json`, run
`./scripts/ensure-scaffold-lez-layout.sh` after `lgs setup` (LEZ 510 nests `sequencer` under `lez/`).

## Runbook notes

Addressed in this runbook: wallet env, `PROPOSAL_EXPIRED` + eviction, N8 pin via
`n8_canonical_wire_hex` (full 177-byte wire), LEZ 510 clock ms normalization in FFI fold,
local recovery [`demo-localnet-recovery.md`](demo-localnet-recovery.md),
`listMyStreams` refresh behavior, stream id allocation, persistence schema v1, keygen in
`proof_abi.rs`, path selection without `STREAM_ALREADY_EXISTS`, strict verify via
`REQUIRE_STREAM_PROOF=1` + `step12-topup-and-prepare.sh`.

Left as-is (simplicity / elsewhere):

- LogosAPI hex for `canonical_request_hex` is in [LogosAPI encoding](#logosapi-encoding).
- `registerProviderMapping` argument naming (routing vs LIP-155 bytes) is intentional.
- No `STREAM_ALREADY_EXISTS` code: active stream → proof path; pending → `PROPOSAL_PENDING`.
- Step 13 provider cross-test: [`step13-provider-eligibility.md`](step13-provider-eligibility.md)
  (`VERIFY_LOGOSCORE=1 ./scripts/verify-step13-dod.sh`); not required for Step 12 DoD.
