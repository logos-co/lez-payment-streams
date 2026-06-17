# Logos Architecture Overview

This document records the architectural facts about the Logos ecosystem
that are prerequisites for understanding `integration-index.md`.
It is not a substitute for the official references,
in particular `logos-tutorial/logos-developer-guide.md`,
`logos-liblogos/README.md`,
`logos-execution-zone/README.md`,
and the journey doc
[Use the Logos Delivery module API from an app](https://github.com/logos-co/logos-docs/blob/main/docs/messaging/journeys/use-the-logos-delivery-module-api-from-an-app.md).

## Names that overlap

Two name collisions trip up newcomers.

### LEZ is the only chain layer we care about

LEZ (Logos Execution Zone) is the programmable layer we interact with.
It owns the sequencer, NSSA account model, on-chain programs
(token, AMM, clock, and others),
the `wallet` binary, and its `wallet_ffi` Rust crate.
All on-chain reads and writes in this integration go to LEZ.

A separate L1 layer also exists in the broader Logos stack
(in a repo called `logos-blockchain`),
but nothing in this work runs it, talks to it, or depends on it.
For the purposes of this integration, treat "the chain" as "LEZ"
and ignore the rest.

The word "logos-blockchain" sometimes appears as a GitHub org prefix
(e.g. `github.com/logos-blockchain/logos-execution-zone`).
That is `<org>/<repo>`; the repo named there is LEZ.

### "Wallet module" always means the LEZ wallet

In module-related docs, "wallet module" refers to `logos_execution_zone`
in the `logos-execution-zone-module` repo.
It wraps LEZ's `wallet_ffi`.
Field values like `"category": "blockchain"`
or `"author": "Logos Blockchain Team"` in its `metadata.json`
are org branding and do not imply communication with anything outside LEZ.

## Hosts and the runtime library

`logos-liblogos` is a library repo.
It produces `liblogos_core` (a C-API shared library)
and `logos_host` (a per-module subprocess binary).

A host is any application binary that links `liblogos_core`
and uses it to load modules, expose `LogosAPI` to them,
and route inter-module calls.

Three hosts ship today:

- `logoscore` is a headless CLI host
  (binary from `logos-logoscore-cli`).
  Used for scripting, CI, and our end-to-end demo.
- `logos-basecamp` is a desktop Qt application host
  with a sidebar, MDI workspace, and package-management UI.
- `logos-standalone-app` is a minimal host
  for running a single UI plugin in isolation during development.

The three hosts are siblings.
They all consume the same `liblogos_core` and load the same `.lgx`-packaged modules.
Diagrammatically:

```
   logoscore           logos-basecamp        logos-standalone-app
  (headless CLI)       (desktop GUI)         (single-UI test host)
         \                   |                    /
          \                  |                   /
           (each links against liblogos_core)   /
            \                |                 /
             v               v                v
         +------------------------------------+
         | liblogos_core                      |
         | + module loading (Qt plugins)      |
         | + Qt Remote Objects IPC            |
         | + LogosAPI surface for modules     |
         +-----------------+------------------+
                           |
                           v
              +------------------------+
              | logos_host subprocess  |
              | one per loaded module  |
              +------------------------+
```

## Modules

A module is a loadable component packaged as a `.lgx` file
(a gzipped tar with platform-specific library variants).
At runtime, the module system loads the library from the `.lgx`
into a `logos_host` subprocess
(or, for QML-only `ui_qml` modules, in-process inside the host),
reads `metadata.json` to identify the module,
and exposes its public methods to other modules through `LogosAPI`.

The module implementation may be written manually (legacy pattern)
or generated from a pure C++ header (universal pattern).
Both approaches result in a Qt plugin at runtime,
but how you write the source differs significantly.
See [Module Implementation Patterns](#module-implementation-patterns) for details.

Three module types matter for this integration:

- `core` is a backend module with no UI.
  Examples: `logos_execution_zone`, `delivery_module`,
  `payment_streams_module` (added by this work).
- `ui_qml` with a C++ backend has a QML view plus a backend process
  exposing typed remoting via a `.rep` file.
  Example: `logos-delivery-demo`.
- `ui_qml` QML-only is a pure QML view that calls backend modules
  through the `logos` bridge.
  Useful for thin UIs over existing backends.

## Hosts versus modules

A clean distinction worth keeping:

- The CLI is a host, not a module.
  `logoscore` parses its own command line and issues `LogosAPI` calls
  directly inside its own process.
  There is no `logoscore.lgx`.
- The Basecamp GUI application is a host, not a module.
  The visible parts of Basecamp (sidebar, workspace, package manager)
  live in its main app process.
- A UI plugin loaded inside Basecamp is a module.
  Its `metadata.json` declares `type: ui_qml`,
  it ships as `.lgx`,
  and Basecamp loads it like any other module.

The asymmetry is structural: hosts wrap the runtime,
modules sit inside the runtime.
Backend modules do not know which host loaded them
and do not need to be re-implemented when a UI host is added.

### Dependencies. Outbound versus Inbound

The `dependencies` array in `metadata.json` declares outbound dependencies,
what other modules this module needs to call.
The runtime loads these dependencies before your module.

Example. `payment_streams_module` calls `logos_execution_zone` for chain reads and writes.
It does not list the wallet in `metadata.json` (D6).
Load the wallet module before payment streams at runtime.

Inbound dependencies, modules that call you, are not declared in `metadata.json`.
They are established at runtime through registration.

Example. `delivery_module` calls `payment_streams_module`
via the eligibility verifier or provider hooks configured in Step 16,
but `payment_streams_module` does not list `delivery_module` in its dependencies.
The host application calls `setEligibilityVerifier("payment_streams_module")`
on `delivery_module`, creating the inbound link dynamically.

For this integration, outbound wallet access is runtime-only, not a static
`dependencies` entry. Other modules may still declare static dependencies when
codegen and dependency types align (Universal-on-Universal or Legacy patterns).

## Two boundaries

There are two distinct boundaries in this stack.
The integration plan describes the same layers as Rust FFI (Boundary A here) and
LogosAPI / inter-module calls (Boundary B here).

### Boundary A. C FFI to non-C++ backends

When a module wraps non-C++ logic (Nim, Rust), it uses a C FFI.
The C++ module calls into a library through a C-compatible binary interface.

#### C FFI as binary-level interoperability

C FFI in this stack refers to binary-level interoperability, not the C programming language.

All languages (Rust, Nim, C++) compile to machine code.
They interoperate at the binary level by following C ABI conventions
for function calling: which registers hold arguments,
how the stack is laid out, how struct fields are aligned.

When Rust exports `#[no_mangle] pub extern "C" fn`,
it generates machine code that follows C calling conventions.
When C++ imports that function via `extern "C"`,
it expects that same calling convention.
The compilers coordinate so the binaries can call each other.

There is no C code at runtime.
Nim compiles to C source first, then to machine code,
but the C source is an implementation detail.
What matters is the resulting binary follows C ABI conventions.

This is per-module and private.
`delivery_module` calls `liblogosdelivery` (Nim with C ABI).
`logos_execution_zone` calls `wallet_ffi` (Rust with C ABI).
`payment_streams_module` will call `lez-payment-streams-ffi` (Rust with C ABI).

Other modules never see this boundary.
They only see the public C++ surface that the module exposes to the runtime.

### Boundary B. Inter-Module Communication via Runtime

This is the boundary between modules.
All communication between modules is mediated by the runtime.

#### Module lifecycle

When a module is loaded:

1. Runtime loads the `.lgx` and extracts the plugin library
2. Runtime calls `initLogos(LogosAPI* api)` (legacy) or `onInit(LogosAPI* api)` (universal)
3. Module stores the `api` pointer for later use
4. Runtime discovers available methods via Qt's meta-object system

The module then lives until the host shuts down or unloads it.
During its lifetime, the runtime may call any of its exposed methods.
The module uses the stored `api` pointer to call other modules.

#### Calling other modules

Modules call each other through the stored `LogosAPI` pointer:

Legacy pattern caller:
```cpp
logosAPI->getClient("other_module")
        ->invokeRemoteMethod("other_module", "methodName", arg…);
```

Universal pattern caller (typical, Universal-on-Universal or when SDK exposes it):
```cpp
LogosAPI::callModule("other_module", "methodName", {arg1, arg2});
```

Payment streams integration (Step 8, D6): a Universal `payment_streams_module`
calling Legacy `logos_execution_zone` uses the same dispatch as Legacy callers on
the pinned SDK: `modules().api->getClient("logos_execution_zone")->invokeRemoteMethod(...)`.
See [`docs/step8-universal-legacy-probe-results.md`](docs/step8-universal-legacy-probe-results.md).

The called module receives the invocation through Qt's meta-object system.
It does not see which module called it, only that the runtime is invoking a method.
Both patterns may also use `logos-cpp-generator`-produced typed wrappers
for more convenient calling.

No C FFI is involved at Boundary B.
The runtime marshals calls across `logos_host` subprocesses.
Data is copied across process boundaries, not shared.

### Why payment_streams_module needs both boundaries

`payment_streams_module` uses Boundary A to call into
`lez-payment-streams-core` Rust library (`lez_payment_streams_core` crate)
(PDA derivation, stream folding, proof canonicalization, instruction encoding).

It uses Boundary B for all external communication.
Calling `logos_execution_zone` for chain access is Boundary B.
Being called by `delivery_module` as an eligibility verifier is also Boundary B.
The runtime mediates both directions.


## Qt's three roles

Qt is in this stack for three independent reasons.

1. Plugin loading and introspection.
   At build time, metadata is embedded into the module binary
   so the runtime can read it without loading the plugin code yet.
   Methods are registered in Qt's meta-object system
   so `liblogos_core` can enumerate and call them by name at runtime.
   With the legacy pattern, you write this registration manually
   using `Q_PLUGIN_METADATA` and `Q_INVOKABLE`.
   With the universal pattern, `logos-cpp-generator` creates it for you.

2. Inter-process IPC via Qt Remote Objects.
   Cross-module calls are marshaled between `logos_host` subprocesses
   using Qt Remote Objects.
   Legacy modules call via `LogosAPIClient::invokeRemoteMethod()`.
   Universal modules may call via `LogosAPI::callModule()` when the SDK supports it;
   this integration uses `invokeRemoteMethod` for Universal → Legacy wallet (Step 8).

3. UI framework (QML, QtQuick, signals and slots in views).
   Only `ui_qml` modules touch this.

A backend `core` module uses Jobs 1 and 2
and is unaware of Job 3 existing.

## C++ in this stack

C++ is the lingua franca at the module surface layer.
Even though most cryptographic and protocol logic
lives in Rust or Nim behind a C FFI,
every module exposes a C++ surface to the runtime.

What you write depends on the pattern:

Legacy pattern:
- The plugin class itself (subclass of `QObject` plus `PluginInterface`,
  with manual `Q_PLUGIN_METADATA` and `Q_INTERFACES`)
- `initLogos(LogosAPI*)` to receive the API pointer
- `Q_INVOKABLE` method bodies using Qt types (`QString`, `QByteArray`, `QVariantMap`)
- Type translation between Qt types and FFI C types
- Cross-module calls via `LogosAPIClient::invokeRemoteMethod()`

Universal pattern:
- A pure C++ impl class with standard types (`std::string`, `std::vector<uint8_t>`)
- An `onInit(LogosAPI* api)` hook to receive the API pointer
- Cross-module calls via `LogosAPI::callModule()`
- The generator produces Qt glue and type conversions automatically

Both patterns use `LogosResult` for error handling and require
FFI error enum translation at the C++ surface.

`logos-rln-module/src/logos_rln_module.cpp` at roughly 500 lines
is a working precedent for the scale to expect with the legacy pattern.

## Type conversion responsibilities

The runtime centralizes the parts whose contract is uniform across all modules:

- Qt types (`QString`, `QByteArray`, `QVariantMap`, `LogosResult`)
  cross process boundaries through Qt's meta-object system automatically.
- Typed cross-module wrappers come from `logos-cpp-generator`.
- `LogosResult` carries success/error/value uniformly.

Each module still owns:

- Hex `QString` ↔ `QByteArray` helpers
  (10-line `hexToBytes` / `bytesToHex`).
- `QByteArray` ↔ `const uint8_t*` + `size_t` for FFI inputs.
- Fixed-size out-buffers ↔ `QByteArray`.
- FFI error enums to `LogosResult` (the enum is per-FFI).
- FFI-owned heap buffers to `QByteArray` plus explicit FFI free calls.

#### Error handling with `LogosResult`

All cross-module calls return `LogosResult`.
Check `success()` to determine if the call succeeded.
On success, access the return value via `data()`.
On failure, get the error message via `errorMessage()`.

`LogosResult` carries success, error, or value uniformly across both patterns.
The runtime handles the marshaling; you only check the result.

Centralization stops at the FFI boundary
because each FFI has its own types and ownership rules.

## Chain side

`logos-scaffold` is a Rust CLI (`lgs` alias) that bootstraps
and runs a LEZ standalone localnet.
`lgs localnet start` brings up exactly one process:
the LEZ sequencer on `127.0.0.1:3040`.

The scaffold's scope statement is explicit
(from its README, section "Scope"):
no L1 dependency,
no indexer,
no full-stack circuits management.
`getAccount` against the sequencer at `:3040`
is the read path used by `logos_execution_zone`,
matching `docs/step1-findings-scaffold-rpc.md`.

The LEZ indexer exists in the broader ecosystem but never runs for this integration.

## Messaging side

The Logos messaging network is the libp2p-based peer fabric
that `delivery_module` joins via the `logos.dev` preset
(currently the public Logos dev network).
Two local instances of `delivery_module` on the same host
need disjoint `portsShift` values to avoid colliding
on TCP, discv5, REST, metrics, and websocket ports.
This is the workaround used by `logos-delivery-demo`
while `logos-delivery-module#18` is open.

No local messaging-side service runs;
peer discovery and gossip happen against the public dev network.

## Module Implementation Patterns

Two patterns exist for implementing Logos Core modules.
Understanding both matters because they may not be runtime-compatible.

### Legacy Pattern (PluginInterface)

You write a Qt plugin manually.
The class inherits from `QObject` and `PluginInterface`,
uses `Q_PLUGIN_METADATA` and `Q_INTERFACES`,
and exposes methods via `Q_INVOKABLE` using Qt types (`QString`, `QByteArray`, `QVariantMap`).
Cross-module calls use `LogosAPIClient::invokeRemoteMethod()`.

Used by existing modules: `logos-rln-module`, `logos-delivery-module`, `logos_execution_zone`.
Documented in `logos-tutorial/logos-developer-guide.md`.

### Universal Pattern (LogosProviderBase)

You write pure C++ with standard types (`std::string`, `std::vector<uint8_t>`, `LogosMap`).
`logos-cpp-generator --from-header` produces the Qt glue automatically.
The impl class has no Qt dependencies.
Cross-module calls use `LogosAPI::callModule()`.
Enabled by `"interface": "universal"` in `metadata.json`.

Defined in `logos-dev-boost/guidelines/universal-module.md`.
Part of the `logos-module-builder` ecosystem with code generation.

#### logos-cpp-generator

`logos-cpp-generator` is the tool that enables the universal pattern.
It is provided by `logos-module-builder` and runs during the Nix build
(`preConfigure` phase in `flake.nix`).

Input: `src/my_module_impl.h` — pure C++ class with public methods
Output: `generated_code/my_module_qt_glue.h`, `my_module_dispatch.cpp`

The generator produces:
- Qt plugin registration (`Q_PLUGIN_METADATA`, `Q_INTERFACES`)
- `Q_INVOKABLE` method wrappers
- Type conversions (`std::string` ↔ `QString`, `std::vector<uint8_t>` ↔ `QByteArray`)
- `LogosResult` success/error wrapping

Without the generator, you would write all Qt boilerplate manually —
which is exactly what the legacy pattern requires.
The generator is what makes "pure C++ with no Qt" possible for universal modules.

### Compatibility Warning

The base classes `PluginInterface` (legacy) and `LogosProviderBase` (universal)
are not interchangeable.
A runtime that expects one may crash when loading the other.
Per `logos-ai-skills/skills/logos-core-module-builder/`:
always confirm partner module patterns before assuming cross-module calls will work.
For `payment_streams_module`, check `delivery_module` and `logos_execution_zone`
at Step 6 implementation time to decide which pattern to use.

### Build Infrastructure vs Implementation Pattern

`logos-module-builder` (`logos-module-builder/` in the workspace)
is build infrastructure that supports both patterns.
It provides `mkLogosModule`, `logos_module()` CMake macro,
and `metadata.json` as the single source of truth.

Adopting `logos-module-builder` for builds does NOT mean adopting the universal pattern.
The actual indicator of the universal pattern is
`"interface": "universal"` in `metadata.json`
plus the presence of `logos-cpp-generator --from-header` in `flake.nix`.

Example: commit `1a4b7a4` in `logos-execution-zone-module`
("use module-builder") modernized the build infrastructure
while keeping the legacy `PluginInterface` implementation.
When checking partner modules for pattern compatibility,
look for `"interface": "universal"` in their `metadata.json`,
not merely for `logos-module-builder` usage.

## Reading map

| Question | Start here |
| --- | --- |
| How modules and hosts fit together | `logos-tutorial/logos-developer-guide.md` and `logos-liblogos/README.md` |
| LEZ sequencer, wallet, account model | `logos-execution-zone/README.md` |
| Logos Core module shape and packaging | `logos-tutorial/logos-developer-guide.md` Parts 1–4 |
| Wrapping a Rust crate as a module | `logos-lez-rln/logos-rln-module/` |
| Wrapping a Nim implementation as a module | `logos-delivery-module/` |
| `ui_qml` plugin pattern | `logos-delivery-demo/` and the journey doc |
| Standalone LEZ localnet | `scaffold/README.md` and `docs/step1-findings-scaffold-rpc.md` |
