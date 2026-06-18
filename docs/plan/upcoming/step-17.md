# Step 17 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 17, End-to-end demo wiring

Prerequisite: Step 16 bridge complete ([N12](../../reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18);
`storeQuery` and eligibility routing on our delivery forks; see [N6](../../reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure)).

Architectural context:
this step exercises every layer at once (two `logoscore` hosts, three backend modules each,
local LEZ, cross-host Store). Scope boundary: [N12](../../reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)
(Step 16 proved the bridge; Step 17 proves full-stack paid Store and eligibility on the wire).

The host application must call `registerProviderMapping` on the streams module before outbound
Store queries in this demo.

Create `scripts/demo-e2e-local.sh` that:

- starts a fresh scaffold workspace,
- deploys `lez_payment_streams`,
- builds `.lgx` packages for `logos_execution_zone` (our branch),
  `payment_streams_module`,
  and `delivery_module` built from our `logos-delivery-module` integration branch (Step 16),
  linking `liblogosdelivery` from the matching `logos-delivery` fork rev
  ([delivery integration branches](../../../integration-index.md#delivery-integration-branches),
  [D2](../../reference/decisions-and-notes.md#d2-delivery-module-hook-design)),
- installs them with `lgpm` into two module directories,
- launches two `logoscore` instances loaded with all three modules
  on disjoint `portsShift` values
  (per [`logos-delivery-module#18`](https://github.com/logos-co/logos-delivery-module/issues/18)
  and `logos-delivery-demo`; example: user `portsShift: 0`, provider `portsShift: 100`),
- starts the provider `delivery_module` with relay and Store service enabled,
  backed by a SQLite archive and a demo retention policy,
- starts the user `delivery_module` with Store client support
  and the provider's explicit peer address as the Store target,
- drives the user flow from vault initialization through Store query,
- drives a manual claim on the provider side,
- captures structured logs at each phase.

The first smoke path uses two nodes:
the provider archives messages and the user queries the provider directly.
For the fastest integration smoke test,
the user may publish a message that the provider archives
and then issue a paid Store query for it.
If time allows,
the demo should add a third publisher node
that publishes messages for the provider to archive,
so the user retrieves historical messages it did not originate.

Components required to run:
LEZ sequencer on `127.0.0.1:3040`,
`lez_payment_streams` program deployed onto it
(Step 11d complete, or documented CLI deploy from Step 10a on a clean workspace),
two `logoscore` daemons (one for user, one for provider),
each daemon hosting `logos_execution_zone`, `payment_streams_module`,
and `delivery_module`,
provider `delivery_module` configuration with relay and Store service enabled,
a SQLite Store archive path,
a retention policy such as `capacity:10000`,
user `delivery_module` configuration with the provider's explicit peer address
as the Store target,
and direct network reachability between the two local hosts.

Definition of done:

1. Bridge and hooks: Step 16 integration branch installed; both hosts register
   `payment_streams_module` as eligibility verifier and provider where the demo requires it.
2. Success path: user-initiated `delivery_module.storeQuery` against the provider peer returns a
   successful Store outcome when chain state and proofs are valid, including provider inbound
   eligibility OK.
3. Failure path: when provider eligibility fails (including missing proof in paid mode — [N3c](../../reference/decisions-and-notes.md#n3c-inbound-missing-proof-null-proof_hex-2025-06-18)),
   provider sees Store `BAD_REQUEST` (400), populated `eligibility_status` (verdict and `desc`),
   and an empty messages list.
4. Artifact: `scripts/demo-e2e-local.sh` runs to completion on a clean workspace and writes a
   log artifact covering every chain transaction, Store request, and eligibility outcomes on both
   ends.

Follow-on: Step 18 (testnet), Step 19 (LIP on-chain), Step 20 (developer journey); optional
Steps 21–22 (UI). See [integration-index.md](../../../integration-index.md#program-outcomes).
