# Architectural Dilemma: Universal vs. Legacy Module Pattern

This document outlines an architectural dilemma encountered during the implementation of the `payment_streams_module` (Step 6c of the integration plan).
It serves as a self-contained summary to share with colleagues and seek advice on the best path forward.

## Background: The Logos Ecosystem Transition

The Logos ecosystem is currently migrating its module architecture:
*   Legacy Pattern: Uses Qt heavily, inherits from `PluginInterface`, and relies on `LogosAPIClient::invokeRemoteMethod` for cross-module communication.
*   Universal Pattern: Uses pure C++ (no Qt exposed to the developer), inherits from `LogosModuleContext`, and relies on `logos-cpp-generator` to create typed wrappers for static dependencies (`modules().<dep>`).
    For dynamic invocation, it uses `LogosAPI::callModule()`.

Note on `metadata.json`: The distinction between these patterns is declared via the `interface` field, not the `type` field.
Both patterns are typically `"type": "core"` (meaning they are headless backend services).
A module is Universal if it specifies `"interface": "universal"`.
If the `interface` field is omitted, the build system defaults to treating it as Legacy.
(For example, PR #18 in `logos-execution-zone-module` migrated the build system to `logos-module-builder` but did not add the `interface` field, meaning the wallet remains on the Legacy pattern).

The core team's strategic direction is to move all modules to the Universal pattern.

## The Situation

We are building a new intermediary module, `payment_streams_module`.
It sits between two existing modules:
1.  Upstream: `delivery_module` (Universal pattern, explicitly declares `"interface": "universal"`).
2.  Downstream: `lez_wallet_module` (Legacy pattern, omits the `interface` field).

### The Upstream Constraint (Dynamic Invocation)

`delivery_module` acts as a generic router.
It needs to call our `payment_streams_module` dynamically by name at runtime (e.g., via a `setEligibilityVerifier("payment_streams_module")` hook) rather than declaring it as a static dependency.
This means the upstream call will use the Universal dynamic API (`LogosAPI::callModule()`).

### The Downstream Constraint (Legacy Target)

Our `payment_streams_module` must call `lez_wallet_module` to read and write to the blockchain.
Because `lez_wallet_module` is still on the Legacy pattern, we cannot use the safe, typed Universal wrapper (`modules().lez_wallet_module`) because the code generator assumes all targets in `metadata.json` are also Universal.

## The Core Problem: Mixing Patterns

The dilemma arises because we are forced to bridge a Universal module and a Legacy module.
While `logoscore` can load both types of modules, bridging them dynamically is an untested edge case in the current architecture.

Specifically, a Universal module calling a Legacy module dynamically (using `LogosAPI::callModule()` targeting a `PluginInterface` module) relies on the host's internal routing to safely map `QVariantList` arguments to the legacy `Q_INVOKABLE` signature.
It is unclear if this mixed-pattern routing is officially supported or tested.

We have two approaches, both with significant trade-offs.

---

## Approach 1: Build as Legacy (The Pragmatic Choice)

Build `payment_streams_module` using the deprecated Legacy pattern.

*   How it works: `delivery_module` (Universal) calls `payment_streams_module` (Legacy) dynamically.
    `payment_streams_module` (Legacy) calls `lez_wallet_module` (Legacy) via `invokeRemoteMethod`.
*   Pros:
    *   Proven Downstream: The Legacy -> Legacy call to the wallet is battle-tested and guaranteed to work.
    *   Isolates Risk: If the integration fails, we know exactly where to look: the upstream Universal -> Legacy dynamic call from the delivery module.
*   Cons:
    *   Technical Debt: We are writing brand new code on a deprecated pattern, guaranteeing a rewrite later.
    *   Still Untested Upstream: We do not escape the mixed-pattern boundary; we just push it to the upstream edge (`delivery_module` calling us).

## Approach 2: Build as Universal (The Strategic Choice)

Build `payment_streams_module` using the new Universal pattern.

*   How it works: `delivery_module` (Universal) calls `payment_streams_module` (Universal) dynamically.
    `payment_streams_module` (Universal) calls `lez_wallet_module` (Legacy) dynamically via `LogosAPI::callModule()`.
*   Pros:
    *   Strategic Alignment: Aligns with the core team's vision.
        No migration debt later.
    *   Proven Upstream: The Universal -> Universal dynamic call from the delivery module is the intended architectural path.
*   Cons:
    *   Untested Downstream: Forces a Universal -> Legacy dynamic call to the wallet.
    *   Loss of Static Safety: We cannot declare the wallet in our `metadata.json` to get a typed wrapper.
        The code generator assumes Universal targets and generates incompatible C++ wrappers for Legacy modules.
        Furthermore, even if the wallet were Universal, constructing the `LogosModules` wrapper in a core sidecar currently causes a segmentation fault (Issue 31).
        Therefore, we must use dynamic calls and handle missing-module errors manually.

---

## The Ask / Next Steps

The core question is: Does the `logoscore` host safely route `LogosAPI::callModule()` from a Universal caller to a Legacy `PluginInterface` callee?

If the host's internal `LogosAPI` implementation can perfectly map the `QVariantList` from the Universal caller to the specific `Q_INVOKABLE` signature of the Legacy wallet, then Approach 2 (Universal) is viable.
If it segfaults or fails to cast types, we must use Approach 1 (Legacy).

To answer this, we will implement a minimal "probe" module to test the Universal -> Legacy dynamic boundary before committing to the architecture for `payment_streams_module`.