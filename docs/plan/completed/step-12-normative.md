# Step 12 — normative plan excerpt

Operator commands: [step12-user-eligibility.md](../step12-user-eligibility.md).
This file keeps closed design choices and DoD detail for audits; agents on Step 14+ should read [integration-contracts.md](../reference/integration-contracts.md) instead.

### Step 12, Session keys and user-side proof construction

Architectural context:
this is the user-side method that `delivery_module` will auto-invoke
once registered as the outbound eligibility provider in Step 16.
It does not, by itself, initiate any Store traffic;
it just produces opaque bytes when asked.
Requires Step 11c (`sign_public_payload`) and the Step 11a read path;
user flows that open streams on-chain use Step 11b (`chainAction` / `createStream`).

Runbook (demo vault, API encoding):
[`docs/step12-user-eligibility.md`](docs/step12-user-eligibility.md).
Local fixture age and reset policy:
[`docs/demo-localnet-recovery.md`](docs/demo-localnet-recovery.md).

#### Status (implementation)

Step 12 is complete for this integration plan:

- Feature: session keygen FFI, `EligibilityProof` wrapper serialize, module methods
  (`registerProviderMapping`, `prepareEligibilityForStoreQuery`, `listMyStreams`,
  `rediscoverStreams`), N4 persistence, N8 tool (`n8_canonical_wire_hex`).
- Verify: `make verify-step12` (offline + logoscore); strict `stream_proof` via
  `REQUIRE_STREAM_PROOF=1` and `./scripts/step12-topup-and-prepare.sh` after Step 11d wallet stack.
- Runbooks: [`docs/step12-user-eligibility.md`](docs/step12-user-eligibility.md),
  [`docs/demo-localnet-recovery.md`](docs/demo-localnet-recovery.md).

Not in Step 12 scope: Step 16 `delivery_module` auto-invoke; Step 13 provider verifier cross-test
(lives in `make verify-step13`, not Step 12 DoD); full Step 17 demo without top-up
helper on aged stream `0`.

#### Quick reference

| Method | Purpose | Called by |
|--------|---------|-----------|
| `prepareEligibilityForStoreQuery` | Returns serialized `EligibilityProof` (stream proposal or proof arm) | `delivery_module` (auto) |
| `registerProviderMapping` | Maps `PeerId` to LEZ payee account (base58) | Host application |
| `listMyStreams` | Lists streams for a vault | Host application |
| `rediscoverStreams` | Re-enumerates streams from chain | Host application (recovery) |

#### User-side flow

The intended sequence for a new provider relationship is:

1. Host application calls `registerProviderMapping`
   to bind the provider's libp2p `PeerId`
   to its LEZ stream payee account (base58; module derives LIP-155 `provider_id` bytes per N5).
2. User issues a Store query.
   `delivery_module` invokes `prepareEligibilityForStoreQuery`.
   The module has no established stream for this `(vault, provider)` pair,
   so it generates a session keypair, persists it,
   and returns an `EligibilityProof` byte string (opaque to Delivery; `stream_proposal` arm).
3. Provider accepts the proposal and serves the first request.
4. User explicitly calls `chainAction` with operation `createStream`
   (Step 11b) to open the stream on-chain.
   This is a manual action by the host application or demo script,
   never triggered automatically by any hook.
5. User issues the next Store query.
   `delivery_module` invokes `prepareEligibilityForStoreQuery` again.
   The module queries `get_account_public` for the `StreamConfig` PDA,
   confirms it exists and is `ACTIVE`,
   and returns an `EligibilityProof` byte string (`stream_proof` arm).

#### Session and stream state management

Add session-keypair management inside `payment_streams_module`,
backed by `payment_streams_state.json` in `instancePersistencePath` (see [N4](#n4-persistence-policy)).
Generate session keypairs via `lez-payment-streams-ffi` (see [FFI session keypair](#ffi-session-keypair-step-12-deliverable));
sign proofs with existing Step 4 FFI helpers. Persist keys as plaintext hex for the demo.
The persisted state per `(vault_id, provider_id)` includes:
the `stream_id` (allocated locally, used as the PDA seed on-chain),
the session keypair,
the proposal status (pending, established, expired),
and the last known on-chain stream state.

The module maintains a local inventory of stream IDs per vault.
Every `create_stream` call records the new `stream_id` in the inventory.
This inventory is the backing store for `listMyStreams`.
Stale proposals are evicted on cold start and on eligibility/inventory API calls (deadline vs clock-10).

#### FFI session keypair (Step 12 deliverable)

Step 4 exports sign/verify with caller-supplied 32-byte NSSA secrets only; it does not generate
session keypairs. Implemented in `payment_streams_ffi_*` naming family as
`payment_streams_ffi_sign_canonical_payload_digest` (`lez-payment-streams-ffi/src/proof_abi.rs`;
core logic in `lez-payment-streams-core`).

Add in `lez-payment-streams-core` (unit-tested) and expose:

```c
PaymentStreamsFfiStatus payment_streams_ffi_generate_session_keypair(
    uint8_t *out_secret_key_32,
    uint8_t *out_public_key_32);
```

Both outputs are 32 bytes (NSSA `PrivateKey` / public key bytes used elsewhere in proof FFI).
Use a CSPRNG; return `PaymentStreamsFfiStatus` on null pointers or generation failure. Regenerate
`cbindgen` output (`lez_payment_streams_ffi.h`); wire through `payment_streams_ffi_bridge` if the
Qt module calls via the existing C bridge pattern.

Step 12 definition of done includes a Rust unit test in `proof_abi.rs`: generate, sign a digest
with `payment_streams_ffi_sign_canonical_payload_digest`, verify with
`payment_streams_ffi_verify_canonical_payload_digest`.

#### Exposed methods

`prepareEligibilityForStoreQuery(canonicalRequestBytes, providerPeerId) -> QString`
LogosAPI passes `canonicalRequestBytes` as lowercase hex of the N8 `canonical_payload` (see
runbook). Returns compact JSON whose `bytes_hex` is the serialized protobuf `EligibilityProof` for
Store tag `30` (D1, D2). Set `stream_proposal` or `stream_proof` (mutually exclusive) with
nested serialized `StreamProposal` or `StreamProof` per LIP-155, depending on whether the
stream for the `(vault, provider)` pair has been established on-chain.
Before returning a `StreamProof`,
the module reads the `StreamConfig` PDA via `get_account_public`,
decodes it through the FFI,
folds it at the current clock time,
and checks that the effective state is `ACTIVE`.
For `StreamProposal` output,
the module calls `logos_execution_zone.account_id_from_base58` to convert
the configured vault owner base58 account ID to hex,
then asks `logos_execution_zone.sign_public_payload(account_id_hex, digest_hex)`
to produce `VaultProof.owner_signature` with the vault owner's LEZ key,
and reads the 64-byte signature from the `result` field of the JSON response.
Later `StreamProof`s are signed with the persisted session key.

`registerProviderMapping(providerPeerId, providerAccountId) -> QString`
lets the host configure the identity mapping (see [N5](#n5-provider-identity-mapping)).
Returns compact JSON (`status` ok/error). `providerAccountId` is base58; the module derives
32-byte `provider_id` for proofs and persistence.

`listMyStreams(vaultId) -> QString`
returns a JSON array of stream statuses
for all locally known streams belonging to the given vault.
For each stream in the local inventory,
the module derives the `StreamConfig` PDA,
reads it via `get_account_public`,
decodes and folds to the current clock time,
and returns the typed status.

`rediscoverStreams(vaultId) -> QString`
re-enumerates streams from the chain
by deriving PDA addresses for `stream_id = 0, 1, 2, ...` sequentially,
reading each via `get_account_public`,
and stopping when an uninitialized account is encountered.
Discovered streams are added to the local inventory.
This is a recovery path for cold-start or persistence-loss scenarios.
For the MVP demo, `listMyStreams` is the primary query path.

#### User-side error conditions

`prepareEligibilityForStoreQuery` returns a structured error
in each of the following cases.
The error string includes a machine-readable code
and a human-readable description.

- `UNKNOWN_PROVIDER`:
  `providerPeerId` not registered via `registerProviderMapping`.
- `NO_ELIGIBLE_VAULT`:
  no vault configured or no vault with sufficient unallocated balance.
- `PROPOSAL_PENDING`:
  a `StreamProposal` for this `(vault_id, provider_id)` pair
  was already issued and has not expired or been resolved.
  User must wait for expiry or call `create_stream`.
- `PROPOSAL_EXPIRED`:
  the pending proposal's `create_stream_deadline` has passed
  without stream creation.
  The module evicts the stale proposal and returns this error on that call;
  a subsequent call may issue a fresh `StreamProposal`.
- `STREAM_NOT_CONFIRMED`:
  user called `create_stream` but the `StreamConfig` PDA
  does not yet exist on-chain.
  User should retry after a short delay.
- `STREAM_DEPLETED`:
  folded stream state shows allocation fully accrued (unaccrued is zero).
  User must top up or close.
- `STREAM_PAUSED`:
  stream is paused (user-initiated).
  User must resume before querying.
- `STREAM_CLOSED`:
  stream has been closed (by user or provider).
  User must open a new stream to this provider.
- When chain state is `ACTIVE` for the `(vault_id, provider_id)` pair,
  `prepareEligibilityForStoreQuery` returns a `stream_proof` (not an error).
  Duplicate on-chain `createStream` for an occupied `stream_id` is rejected by the chain, not a
  separate module error code.
- `WALLET_SIGNING_FAILED`:
  `sign_public_payload` returned `{"status":"error",...}` or IPC failed.
  Error includes upstream details from the `error` field.
- `CHAIN_READ_FAILED`:
  `get_account_public` call failed.
  Error includes upstream details.

#### Components required to run

`logoscore` daemon hosting both modules.
The definition of done's verifier round-trip is in-process through the FFI;
a live sequencer is not strictly required for that verification itself,
but the same Steps 10a–11b stack remains useful for sanity-checking
that vault data the proof asserts matches chain state.
After code changes, rebuild and reload via
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 3).

#### Definition of done

Landed (current tree):

1. `make verify-step12` with `VERIFY_LOGOSCORE=0` exits 0: N8 digest test, FFI
   session keygen + eligibility wrapper tests, N8 wire tool, installed module plugin, four
   Step 12 methods in `lm methods`.
2. With `VERIFY_LOGOSCORE=1` and `REQUIRE_STREAM_PROOF=1`: logoscore path via
   `step12-topup-and-prepare.sh` (register, `topUpStream`, `stream_proof` prepare) and
   persistence under `--persistence-path`. Default `REQUIRE_STREAM_PROOF=0` may SKIP prepare when
   stream `0` is depleted on an aged localnet (see recovery doc).
3. Runbook [`docs/step12-user-eligibility.md`](docs/step12-user-eligibility.md) matches API and
   error codes.

Product criteria (unchanged):

The module produces a syntactically valid eligibility proof byte string
for fixed inputs;
`payment_streams_ffi_generate_session_keypair` is implemented and covered by FFI tests;
restarts cleanly with state intact;
the FFI structural verifier accepts the proof format;
`listMyStreams` returns correct folded status for locally known streams;
each user-side error condition returns the documented error code;
and (when chain state is available) the provider-side verifier accepts
the proof against actual on-chain stream state (Step 13; recommended cross-test).

#### Verification (Step 11d follow-up — landed)

After the Step 11d wallet pin bump:

1. `make verify-step12` supports `REQUIRE_STREAM_PROOF=1` (top-up + prepare via
   `step12-topup-and-prepare.sh`). Default logoscore smoke allows SKIP on depleted stream when
   `REQUIRE_STREAM_PROOF=0`.
2. Demo scripts document `PAYMENT_STREAMS_GUEST_BIN`, `ensure-scaffold-lez-layout.sh`, and
   `REINIT_WALLET=1` recovery; `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF` stays emergency-only.
3. FFI fold normalizes LEZ 510+ millisecond clock timestamps to seconds for accrual checks.

CI may keep `VERIFY_LOGOSCORE=0`; local strict checks use
[`docs/demo-localnet-recovery.md`](docs/demo-localnet-recovery.md) and `REQUIRE_STREAM_PROOF=1`.

