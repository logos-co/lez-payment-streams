# Step 17 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 17, End-to-end demo wiring

Prerequisite: Step 16 bridge complete ([N12](../../reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18);
`storeQuery` and eligibility routing on our delivery forks; see [N6](../../reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure)).

Architectural context:
this is the only step that exercises every layer at once:
two Logos hosts (`logoscore` daemons),
all three backend modules in each host,
the LEZ sequencer for chain reads and writes,
and direct Store traffic from the user host to the provider host.

Step 16 landed eligibility routing and async `storeQuery` on `delivery_module`.
This step owns full-stack verification: paid Store query across two hosts, eligibility
outcomes on user and provider, and inbound Store `BAD_REQUEST` when verification fails
([N12](../../reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)).

Note that the host application is responsible for calling
`registerProviderMapping` on the streams module before outbound Store queries in the Step 17 demo.

Create a single shell script that
starts a fresh scaffold workspace,
deploys `lez_payment_streams`,
builds `.lgx` packages for `logos_execution_zone` (our branch),
`payment_streams_module`,
and `delivery_module` built from our `logos-delivery-module` integration branch (Step 16),
linking `liblogosdelivery` from the matching `logos-delivery` fork rev
([delivery integration branches](../../../integration-index.md#delivery-integration-branches),
[D2](../../reference/decisions-and-notes.md#d2-delivery-module-hook-design)),
installs them with `lgpm` into two module directories,
launches two `logoscore` instances loaded with all three modules
on disjoint `portsShift` values
(per the workaround documented in
[`logos-delivery-module#18`](https://github.com/logos-co/logos-delivery-module/issues/18)
and used by `logos-delivery-demo`;
example: user `portsShift: 0`, provider `portsShift: 100`),
starts the provider `delivery_module` with relay and Store service enabled,
backed by a SQLite archive and a demo retention policy,
starts the user `delivery_module` with Store client support
and the provider's explicit peer address configured as the Store target,
drives the user flow from vault initialization through Store query,
and drives a manual claim on the provider side.
The script captures structured logs at each phase.

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

The Step 16 bridge is installed from the integration branch and both hosts register
`payment_streams_module` as eligibility verifier and provider where the demo requires it.

A user-initiated Store query through `delivery_module.storeQuery` against the provider peer
returns a successful Store outcome when chain state and proofs are valid, including successful
eligibility on the provider inbound path.

When eligibility checks fail on the provider (including Store requests with no proof while
paid mode expects one — see [N3c](../../reference/decisions-and-notes.md#n3c-inbound-missing-proof-null-proof_hex-2025-06-18)),
the provider observes `BAD_REQUEST` (400), a populated `eligibility_status` object with
verdict and `desc`, and an empty messages list.

The script runs to completion against a clean workspace
and produces a log artifact that documents
every chain transaction, every Store request,
and the eligibility outcomes observed on both ends.

