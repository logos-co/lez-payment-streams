# Step 16 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 16, Generic eligibility routing in `logos-delivery-module`

Blocked on upstream Store query API (N6, Step 6).
Implement when `master` exposes query entrypoints;
method name may differ from early PR sketches.

Architectural context:
this step modifies the C++ Qt-plugin shell of `delivery_module`.
It bridges the Step 15 C callbacks into LogosAPI calls
on a configurable named module
(`payment_streams_module` in our demo;
any module with the same method names in the future).
The registration uses the auto-generated `getPluginMethods`
introspection surface every Logos module already exposes.

On our branch of `logos-delivery-module` (eligibility hooks only; build
`liblogosdelivery` / module against upstream `master` for Store query),
extend the `delivery_module` interface with
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`,
wire through upstream `queryStore` when present on `master`,
and add a `paidStoreMode` configuration toggle to `createNode`.
Do not add a parallel `queryStore` implementation in our fork.
Implement the bridge that translates the new `liblogosdelivery` callbacks
into `LogosAPIClient` calls on the named module
(`verifyEligibilityForStoreQuery`, `prepareEligibilityForStoreQuery`).
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

Definition of done:
Prerequisite: upstream Store query API on `logos-delivery-module` `master`.
Without any verifier registered,
`delivery_module` behaves exactly as it did at the pre-eligibility baseline aside from upstream
Store query APIs.
Registering a module that does not expose the expected methods
returns a structured error and leaves the previous registration in place.
Store queries can be issued through `delivery_module`
against an explicit provider peer address using the upstream Store query API.
With `payment_streams_module` registered as both verifier and provider,
an end-to-end Store query produced by the user
returns a successful Store outcome
and a successful eligibility outcome on the provider side.
Requests failing eligibility checks immediately return
a `BAD_REQUEST` (400) Store status code,
a populated `eligibility_status` object with the specific verdict,
and an empty messages list.

