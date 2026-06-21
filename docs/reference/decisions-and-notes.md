# Decisions and notes

Normative decisions (D1–D6) and carry-forward notes (N1–N12) for payment-streams integration.
Index: [integration-index.md](../../integration-index.md). Cross-step APIs: [integration-contracts.md](../integration-contracts.md).
Plan excerpts: [plan/README.md](../plan/README.md).

## Decisions and Notes

Items resolved before implementation starts,
plus non-blocking notes carried forward into the demo.

### D1, Store wire format

RFC 73 defines the general pattern: eligibility proof on the request, eligibility status on
the response, with extensible proof types.
Spec: [RFC 73](https://rfc.vac.dev/spec/73/) (general incentivization pattern; this demo applies
it to Store at tag `30` with LIP-155 bytes).
This demo binds that pattern to Store RPC at tag `30`: opaque LIP-155 payment-stream
`EligibilityProof` bytes on the request and a nested payment-stream verdict object on the
response.
That is a third concrete proof flavor relative to RFC 73’s examples (for example
proof-of-payment TXID, optional membership bytes), specified here via LIP-155 rather than by
extending the old `waku/incentivization/` POC in-tree.

Add an optional opaque `eligibility_proof` field on `StoreQueryRequest`
and an optional opaque `eligibility_status` object on `StoreQueryResponse`.
Tags currently used on the request are `1`, `2`, `10`, `11`, `12`, `13`, `20`, `51`, `52`, `53`.
The new fields take tags from a fresh block starting at `30`,
i.e. `eligibility_proof` at tag `30` on the request
and `eligibility_status` at tag `30` on the response.

Eligibility semantics are fully hidden from Store.
Store status codes (`status_code`, `status_desc`) remain reserved for query-execution outcomes.
Eligibility outcomes use a separate enumeration carried exclusively inside `eligibility_status`.
If eligibility fails,
the Store handler returns a `StoreQueryResponse`
with the existing `BAD_REQUEST` status code (`400`)
and an empty `messages` list,
skipping the database query.
The detailed eligibility verdict
(`OK`, `PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`)
lives only inside the `eligibility_status` object at tag `30`.
Those four codes are the Store-visible vocabulary; multiple distinct
`PolicyRejectReason` / FFI discriminant failures may map into `PARAMS_REJECTED`
on the wire (see Step 3a mapping table).
Store never interprets these values;
the payment-streams module on each side
reads and acts on the structured `eligibility_status` payload.
The `eligibility_proof` field carries a protobuf `EligibilityProof`.
`stream_proposal` and `stream_proof` contain serialized
`StreamProposal` or `StreamProof` messages (see LIP-155 LEZ integration).
No new status codes are added to the Store `StatusCode` enum.

No protocol-ID version bump and no codec migration is required for the demo.
Confirm tag `30` is unused in `logos_delivery/waku/waku_store/rpc_codec.nim` before implementation.

Step 14 implementation uses `logos_delivery/waku/waku_store/common.nim` and `rpc_codec.nim` only.
`waku/incentivization/` (RFC 73–linked proof-of-payment POC) is unrelated wire and must not
be imported for Store tag `30`.

### D2, Delivery module hook design

`liblogosdelivery` gains two C ABI registration entry points:
`logosdelivery_set_eligibility_verifier` (inbound Store requests)
and `logosdelivery_set_eligibility_provider` (outbound Store queries, symmetric).
Both take synchronous blocking C function pointers per N3.
The verifier is invoked for every inbound Store request;
`proof_hex` is NULL when no proof field is present,
giving the callback full control over whether to accept unauthenticated requests.
There is no `eligibilityRequired` flag; gating policy is expressed in the registered module
after the Step 16 bridge forwards the hook (see [N3c](#n3c-inbound-missing-proof-null-proof_hex-2025-06-18)).
`logos-delivery-module` (our fork for eligibility hooks) gains
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`,
and adds a `storeQuery(...)` LogosAPI method backed by `logosdelivery_store_query`
(added on our fork of `logos-delivery`; does not depend on upstream N6).
The bridge validates the named module's surface at registration time
via the auto-generated `getPluginMethods` introspection.
Both layers stay payment-streams-agnostic.
On the eligibility hooks, opaque bytes mean the complete serialized protobuf `EligibilityProof`
(D1) as carried on `StoreQueryRequest` tag `30`: the registered module produces and parses that
blob; Delivery forwards it without interpreting `stream_proposal`, `stream_proof`, or other
extension fields. Future eligibility mechanisms can register a different module or populate
other `EligibilityProof` arms without changing that hook contract.
For outbound Store queries,
`delivery_module` passes the target provider's libp2p `PeerId`
as `providerPeerId` to the eligibility provider hook.
For inbound Store requests,
it passes the caller's libp2p `PeerId`
as `requesterPeerId` to the eligibility verifier hook.
It does not interpret either value or know about LEZ account IDs.
We ship this on our branches and do not negotiate spec changes.

Step 16 bridge and verification (normative): threading and async `storeQuery` ([N3a](#n3a-step-16-threading--approach-a-experiment-2025-06-18)),
hook lifecycle ([N3b](#n3b-step-16-hook-registration-lifecycle-2025-06-18)), NULL inbound proof ([N3c](#n3c-inbound-missing-proof-null-proof_hex-2025-06-18)),
scope vs Step 17 E2E ([N12](#n12-step-16-vs-step-17-verification-scope-2025-06-18)).
Agent summary table: [step-16.md](../plan/completed/step-16.md#resolved-implementation-decisions-2025-06-18).

Inbound `eligibility_status.desc` on the Store wire (D1) is filled by `liblogosdelivery`
after the verifier callback returns:

- If the callback writes a non-empty UTF-8 string into `out_desc`, that string is used
  (Step 16 copies `verifyEligibilityForStoreQuery` JSON `message` here on verdict failures).
- Otherwise `liblogosdelivery` uses the default phrase for the returned
  `EligibilityStatusCode` (same short strings as Step 14 normative:
  `"ok"`, `"proof invalid"`, `"stream not active"`, and `"params rejected"` for
  `PARAMS_REJECTED`). Callback return `-1` maps to `PROOF_INVALID` with default
  `"proof invalid"`.

`liblogosdelivery.h` has no separate ABI version symbol today; “bump the ABI” on our fork
means additive C exports in that header plus rebuilding `liblogosdelivery` and every
consumer (`logos-delivery-module`, C smoke tests). Do not remove or change existing
`logosdelivery_*` signatures without coordinating downstream.

### D3, Wallet write path

Reproducible flake refs and pin maintenance:
[`docs/feature-branch-pins.md`](docs/feature-branch-pins.md).
This section records integration intent; pin tables live in that doc only.

`payment_streams_module` chain writes go through `logos_execution_zone`, which delegates
to the generic public transaction APIs in `wallet_ffi`.

#### LEZ FFI — PR 491 (canonical)

Generic public transactions on LEZ `main` ([491 merged](https://github.com/logos-blockchain/logos-execution-zone/pull/491)).
Deprecated 429/16 wallet JSON path: [`docs/archive/superseded-wallet-pr-429-16.md`](docs/archive/superseded-wallet-pr-429-16.md).

491 exposes (among others):

- `wallet_ffi_resolve_public_account` — map each 32-byte account id to `FfiAccountIdentity`
  (signer vs read-only via `needs_sign`).
- `wallet_ffi_serialization_helper` — Borsh instruction bytes to RISC0 `u32` instruction words
  (same wire step as `lez-payment-streams-core` / `instruction_wire.rs`).
- `wallet_ffi_send_generic_public_transaction` — submit using ordered account identities,
  instruction words, and `FfiProgramWithDependencies` (guest program ELF plus dependency ELFs).

Private/generic PP paths exist in 491 but are out of scope for the MVP transparent vault demo.

#### Wallet module Qt surface — PR 19 (primary)

Upstream exposes 491 to Logos modules via
[logos-execution-zone-module PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19)
(`feat: general transactions flow`, branch `Pravdyvy/generic-transactions-extension`).
Same author and timeline as PR 491.

Primary path: pin and build the patched wallet wrapper against PR 19 head + LEZ `main`
(510 merge; includes 491 generic public tx — see [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md)).
Step 11b submits through PR 19 `send_generic_public_transaction` in the wallet; the Universal
module uses a repo-specific `send_generic_public_transaction_json` IPC helper (N10). Read PR 19
for the underlying QList request shape.

Our wallet work (Steps 10b and 11c, reduced scope):

- Step 10b: packaging — patched wrapper flake pins PR 491 + PR 19; CMake `wallet_ffi.h` include,
  codegen headers for dependents, `.lgx` bundle. Module id stays upstream `logos_execution_zone`.
  Do not reimplement generic public send if PR 19 already does. Step 11b adds guest-ELF-from-env and
  JSON submit patches on the same wrapper (see N10).
- Step 11c: `sign_public_payload` per [N1](#n1-off-chain-canonical-payload-signing) — not in
  491 or 19; add on our patched wrapper (LEZ FFI + Qt) until upstream ships it.

Do not pin [PR 429 / PR 16](docs/archive/superseded-wallet-pr-429-16.md).

#### Pinning

Pin `logos-execution-zone` to `main` at the current LEZ revision in
[`docs/feature-branch-pins.md`](docs/feature-branch-pins.md) (510 merge after Step 11d) and the wallet module upstream
input to `refs/pull/19/head` until PR 19 merges; then pin `main` on the wallet module repo.
LEZ is no longer pinned to `refs/pull/491/head` in this integration.

### D4, Wallet module runtime name

Use the upstream PR 19 Logos module id `logos_execution_zone`
(`metadata.json` `name`, `LogosExecutionZoneWalletModule::name()`,
plugin `logos_execution_zone_plugin`). The payment-streams wrapper flake adds
behavioral patches only (guest ELF from env, JSON submit helper, future
`sign_public_payload`); it does not rename the module (see Step 7).
Universal `payment_streams_module` does not list the wallet in `metadata.json`
(D6); load wallet before payment streams at runtime.
`logos-rln-module` may still call a wallet plugin registered under the historical
id `liblogos_execution_zone_wallet_module`.
That is unrelated to the payment-streams demo, which installs `logos_execution_zone`
from the patched wrapper (D4).

### D5, New module naming

Metadata `name` is `payment_streams_module`.
Plugin binary stem is `payment_streams_module_plugin`.
Repository directory is `logos-payment-streams-module`,
placed as a sibling of `lez-payment-streams-core/` and `lez-payment-streams-ffi/`
inside the existing `lez-payment-streams` repo
(mirroring `logos-lez-rln`,
which co-locates `lez-rln/` and `logos-rln-module/`).

Justification.
The metadata name follows the protocol-named convention of `delivery_module`:
the module speaks the LIP-155 payment-streams protocol,
with the LEZ-specific bits behind its FFI.
The wallet module keeps upstream PR 19 naming (`logos_execution_zone`), not a separate
`lez_`-prefixed id in this integration.
Snake_case in the metadata name plus a `-module` kebab-case suffix on the directory
matches every existing module in the ecosystem.
Co-locating the module with the SPEL program and the FFI crate
keeps the demo versioned as one unit and matches the `logos-lez-rln` precedent.
Extracting into a separate repository remains an option after the demo stabilises.

### D6, Universal module interface

Build `payment_streams_module` with `"interface": "universal"` and
`LogosModuleContext` (`payment_streams_module_impl.{h,cpp}`).
Do not restore the Legacy `PluginInterface` shell.
Archived bootstrap notes live under `docs/archive/`.

Call `logos_execution_zone` at runtime via
`modules().api->getClient("logos_execution_zone")->invokeRemoteMethod(...)`.
Keep `"dependencies": []`.
Step 8 validated Universal caller to Legacy callee.

Justification.
Universal static dependencies exist so codegen can emit typed `modules().<name>` wrappers.
That assumes every dependency is Universal.
`logos_execution_zone` is still Legacy, so listing it in `metadata.json` would not produce safe typed calls to its `Q_INVOKABLE` API.
Dynamic access keeps payment streams on the Universal side (with `delivery_module`) while the wallet stays Legacy.
We rely on explicit load order and runtime errors if the wallet is absent.
Revisit a static dependency when the wallet module is Universal upstream and codegen supports it.

### N1, Off-chain canonical-payload signing

Neither `wallet_ffi` nor `logos_execution_zone` currently exposes
a primitive that signs an arbitrary canonical payload with a wallet account's key.
That primitive is required for `VaultProof.owner_signature`,
because the vault proof must prove control of the LEZ vault owner key.
For the MVP, we add `sign_public_payload` to `logos_execution_zone`
on our branch (see Step 11c; patch delivery uses `lez-wallet-ffi-patched`).

Decided call convention (Step 11c):
`sign_public_payload(account_id_hex: QString, digest_hex: QString) -> QString`
where `account_id_hex` is the 64-char hex account ID (same format as
`get_account_public` and `get_public_account_key`; convert base58 with
`account_id_from_base58` first if needed),
`digest_hex` is the 64-char hex SHA-256 digest (32 bytes),
and the return is a JSON envelope `{"status":"ok","result":"<128-char hex>"}`.
Verification uses `smoke_verify` from `lez-payment-streams-ffi`
(same `verify_canonical_payload_digest` path as the Step 13 provider FFI).

No domain parameter is included.
The NSSA wallet avoids exposing generic signing entirely
(`wallet_ffi` only exposes complete transaction workflows;
see `logos-execution-zone/wallet-ffi/src/transfer.rs`).
Our method introduces the first generic signing endpoint,
but any co-hosted module can already submit arbitrary transactions
via `send_public_transaction`, which is strictly more powerful.
Domain separation would not reduce the attack surface
in the current trust model.
The payment-streams FFI already builds a domain-prefixed `canonical_payload`
and hashes it to a 32-byte `canonical_payload_digest` for signing
(see [N8](#n8-canonical-store-request-bytes-format)).
If the ecosystem later introduces a module permission model,
domain separation on signing should be revisited.

Payment-stream proofs use NSSA's existing transparent signature contract:
32-byte x-only secp256k1 public keys,
64-byte Schnorr signatures,
and signatures over a 32-byte `canonical_payload_digest`
(SHA-256 of that `canonical_payload`).
This matches the `nssa::PublicKey` and `nssa::Signature` types
already used for public and privacy-preserving LEZ transactions.

`VaultProof.owner_signature` is signed by the LEZ vault owner key.
The verifier checks that the provided owner public key derives to
the owner account stored in `VaultConfig`,
then verifies the signature over the canonical vault-proof payload.
That payload covers the vault proof fields,
the proposed stream parameters,
and the `StreamProposal.public_key`,
so the owner authorizes this exact proposal and session key.

`StreamProposal.public_key` is a module-generated session public key.
`payment_streams_module` owns the matching session private key
and persists it in `instancePersistencePath`.
`StreamProof.signature` is signed by that session key over
the canonical Store request payload.
Session-key persistence policy is covered in N4.

### N2, Read freshness from the wallet module

`get_account_public` returns whatever state the underlying wallet client returns,
without distinguishing live sequencer state from indexer-finalized state.
The MVP treats it as authoritative for hot-path eligibility verification,
which is acceptable on a local sequencer where finality lag is small.

### N3, Provider-side verification latency and blocking hooks

Routing eligibility verification from Nim through `liblogosdelivery` to `delivery_module`
to `payment_streams_module` adds two IPC hops per Store request,
plus wallet-module chain reads inside the verifier.
The MVP accepts that cost.

Step 15 fixes the hook contract at the C ABI (see [D2](#d2-delivery-module-hook-design)):

- `EligibilityVerifierCb` and `EligibilityProviderCb` are synchronous blocking
  function pointers. The registered implementation runs to completion before the call returns.
- On the inbound Store path, `liblogosdelivery` invokes the verifier from its async Store
  handler and awaits the hook; the handler thread is held until the C callback returns.
  Implementations must not re-enter `liblogosdelivery` from inside the callback.
- Step 16’s `delivery_module` bridge performs synchronous `LogosAPI` calls into the named
  eligibility module from inside those callbacks where threading allows (see
  [N3a](#n3a-step-16-threading--approach-a-experiment-2025-06-18)).

That matches the existing Store handler shape (one eligibility decision per request before
the inner `requestHandler` runs) without introducing a second async completion channel on
the MVP ABI. Production traffic that needs non-blocking hooks uses the versioned async ABI
described in the Step 15 migration note (`result_cb` trailing parameters), not N3’s MVP path.

### N3a, Step 16 threading — Approach A experiment (2025-06-18)

Chosen bridge shape: store `LogosAPI` / `LogosModules` from universal `onInit` and invoke
`callModule` directly from the Step 15 C trampolines (no custom Qt hop inside
`delivery_module` beyond what the C++ SDK already does).

Verified with `logos-delivery-module` unit tests
`tests/test_approach_a_thread_probe.cpp` (mirrors `logos-cpp-sdk`
`logos_thread_marshal.h` / `LogosAPI::callModule`):

- When the module owner thread blocks on a semaphore the way `callApiRetValue` does
  without calling `QCoreApplication::processEvents`, a worker thread that invokes
  `runOnOwnerThread` (`Qt::BlockingQueuedConnection`) does not complete within 400ms
  (deadlock class).
- The same worker call completes in milliseconds if the owner thread pumps Qt events
  while waiting.

Implications for Step 16:

- Inbound verifier (`EligibilityVerifierCb` from liblogosdelivery’s Store handler):
  the hook runs on liblogosdelivery’s chronos/async thread while the module owner thread is
  not held on the store-query semaphore. Direct `callModule` from the trampoline is
  feasible (SDK marshals onto the owner thread; the host process event loop can run).
- Outbound `storeQuery` with a registered provider: sync
  `callApiRetValue` plus `logosdelivery_store_query` blocks the same owner thread that
  must service `runOnOwnerThread` for `prepareEligibilityForStoreQuery`. Direct
  `callModule` inside the provider C callback is not feasible on that sync path.
- Mitigation for outbound (still Approach A): implement `storeQuery` like `start` /
  `stop` — return once the FFI call is dispatched and complete via a typed event (or another
  non-blocking completion channel). Do not rely on pumping `processEvents` in the wait loop
  (fragile; not recommended for production).

Do not re-enter `liblogosdelivery` from inside eligibility callbacks (unchanged from N3).

### N3b, Step 16 hook registration lifecycle (2025-06-18)

Normative policy for `setEligibilityVerifier` / `setEligibilityProvider` in
`logos-delivery-module` (implements Step 16 DoD and Step 15 `NULL` clear semantics).

- Require `createNode` before any set call (same gate as `send` / `subscribe`).
- Use fixed C trampolines on `DeliveryModuleImpl` with `user_data = this`, mirroring
  `logosdelivery_set_event_callback` after `createNode`.
- Trampolines read the current target module name and `LogosAPI` / `callModule` state at
  invoke time.
- Call `logosdelivery_set_eligibility_verifier` / `logosdelivery_set_eligibility_provider`
  when enabling (non-empty module name after introspection succeeds) and pass `NULL` cb when
  clearing, so Nim removes the inbound handler wrapper and skips outbound provider attach
  when disabled.
- When changing the module name while already enabled, update C++ state only; do not
  re-register FFI if the same trampoline pointer is already installed.
- Before any successful registration change, call the target module’s `getPluginMethods`
  (or equivalent introspection) and require
  `verifyEligibilityForStoreQuery` / `prepareEligibilityForStoreQuery` as applicable.
  On failure, return a structured error and leave the previous registration unchanged.
- Verifier and provider registrations are independent (two LogosAPI methods, two FFI hooks).
- Before `logosdelivery_destroy`, clear both hooks with `NULL` when `deliveryCtx` is set.
- Do not auto-clear registration when the target module unloads; MVP failures surface at
  `callModule` time.

### N3c, Inbound missing proof (NULL proof_hex) (2025-06-18)

When Step 15 calls the verifier with `proof_hex == NULL` (no tag-30 proof on the Store request),
Step 16’s bridge does not short-circuit in `delivery_module`. It always delegates to
`verifyEligibilityForStoreQuery` with empty lowercase hex for `proofBytes` (same encoding
rules as Step 12–13), plus `canonicalRequestBytes` and `requesterPeerId`.

Eligibility policy for unauthenticated Store requests lives in the registered module, not in
Delivery. For the LIP-155 paid Store demo, `payment_streams_module` rejects empty proof with a
verdict failure JSON (`status: error`, non-OK `eligibility`, and `message`), which the bridge
maps to the C status code and `out_desc` per integration contracts.

Inbound wire semantics and Step 15 wrapper behavior are unchanged: a module OK allows the
inner Store handler to run; a module verdict failure yields Store 400 with
`eligibility_status`.

### N12, Step 16 vs Step 17 verification scope (2025-06-18)

| Step | Repo / focus | Prove |
| --- | --- | --- |
| 16 | `logos-delivery-module` | Eligibility bridge, hook lifecycle (N3b), threading (N3a), async `storeQuery`, registration introspection, unit mocks and logoscore registration checks. |
| 17 | Demo script + two hosts | Full stack: sequencer, wallet, both `logoscore` instances, relay and Store archive, paid outbound query, inbound verify including failed eligibility on the wire, structured log artifact. |

Do not block Step 16 merge on the Step 17 script. Step 17 is the integration gate for
end-to-end Store and eligibility outcomes described in the former monolithic Step 16 DoD.

### N13, Step 17 `liblogosdelivery` bundle vs local overlay (2026-06-18)

Step 17 installs `delivery_module` via `nix build …#lgx` and `lgpm install` (plugin plus bundled
`liblogosdelivery.so` and runtime deps from the module flake). Paid Store E2E failed when only
the plugin was refreshed while an older `liblogosdelivery.so` remained, and also when the locked
nix library still contained a bug in
`logosdelivery_store_query` that cleared `eligibilityProof` immediately after JSON parse
(`storeQueryRequest.eligibilityProof = none(seq[byte])`), so outbound queries never carried
tag-30 proof and the provider verifier saw empty `proofBytes`.

Symptoms on the provider: inbound eligibility hook with `proof_len=0`, verify JSON
`proofBytes and canonicalRequestBytes must be non-empty even-length hex`, Store response
`BAD_REQUEST` (400). Direct `logoscore call payment_streams_module verifyEligibilityForStoreQuery`
on the same host still passed because proof and N8 were supplied on the CLI path.

Mitigations (demo script [`scripts/demo-e2e-local.sh`](../scripts/demo-e2e-local.sh)):

- Default: after `lgpm install` of the delivery `.lgx`, optionally overlay
  `make liblogosdelivery` from sibling `logos-delivery` (`LOGOS_DELIVERY_ROOT`) unless
  `SKIP_LIBLOGOSDELIVERY_OVERLAY=1`.
- Hermetic path: set `SKIP_LIBLOGOSDELIVERY_OVERLAY=1`, use `DELIVERY_MODULE_ROOT` with
  `flake.lock` at `logos-delivery` ≥ `39b467ec`; see
  [step17-e2e-local.md](../step17-e2e-local.md#hermetic-run-hand-off).

Clean nix-only path: push the `logos-delivery` fix (retain JSON `eligibilityProofHex` through
`logosdelivery_store_query`), run `nix flake update logos-delivery` in `logos-delivery-module`,
commit `flake.lock`, update [feature-branch-pins.md](../feature-branch-pins.md), re-run E2E with
`SKIP_LIBLOGOSDELIVERY_OVERLAY=1`, then drop the default overlay from the script when stable.

Inbound bridge (`delivery_module` → `payment_streams_module` during Store handling) must invoke
`LogosAPIClient` on the client object's thread (`runOnOwnerThread`); see [N3a](#n3a-step-16-threading--approach-a-experiment-2025-06-18).

### N14, Step 17 paid-query verify rejects (2026-06-19)

After the N13 wiring was correct (proof carried tag-30, provider verifier reached), the happy-path
`storeQuery` still returned client-visible `BAD_REQUEST`. The client only ever sees `BAD_REQUEST`
on a non-OK verdict, so the orchestrator now calls
`payment_streams_module verifyEligibilityForStoreQuery` directly on the provider and writes the
real verdict to a `store_query_eligibility_verdict` artifact line (`eligibility` + `message`).
Policy rejects carry `reject_reason=N` (the FFI `PolicyRejectReason` discriminant) in the message.

Three root causes surfaced through that observability, in order:

- `STREAM_NOT_ACTIVE` / stream depleted. The seed fixture allocated `400` at rate `1`
  (≈400 s), so the stream depleted before or during a run. Fixed by sizing the seed to deposit
  `2400` / allocation `1800` / rate `1` (≈30 min runway) in
  [`scripts/seed-localnet-fixture.sh`](../../scripts/seed-localnet-fixture.sh). The depletion
  bypass `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF` is now genuinely unnecessary for the demo;
  its check was also made symmetric across prepare and verify with explicit truthy parsing
  (`1`/`true`/`yes`/`on`).
- `PROOF_INVALID` / session public key unknown. `scripts/e2e/seed_provider_acceptance.py` picked a
  stale negotiation row when `PERSIST_USER` was reused. Hardened to match the current manifest
  provider, newest-first.
- `PARAMS_REJECTED` / `RateBelowAcceptedParams` (`reject_reason=4`). `fillServiceId` in
  `payment_streams_module_eligibility.cpp` overwrote the on-chain `rate`/`allocation` with demo
  defaults while building `acceptedParams`, so verify compared chain rate `1` against accepted
  rate `10`. `fillServiceId` now only sets `service_id` fields; the proposal arm sets
  `proposal.params.rate = kDemoRate` explicitly.

### N15, Step 17b localnet snapshot restore (2026-06-19)

Step 17 back-to-back runs reused a live stream whose accrual continued across sessions;
`verify-step10a-dod.sh` only checks that stream PDAs exist, not eligibility runway ([N14](#n14-step-17-paid-query-verify-rejects-2026-06-19)).

Fix: snapshot a **pre-stream** funded baseline (vault `0` deposited, no stream `0`) while the
sequencer is stopped, copying `~/.cache/logos-scaffold/repos/lez/<scaffold.toml pin>/rocksdb/`
plus `.scaffold/wallet`, `.scaffold/state`, and fixture owner/provider state files into
`.scaffold/snapshots/funded/`. Each run restores that baseline and runs
`create-stream-onchain` so stream `0` accrual starts at restore time.

Validity keys in `snapshot.json`: `lez_pin`, `program_id_hex` (same guard as Step 10a),
owner/provider account ids, deposit and stream params. Mismatch → operator runs
`FULL_RESET=1` (prefund + snapshot rebuild). Default demo path:
[`scripts/demo-localnet-prepare.sh`](../../scripts/demo-localnet-prepare.sh); legacy full rebuild:
[`scripts/demo-localnet-fresh.sh`](../../scripts/demo-localnet-fresh.sh) sets `FULL_RESET=1`.

Plan packet: [step-17b-localnet-snapshot-restore.md](../plan/completed/step-17b-localnet-snapshot-restore.md).

### N4, Persistence policy

`payment_streams_module` persists pending-proposal state and per-stream session keys
as a flat JSON file `payment_streams_state.json` in `instancePersistencePath`, atomically written.
If persistence fails (disk full, permissions), the module logs the error
and continues with in-memory state only.
Stale proposals are evicted on cold start and when eligibility/inventory APIs run, by comparing
stored `create_stream_deadline` to clock-10 (no background timer required for the MVP demo).
Session private keys are stored as lowercase hex in that JSON for the demo; treat the persistence
directory as sensitive. A hardened build would encrypt session keys through a wallet-rooted KDF
or keep them ephemeral.

### N5, Provider identity mapping

LIP-155's `provider_id` remains the generic provider identity
used by the payment-stream protocol for replay protection
and provider-specific policy.
The LEZ demo puts the 32-byte stream payee `AccountId` in
`VaultProof.provider_id`.
Predicates compare it to `StreamConfig.provider` with octet equality.
The provider's libp2p `PeerId` is used only for Store routing.
For the MVP, the host configures `payment_streams_module` with a mapping
from `PeerId` to that `AccountId` (e.g. `registerProviderMapping`).
This keeps the delivery layer agnostic to payment-stream identity
while fixing the bytes used in proofs and on-chain streams.

### N6, Delivery module Store query exposure

Step 6 closed the old integration stance: do not block the demo on upstream
Store query landing on `logos-delivery` / `logos-delivery-module` `master`, and do not
pin or maintain the retired exploratory branch
(`feat/liblogosdelivery-query-store`, early `logosdelivery_query_store` / `queryStore` PRs)
in payment-streams flakes. Upstream may still ship Store access on its own timeline; that
roadmap item is independent of this integration.

Active path ([D2](#d2-delivery-module-hook-design)): add `logosdelivery_store_query` on our
`logos-delivery` fork (Step 15) and `storeQuery(...)` on our `logos-delivery-module` fork
(Step 16), wired to eligibility hooks and `payment_streams_module`. Steps 14–20 do not wait on
upstream N6. Step 17 E2E depends on Step 16 bridge landing on those forks ([N12](#n12-step-16-vs-step-17-verification-scope-2025-06-18)), not on upstream
`master`.

Branch workflow: fork from upstream `master` (not module release tags); default shared
branch name `feat/payment-streams-store-eligibility` on both delivery repos; Steps 14–15 on
`logos-delivery`, Step 16 on `logos-delivery-module` with `flake.nix` input
`ref=feat/payment-streams-store-eligibility`; do not reuse `feat/liblogosdelivery-query-store`.
Pin table:
[`feature-branch-pins.md`](feature-branch-pins.md) (flake lock rev documented there).
Summary: [integration-index.md](../../integration-index.md#delivery-integration-branches).

`logos-chat` is not a reusable Store path.
We checked it at `origin/main` (`3a5f508`) and at the `logos-chat-module`
flake pin (`53302e4`): the embedded delivery node mounts only metadata,
filter, and relay (`src/chat/delivery/waku_client.nim`), issues no Store query,
and exposes no Store method.
Chat fetches messages live over relay/filter, so it neither uses nor
re-exports the Store protocol and does not replace the delivery fork Store API ([D2](#d2-delivery-module-hook-design)).

### N7, Session key concurrency

Session key signing is synchronous.
Concurrent Store queries to the same provider serialize at the session key.
The MVP assumes low concurrency; production would need key pooling or async queuing.

### N8, Canonical Store request bytes format

Normative spec for Store eligibility canonical bytes in this integration.
Step 4, Step 15, and Step 3b reference this section; do not copy the struct
layout elsewhere.

The Store eligibility `canonical_payload` is the concatenation
of the domain `PREFIX` and Borsh(`CanonicalStoreRequest`) (see below).
`StreamProof.signature` signs `canonical_payload_digest = SHA-256(canonical_payload)`;
`payment_streams_module` recomputes that digest when verifying.
They are produced by `liblogosdelivery` (Nim, Step 15)
and consumed by `lez-payment-streams-ffi` (Rust, Step 4).
Both sides must produce identical `canonical_payload` for the same input.

#### Domain prefix

```
b"/LEZ/v0.1/StoreEligibility/\x00\x00\x00\x00\x00"
```

Padded to exactly 32 bytes with null bytes,
matching the `PREFIX` pattern in `nssa/src/public_transaction/message.rs`.

#### `CanonicalStoreRequest` struct

The struct is Borsh-serialized in the following field order.
This matches `StoreQueryRequest` from `waku/waku_store/common.nim`
with `eligibility_proof` excluded
and response-only fields absent.

| Field | Borsh type | Source |
| --- | --- | --- |
| `request_id` | `string` (4-byte LE length + UTF-8 bytes) | `StoreQueryRequest.requestId` |
| `include_data` | `u8` (0 or 1) | `StoreQueryRequest.includeData` |
| `has_pubsub_topic` | `u8` (0 or 1) | presence flag |
| `pubsub_topic` | `string` (only if present) | `StoreQueryRequest.pubsubTopic` |
| `content_topics_count` | `u32` LE | length of `contentTopics` |
| `content_topics[i]` | `string` each | `StoreQueryRequest.contentTopics` |
| `has_start_time` | `u8` (0 or 1) | presence flag |
| `start_time` | `i64` LE (only if present) | `StoreQueryRequest.startTime` |
| `has_end_time` | `u8` (0 or 1) | presence flag |
| `end_time` | `i64` LE (only if present) | `StoreQueryRequest.endTime` |
| `message_hashes_count` | `u32` LE | length of `messageHashes` |
| `message_hashes[i]` | 32 bytes each | `StoreQueryRequest.messageHashes` |
| `has_pagination_cursor` | `u8` (0 or 1) | presence flag |
| `pagination_cursor` | 32 bytes (only if present) | `StoreQueryRequest.paginationCursor` |
| `pagination_forward` | `u8` (0 or 1) | `StoreQueryRequest.paginationForward` |
| `has_pagination_limit` | `u8` (0 or 1) | presence flag |
| `pagination_limit` | `u64` LE (only if present) | `StoreQueryRequest.paginationLimit` |

Borsh `string` encoding is a 4-byte little-endian length prefix
followed by the raw UTF-8 bytes (no null terminator).
Optional fields use a presence byte:
`0x00` means absent (field bytes omitted),
`0x01` means present (field bytes follow immediately).

#### Canonical payload digest

```
canonical_payload = PREFIX || borsh(CanonicalStoreRequest)
canonical_payload_digest = SHA-256(canonical_payload)
```

This 32-byte `canonical_payload_digest` is what `StreamProof.signature` signs.

#### Cross-language test vector

The definition of done for Step 15 required a pinned test vector:
construct a `StoreQueryRequest` with fixed known field values,
produce canonical bytes from the Nim serializer and the Rust serializer independently,
and assert byte-level equality.
This mirrors the `hash_public_pinned` test
in `nssa/src/public_transaction/message.rs`
that spells out the expected Borsh encoding byte by byte.

Step 12 demos and `./scripts/verify-step12-dod.sh` use the same field values via Rust:
`cargo run -p lez-payment-streams-core --bin n8_canonical_wire_hex` prints lowercase hex of the
full `canonical_payload` (32-byte domain prefix + Borsh body, 138 bytes for the
reference demo fixture). LogosAPI `canonicalRequestBytes` must be that full
wire, not Borsh-only bytes.
Digest checks use `store_eligibility_digest_matches_n8_reference_fixture` in
`lez-payment-streams-core/src/off_chain/canonical.rs`.

Step 15 Nim parity test lives in `logos-delivery` beside the Store codec tests:
`tests/waku_store/test_store_eligibility_canonical.nim`, comparing lowercase hex to
`cargo run -p lez-payment-streams-core --bin n8_canonical_wire_hex` for the N8 reference
fixture. C ABI smoke: `library/tests/test_eligibility_hooks.c`, target
`make logosdelivery_eligibility_smoke` in `logos-delivery` (not a script in this repo).

### N9, Step 10a local chain fixture (decisions)

Scaffold config and runtime layout

- Commit `scaffold.toml` in this repo (LEZ/SPEL pins move with the integration).
- Do not commit sequencer or wallet state. Use `SCAFFOLD_WS` outside the git tree
  or a gitignored `.scaffold/` under the repo; keys and `.scaffold/state/` stay local.

LEZ pin

- `[repos.lez]` in `scaffold.toml` uses the same revision as LEZ PR 491 and
  [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md) / `nix/payment-streams-ffi.nix`
  (no separate “old LEZ” localnet for 10a and 491 for 10b). Re-run `lgs setup` after pin bumps.

Deploy (canonical for 10a script and runbook)

- After `lgs init`, `lgs setup`, `lgs localnet start` (from the chosen workspace):
  from repo root, `make build`, `make idl`, `make deploy` (`wallet deploy-program` on the guest
  binary), then `make program-id` into the fixture manifest.
- Operator detail and RPC formats: [`docs/step1-findings-scaffold-rpc.md`](docs/step1-findings-scaffold-rpc.md)
  (scaffold discovery doc — not integration plan Step 1, which is Rust FFI only).

Fixture scope

- Step 10a seeds the full demo chain state in one pass: fund owner, deploy program,
  `initialize_vault`, deposit, and `create_stream` for a designated demo vault/stream
  (CLI / IDL + `wallet` until Step 11b), so Step 11a can decode all account types.
- PDAs in the manifest come from the same derivation as `lez-payment-streams-ffi` tests,
  not fragile SPEL CLI account-seed helpers.
- Step 11b still proves module-driven lifecycle; use a fresh `vault_id` or reset chain
  when testing init from `payment_streams_module`, not an empty chain at 11a.

Artifacts

- Idempotent seed script, gitignored `fixtures/localnet.json`, committed
  `fixtures/localnet.json.example`, brief runbook under `docs/`.
- Runbook troubleshooting: owner or `SIGNER_ID` vs wallet home, and foreign localnet on
  3040 — [`docs/step10a-local-chain-fixture.md`](docs/step10a-local-chain-fixture.md)
  (Troubleshooting).
- Step 10a operator troubleshooting and verify failures —
  [`docs/step10a-handoff-and-follow-up.md`](docs/step10a-handoff-and-follow-up.md).
- Why `seed_localnet_fixture` inflates workspace `Cargo.lock` (fat LEZ `wallet`/`lee` deps,
  dual LEZ pins) and optional slimming paths —
  [`docs/step10a-local-chain-fixture.md`](docs/step10a-local-chain-fixture.md)
  (Seed binary and workspace Cargo.lock).

SPEL-on-LEE cleanup (public PDA prefix)

Published SPEL (`v0.5.0`) derives in-guest public PDAs with the NSSA prefix
(`/NSSA/v0.2/...`). LEZ PR 491 and `lez-payment-streams-core` use LEE
(`/LEE/v0.2/...`). Until upstream SPEL matches LEZ, this repo vendors
`vendor/spel-framework-core` and patches `compute_pda` to call
`lee_core::AccountId::for_public_pda` (root `Cargo.toml` and
`methods/guest/Cargo.toml` `[patch]` on `spel-framework-core`).

When SPEL officially targets LEE, that vendor fork should be removable.
Do not drop it on a version bump alone; confirm upstream `compute_pda`
matches host PDA derivation (FFI tests) and 491 localnet
(`initialize_vault` without `MismatchedPdaClaim`).

Then simplify: remove `vendor/spel-framework-core`, remove both
`[patch."https://github.com/logos-co/spel.git"]` entries, bump the SPEL
pin if needed, `make build`, full 10a chain reset, and
`./scripts/verify-step10a-dod.sh` exit 0.

The guest deposit `authenticated_transfer` enum encoding is implemented in tree; SPEL-on-LEE may
allow removing that shim later — verify deposit on 491 before deleting it.

### N10, Step 11b module writes (decisions)

Wallet submit and module shape

- Submit via PR 19 `send_generic_public_transaction` inside the wallet. The Universal module
  calls `send_generic_public_transaction_json` (one JSON string) over LogosAPI because
  QList-shaped cross-module IPC to the Legacy wallet is unreliable.
- Instruction bytes passed to the wallet are Borsh-serialized guest instruction bytes as
  `QList<uint8_t>`; the wallet runs `wallet_ffi_serialization_helper` to LE u32 words (not
  caller-supplied decimal string lists).
- Guest ELF: Step 10a `lez_payment_streams.bin` with `PAYMENT_STREAMS_GUEST_BIN` on the daemon;
  the PS module omits the ELF blob from IPC when that env var is set. Deposit uses wallet
  `authenticated_transfer_elf()` as a dependency when deps are empty. Bundling guest ELF inside
  the PS `.lgx` remains a follow-on. Step 11d ([LEZ PR 510](https://github.com/logos-blockchain/logos-execution-zone/pull/510))
  should replace or narrow this env-var path once deploy and program ELF are registered through
  `wallet_ffi` and exposed on `logos_execution_zone`.
- Nine write operations plus two status queries are implemented on the impl class; a single
  public `chainAction(operation, paramsJson)` router exposes them on the LogosAPI surface
  (see [N11](#n11-universal-module-public-api)). `signing_requirements` are derived
  from the signer vs the FFI-planned account list.
- Submit-level JSON only (`success`, `tx_hash`, `error`) from writes. Callers and
  `./scripts/verify-step11b-dod.sh` use wallet `sync_to_block` when sequencer height is
  available, retries on status `chainAction`, and may SKIP status when derived PDAs are not yet
  readable after successful submits.

E2E signer and wallet (G)

- Writes take signer/provider base58 in `chainAction` JSON. DoD uses manifest
  `owner_account_id` and `provider_account_id` with wallet storage copied from
  `.scaffold/wallet/storage.json` into the e2e dir (seeded owner keys).
- Module-driven lifecycle uses `vault_id` 1 by default or a reset chain (`reserved_for_step_11b`
  in the manifest). Demo vault `0` stays for Step 11a decode only.

Fixture and config (H)

- Chain fixture: gitignored `fixtures/localnet.json` (template `fixtures/localnet.json.example`).
  Override with env `FIXTURE_MANIFEST`.
- The module loads `program_id_hex` and related manifest fields once (init or first chain use).
  Write and status helpers do not take `program_id` on every call. Default clock account is
  `CLOCK_10` from the manifest when needed.
- Wallet sequencer RPC comes from `wallet_config.json` (`sequencer_addr`). Manifest
  `sequencer_url` documents the expected endpoint for operators and verify scripts.

### N11, Universal module public API

Universal modules (`"interface": "universal"`) export every `public:` method on
`PaymentStreamsModuleImpl` through `logos-cpp-generator --from-header` (plugin
`callMethod` / `getMethods`). There is no fixed cap on method count. Reserved
`LogosModuleContext` hooks (`onContextReady`, `modules`, `modulePath`, `instanceId`,
`instancePersistencePath`) are not exported.

Step 11b uses a `chainAction` router to keep write/status operations behind one
Logos entry point; that is an API ergonomics choice, not a codegen limit. Step 12
adds named eligibility methods; Step 16 registration must match those names exactly
(for example `prepareEligibilityForStoreQuery`).
