# Decisions — historical postmortems

Moved from integration-decisions.md (maintainer depth).

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

Mitigations (`make verify-step17` / [`scripts/e2e.sh`](../scripts/e2e.sh) `local run`; archived
[`scripts/archive/demo-e2e-local.sh`](../scripts/archive/demo-e2e-local.sh)):

- Default: after `lgpm install` of the delivery `.lgx`, optionally overlay
  `make liblogosdelivery` from sibling `logos-delivery` (`LOGOS_DELIVERY_ROOT`) unless
  `SKIP_LIBLOGOSDELIVERY_OVERLAY=1`.
- Hermetic path: set `SKIP_LIBLOGOSDELIVERY_OVERLAY=1`, use `DELIVERY_MODULE_ROOT` with
  `flake.lock` at `logos-delivery` ≥ `39b467ec`; see
  [archive/steps/local-store-dual-host-runbook.md](../archive/steps/local-store-dual-host-runbook.md#hermetic-run-hand-off).

Clean nix-only path: push the `logos-delivery` fix (retain JSON `eligibilityProofHex` through
`logosdelivery_store_query`), run `nix flake update logos-delivery` in `logos-delivery-module`,
commit `flake.lock`, update [feature-branch-pins.md](../reference/feature-branch-pins.md), re-run E2E with
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
  (≈400 s), so the stream depleted before or during a run. Fixed at the time by larger seed
  sizing (deposit `2400` / allocation `1800`). **Superseded (2026-06-28):** conservative
  `1000` / `200` defaults plus clock sync after restore ([Step 24c](../plan/completed/step-24c-simplify-demo-flow.md));
  per-run streams and teardown replace a single long-lived stream `0`.
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
`verify-step10a-dod.sh` only checks that stream PDAs exist, not eligibility runway ([N14](decisions-historical.md#n14-step-17-paid-query-verify-rejects-2026-06-19)).

Fix: snapshot a **pre-stream** funded baseline (vault `0` deposited, no stream `0`) while the
sequencer is stopped, copying `~/.cache/logos-scaffold/repos/lez/<scaffold.toml pin>/rocksdb/`
plus `.scaffold/wallet`, `.scaffold/state`, and fixture owner/provider state files into
`.scaffold/snapshots/funded/`. Each run restores that baseline and runs
`create-stream-onchain` at `next_stream_id` after `wait-clock-synced` (Clock10 ≈ wall time;
supersedes relying on restore-time accrual anchor alone).

Validity keys in `snapshot.json`: `lez_pin`, `program_id_hex` (same guard as Step 10a),
owner/provider account ids, deposit and stream params. Mismatch → operator runs
`make full-reset-localnet` (prefund + snapshot rebuild). Default demo path:
`make prepare-localnet` (formerly [`scripts/archive/demo-localnet-prepare.sh`](../../scripts/archive/demo-localnet-prepare.sh));
full rebuild: `make full-reset-localnet` (formerly
[`scripts/archive/demo-localnet-fresh.sh`](../../scripts/archive/demo-localnet-fresh.sh), which set `FULL_RESET=1`).

Plan packet: [step-17b-localnet-snapshot-restore.md](../plan/completed/step-17b-localnet-snapshot-restore.md).
