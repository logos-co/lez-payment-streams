# Legacy payment streams plugin bootstrap (archived)

Status: outdated (2026-06-08). Superseded by Step 9 in
[`program-index.md`](../development-map/program-index.md) and
[`docs/archive/steps/logos-runtime-guide.md`](../archive/steps/logos-runtime-guide.md).

This records the original Legacy bootstrap plan (pre–Step 9): a Legacy `PluginInterface` Qt plugin
(`payment_streams_module_plugin.{h,cpp}`, `i_payment_streams_module.h`) with
`logos_execution_zone` listed in `metadata.json` dependencies.

We chose the Universal interface instead after
[`docs/archive/steps/universal-legacy-probe-results.md`](../archive/steps/universal-legacy-probe-results.md)
(Step 8).

## What the Legacy shell contained

- `metadata.json` without `"interface": "universal"`; `dependencies: ["logos_execution_zone"]`
- Qt plugin implementing `PluginInterface` and `Q_INVOKABLE initLogos`
- Wallet plumbing via `getClient("logos_execution_zone")` and `invokeRemoteMethod`
  inside `initLogos`
- `flake.nix` with `mkLogosModule`, `lez_payment_streams_ffi` external lib,
  optional `logos_execution_zone` flake input for builder dependency wiring
- Linked `liblez_payment_streams_ffi`; no instruction entrypoints called from C++
  until chain-write steps

## Why it was retired

- Universal → Legacy dynamic wallet calls are validated; no need to stay Legacy
  for downstream wallet access.
- Typed `modules().logos_execution_zone` remains unsafe in core sidecars (Issue 31);
  Universal with empty dependencies plus dynamic `invokeRemoteMethod` matches
  the probe pattern.
- Legacy plugin sources were a thin scaffold (~90 lines), not product logic.

## Legacy implementation guidance (detail)


| Decision | Choice | Rationale |
|----------|--------|-----------|
| Build infrastructure | `logos-module-builder` | Stable, well-tested, provides `mkLogosModule` and packaging |
| Implementation pattern | Legacy `PluginInterface` | Downstream dependency uses legacy; avoids untested mixed-pattern routing |
| Code generation | Not used | `logos-cpp-generator` produces `LogosModules` wrapper that crashes in core sidecars (see Issue #31) |
| Cross-module calls | Raw `invokeRemoteMethod` | Safe, battle-tested, matches `logos-rln-module` precedent |
| AI guidance source | `logos-ai-skills` | Already installed in workspace, provides task-specific guidance |

## Required Updates (Legacy shell, archived)

### logosAPI Member Assignment

The module must assign to the inherited `logosAPI` member from `PluginInterface`, not a private `m_logosApi` member.
The host reads `logosAPI` directly to dispatch calls.
A separate member is invisible.

Current (incorrect) in `payment_streams_module_plugin.cpp`:

```cpp
void PaymentStreamsModulePlugin::initLogos(LogosAPI* logosApiInstance)
{
    m_logosApi = logosApiInstance;  // Wrong - host cannot see this
    ...
    LogosAPIClient* walletClient = m_logosApi->getClient(...);
}
```

Required fix:

```cpp
void PaymentStreamsModulePlugin::initLogos(LogosAPI* logosApiInstance)
{
    logosAPI = logosApiInstance;  // Correct - assign to inherited member
    ...
    LogosAPIClient* walletClient = logosAPI->getClient(...);
}
```

In `payment_streams_module_plugin.h`, remove:

```cpp
private:
    LogosAPI* m_logosApi = nullptr;  // Delete this line
```

Rationale: `logos-module-builder` commit `131faf1` ("fix logosAPI handling") updated templates to assign to the inherited `logosAPI` member.
The host's call dispatch mechanism reads this member directly.
Private shadow members cause silent failures.

Verification after fix:

```bash
nix build ./logos-payment-streams-module#lgx
lgpm --modules-dir "$MODULES" install --file "$REPO/result"/*.lgx
logoscore -D -m "$MODULES" -l logos_execution_zone,payment_streams_module -v
logoscore call logos_execution_zone list_accounts  // Should not crash
```

## Components to Use

### Build and Packaging

- `logos-module-builder` via Nix flake input
  - Provides `mkLogosModule` function
  - Handles `.lgx` packaging
  - Manages Qt 6 and SDK dependencies

- `with-external-lib` template from `logos-module-builder`
  - Pre-configured for wrapping external libraries (our Rust FFI)
  - Includes CMake setup for linking `liblez_payment_streams_ffi`

### C++ Implementation

- Legacy pattern classes
  - Inherit from `QObject` and `PluginInterface`
  - Use `Q_PLUGIN_METADATA` and `Q_INTERFACES` macros
  - Implement `initLogos(LogosAPI*)` manually

- `Q_INVOKABLE` methods with Qt types
  - `QString` for strings
  - `LogosResult` for return values
  - `QByteArray` for binary data

- Legacy bootstrap used `invokeRemoteMethod` only inside startup code (typically `initLogos`) to prove `getClient` and dispatch work; keep the public plugin surface to `initLogos`, `name`, and whatever `PluginInterface` requires. Modern Step 11a adds helpers that wrap wallet reads.

### FFI Integration

- Vendored `lez_payment_streams_ffi.h`
  - Copy from `lez-payment-streams-ffi/lez_payment_streams_ffi.h`
  - Include in `src/` directory

- Link against `liblez_payment_streams_ffi`
  - Shared library per platform (`.so`, `.dylib`, `.dll`)
  - Listed in `metadata.json` `include` array

## Components to Avoid

| Component | Reason |
|-----------|--------|
| `logos-cpp-generator` | Generates code that crashes in core module sidecars (Issue #31) |
| `LogosModules` typed wrapper | Segfaults during construction in `LogosAPI::getClient` |
| Universal pattern (`"interface": "universal"`) | Mixing patterns requires untested dynamic routing (`callModule` to `PluginInterface`) |
| `LogosProviderBase` | Base class for universal pattern only |
| `LogosAPI::callModule()` | Only available in universal pattern |
| `logos-dev-boost` templates | Target universal pattern by default |

## Cross-Module Call Pattern

Use raw `invokeRemoteMethod` (same as `logos-rln-module` and dependencies). For Legacy bootstrap, perform one probe from `initLogos` only; do not add extra exported methods on `payment_streams_module` for wallet access until Step 11a.

```cpp
// payment_streams_module_plugin.cpp — startup plumbing only (Legacy bootstrap, archived)
void PaymentStreamsModulePlugin::initLogos(LogosAPI* logosApiInstance) {
    m_logosApi = logosApiInstance;
    LogosAPIClient* walletClient =
        m_logosApi->getClient(QStringLiteral("logos_execution_zone"));
    const QVariant probe = walletClient->invokeRemoteMethod(
        QStringLiteral("logos_execution_zone"), QStringLiteral("list_accounts"));
    (void)probe;
}
```

Declare `initLogos` as `Q_INVOKABLE` in the plugin header. Current `PluginInterface` does not always expose a virtual `initLogos`, so do not mark it `override` unless your pinned `logos-module` headers actually declare one.

Step 7 repeats the same `getClient` plus `invokeRemoteMethod` pattern inside dedicated helpers (for example wrappers around `get_account_public`) exposed as `Q_INVOKABLE` methods.

## References

- `logos-rln-module/src/logos_rln_module.cpp` - Working precedent for legacy pattern + FFI wrapping
- `logos-delivery-module/src/delivery_module_plugin.h` - Dependency module (legacy pattern)
- `logos-execution-zone-module/src/logos_execution_zone_wallet_module.h` - Dependency module (legacy pattern)
- GitHub Issue #31 in `logos-delivery-module` - Documents `LogosModules` wrapper crash in core sidecars
- [logos-architecture-overview.md](../archive/reference/logos-architecture-overview.md) Section "Module Implementation Patterns" - Pattern comparison
