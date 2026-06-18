# Step 16 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 16, Generic eligibility routing in `logos-delivery-module`

Architectural context:
this step modifies the C++ Qt-plugin shell of `delivery_module`.
It bridges the Step 15 C callbacks into LogosAPI calls
on a configurable named module
(`payment_streams_module` in our demo;
any module with the same method names in the future).
The registration uses the auto-generated `getPluginMethods`
introspection surface every Logos module already exposes.

On our fork of `logos-delivery-module`,
extend the `delivery_module` interface with
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`,
and add a `storeQuery(...)` LogosAPI method that calls `logosdelivery_store_query`
(added on our fork of `logos-delivery` in Step 15).
Do not wait on upstream N6 for the Store query API;
the method is added entirely within our fork.
Point the flake `logos-delivery` input at the Step 15 fork rev
([delivery integration branches](../../../integration-index.md#delivery-integration-branches)).
Implement the bridge that translates the new `liblogosdelivery` callbacks
into `LogosAPIClient` calls on the named module
(`verifyEligibilityForStoreQuery`, `prepareEligibilityForStoreQuery`).
The verifier bridge runs inside the blocking `EligibilityVerifierCb` ([N3](../../reference/decisions-and-notes.md#n3-provider-side-verification-latency-and-blocking-hooks)):
map JSON `eligibility` to the integer status code for the C return value, and copy JSON
`message` into `out_desc` when the module returns a verdict failure (truncate to buffer size;
NUL-terminate). Leave `out_desc` empty on OK so inbound success responses omit
`eligibility_status` from the inner handler path as today.
Method names, argument shapes, and return shapes for these calls are specified in
[integration-contracts.md](../../integration-contracts.md).
Note that the host application is responsible for calling
`registerProviderMapping` on the streams module before initiating queries.
On registration, the bridge calls the named module's auto-generated
`getPluginMethods` and rejects the registration with a structured error
if the expected method names are not present,
so misconfiguration surfaces at setup time rather than on the first Store request.

Components required to run:
the unit-level checks (no verifier registered, structured error on misregistration)
needs only a `logoscore` daemon with `delivery_module` loaded.
The full Store query exchange is the Step 17 demo
and requires the full stack documented there.

Note on `storeQuery` return shape:
`storeQuery` returns a JSON-serialised `StoreQueryResponse` via the standard LogosAPI
callback mechanism, including the messages list and, when present, the `eligibilityStatus`
object (with `code` and `desc` fields).
How consuming modules display or act on `eligibilityStatus` is outside this step's scope.

Definition of done:
Without any verifier registered,
`delivery_module` behaves exactly as it did at the pre-eligibility baseline.
Registering a module that does not expose the expected methods
returns a structured error and leaves the previous registration in place.
Store queries can be issued through `delivery_module`'s `storeQuery` method
against an explicit provider peer address.
With `payment_streams_module` registered as both verifier and provider,
an end-to-end Store query produced by the user
returns a successful Store outcome
and a successful eligibility outcome on the provider side.
Requests failing eligibility checks immediately return
a `BAD_REQUEST` (400) Store status code,
a populated `eligibility_status` object with the specific verdict and desc,
and an empty messages list.

