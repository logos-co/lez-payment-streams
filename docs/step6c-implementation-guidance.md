# Step 6c Implementation Guidance

This document records the architectural decisions for implementing the Logos Core module shell in Step 6c.
It serves as the single source of truth for component selection, pattern choice, and implementation approach.

## Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Build infrastructure | `logos-module-builder` | Stable, well-tested, provides `mkLogosModule` and packaging |
| Implementation pattern | Legacy `PluginInterface` | Both dependencies use legacy; avoids runtime incompatibility |
| Code generation | Not used | `logos-cpp-generator` produces `LogosModules` wrapper that crashes in core sidecars (see Issue #31) |
| Cross-module calls | Raw `invokeRemoteMethod` | Safe, battle-tested, matches `logos-rln-module` precedent |
| AI guidance source | `logos-ai-skills` | Already installed in workspace, provides task-specific guidance |

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

- Step 6c uses `invokeRemoteMethod` only inside startup code (typically `initLogos`) to prove `getClient` and dispatch work; keep the public plugin surface to `initLogos`, `name`, and whatever `PluginInterface` requires. Step 7 adds `Q_INVOKABLE` helpers that wrap wallet reads.

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
| Universal pattern (`"interface": "universal"`) | Runtime incompatible with `PluginInterface` dependencies |
| `LogosProviderBase` | Base class for universal pattern only |
| `LogosAPI::callModule()` | Only available in universal pattern |
| `logos-dev-boost` templates | Target universal pattern by default |

## Cross-Module Call Pattern

Use raw `invokeRemoteMethod` (same as `logos-rln-module` and dependencies). For Step 6c, perform one probe from `initLogos` only; do not add extra exported methods on `payment_streams_module` for wallet access until Step 7.

```cpp
// payment_streams_module_plugin.cpp — startup plumbing only (Step 6c)
void PaymentStreamsModulePlugin::initLogos(LogosAPI* logosApiInstance) {
    m_logosApi = logosApiInstance;
    LogosAPIClient* walletClient =
        m_logosApi->getClient(QStringLiteral("lez_wallet_module"));
    const QVariant probe = walletClient->invokeRemoteMethod(
        QStringLiteral("lez_wallet_module"), QStringLiteral("list_accounts"));
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
- `logos-architecture-overview.md` Section "Module Implementation Patterns" - Pattern comparison
