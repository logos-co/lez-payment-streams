# Step 16 — plan excerpt

Status: complete on `logos-delivery-module` (`bf104a6bfde35ce4fcae5081278d1996ebf5e3c1`).
Completed agent packet. Index: [index.md](../index.md).
Normative bridge policy: [Resolved implementation decisions](#resolved-implementation-decisions-2025-06-18)
and [integration-contracts.md](../../reference/integration-contracts.md).

### Step 16, Generic eligibility routing in `logos-delivery-module`

Architectural context:
this step modifies the C++ Qt-plugin shell of `delivery_module`.
It bridges the Step 15 C callbacks into `LogosAPI` `callModule` invocations
on a configurable named module
(`payment_streams_module` in our demo;
any module with the same method names in the future).

On our fork of `logos-delivery-module`,
extend the `delivery_module` interface with
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`,
and add `storeQuery(queryJson, providerAddr)` backed by `logosdelivery_store_query`
(Step 15 on `logos-delivery`).
Do not wait on upstream N6; the API is added on our forks only.
Pin the `logos-delivery` flake input to
`feat/payment-streams-store-eligibility`
([delivery integration branches](../index.md#delivery-integration-branches),
[feature-branch-pins.md](../../reference/feature-branch-pins.md)).

Implement trampolines that call
`verifyEligibilityForStoreQuery` and `prepareEligibilityForStoreQuery`
on the registered module ([N3](../../reference/integration-decisions.md#n3-provider-side-verification-latency-and-blocking-hooks),
[N3a](../../reference/integration-decisions.md#n3a-step-16-threading--approach-a-experiment-2025-06-18)).
Map verify JSON to the C return code and copy failure `message` into `out_desc`
(truncate, NUL-terminate). Leave `out_desc` empty on OK.

On registration, validate the target module via `getPluginMethods`; reject with a
structured error if required method names are missing, leaving the previous registration
in place ([N3b](../../reference/integration-decisions.md#n3b-step-16-hook-registration-lifecycle-2025-06-18)).

### Resolved implementation decisions (2025-06-18)

Agents should treat the following as fixed unless this section is updated.
Detail: [N3a](../../reference/integration-decisions.md#n3a-step-16-threading--approach-a-experiment-2025-06-18),
[N3b](../../reference/integration-decisions.md#n3b-step-16-hook-registration-lifecycle-2025-06-18),
[N3c](../../reference/integration-decisions.md#n3c-inbound-missing-proof-null-proof_hex-2025-06-18),
[N12](../../../reference/decisions-historical.md#n12-step-16-vs-step-17-verification-scope-2025-06-18).

| Topic | Decision |
| --- | --- |
| Integration branch | `feat/payment-streams-store-eligibility` on `logos-delivery-module`; flake input `logos-delivery` on the same branch ([feature-branch-pins.md](../../reference/feature-branch-pins.md)). |
| Bridge IPC | Approach A: `LogosAPI` / `LogosModules` from universal `onInit`; `callModule` from Step 15 C trampolines. |
| Inbound verifier | Sync `callModule` from the verifier trampoline (liblogosdelivery async thread; owner not blocked on store-query sem). |
| NULL `proof_hex` | Always delegate to the module with empty `proofBytes` ([N3c](../../reference/integration-decisions.md#n3c-inbound-missing-proof-null-proof_hex-2025-06-18)). |
| Outbound `storeQuery` | Async like `start` / `stop`: dispatch FFI, complete on typed event with StoreQueryResponse JSON ([N3a](../../reference/integration-decisions.md#n3a-step-16-threading--approach-a-experiment-2025-06-18)). |
| `storeQuery` args | `(queryJson, providerAddr)` per [integration-contracts.md](../../reference/integration-contracts.md). |
| Registration | `setEligibilityVerifier` / `setEligibilityProvider`; `getPluginMethods` before commit; failed set leaves prior registration. |
| Hook lifecycle | [N3b](../../reference/integration-decisions.md#n3b-step-16-hook-registration-lifecycle-2025-06-18) (within [N3a](../../reference/integration-decisions.md#n3a-step-16-threading--approach-a-experiment-2025-06-18) threading constraints). |
| Verification scope | [N12](../../../reference/decisions-historical.md#n12-step-16-vs-step-17-verification-scope-2025-06-18): bridge only in Step 16; full stack in Step 17+. |
| Tests | Unit mocks for new FFI; `tests/test_approach_a_thread_probe.cpp`; logoscore registration checks. |

Components required to run:
`logoscore` with `delivery_module` suffices for registration and misconfiguration tests.
Full Store exchange: [N12](../../../reference/decisions-historical.md#n12-step-16-vs-step-17-verification-scope-2025-06-18) (Step 17+).

Definition of done (Step 16 — bridge only):

- With no verifier registered at FFI, inbound Store follows the pre-eligibility baseline ([N3b](../../reference/integration-decisions.md#n3b-step-16-hook-registration-lifecycle-2025-06-18)).
- Bad registration ⇒ structured error; prior registration unchanged.
- Verifier enabled ⇒ trampoline invokes module; NULL `proof_hex` ⇒ empty `proofBytes` ([N3c](../../reference/integration-decisions.md#n3c-inbound-missing-proof-null-proof_hex-2025-06-18)).
- Provider enabled ⇒ async `storeQuery` without owner-thread deadlock ([N3a](../../reference/integration-decisions.md#n3a-step-16-threading--approach-a-experiment-2025-06-18)).
- Unit tests and logoscore checks cover the wiring described above.

Out of scope per [N12](../../../reference/decisions-historical.md#n12-step-16-vs-step-17-verification-scope-2025-06-18):
[step-17.md](../completed/step-17.md), [step-18-public-testnet-demo.md](../completed/step-18-public-testnet-demo.md).
