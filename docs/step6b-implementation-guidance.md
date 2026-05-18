# Step 6b Implementation Guidance

This document records the architectural decisions for implementing the Logos Core module shell in Step 6b.
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

- Direct `invokeRemoteMethod` calls
  ```cpp
  LogosResult result = logosAPI->getClient("lez_wallet_module")
      ->invokeRemoteMethod("lez_wallet_module", "get_account_public", accountIdHex);
  ```

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

The safe pattern for calling `lez_wallet_module` from `payment_streams_module`:

```cpp
// In payment_streams_module_plugin.h
class PaymentStreamsModulePlugin : public QObject, public PluginInterface {
    Q_OBJECT
    Q_PLUGIN_METADATA(IID PluginInterface_iid FILE "metadata.json")
    Q_INTERFACES(PluginInterface)

public:
    Q_INVOKABLE void initLogos(LogosAPI* api) override {
        logosAPI = api;
    }

private:
    LogosAPI* logosAPI = nullptr;
};

// In payment_streams_module_plugin.cpp
LogosResult PaymentStreamsModulePlugin::callWalletGetAccount(
    const QString& accountIdHex) {
    return logosAPI->getClient("lez_wallet_module")
        ->invokeRemoteMethod("lez_wallet_module", "get_account_public", accountIdHex);
}
```

## References

- `logos-rln-module/src/logos_rln_module.cpp` - Working precedent for legacy pattern + FFI wrapping
- `logos-delivery-module/src/delivery_module_plugin.h` - Dependency module (legacy pattern)
- `logos-execution-zone-module/src/logos_execution_zone_wallet_module.h` - Dependency module (legacy pattern)
- GitHub Issue #31 in `logos-delivery-module` - Documents `LogosModules` wrapper crash in core sidecars
- `logos-architecture-overview.md` Section "Module Implementation Patterns" - Pattern comparison
