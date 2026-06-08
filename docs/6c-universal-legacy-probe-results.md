# Step 6c-probe results (Universal to Legacy wallet)

Date: 2026-06-08 (E2E re-run after patched wallet `.lgx` + aligned LEZ)

Probe repository:
[`logos-universal-legacy-probe`](../../logos-universal-legacy-probe)
(runbook:
[`docs/probe-runbook-and-results.md`](../../logos-universal-legacy-probe/docs/probe-runbook-and-results.md)).

Automated run: `logos-universal-legacy-probe/scripts/run-e2e-probe.sh`
(logs under `/tmp/logoscore-probe-*.log`).

## Question

Can a Universal core module call the Legacy `lez_wallet_module` dynamically
(without listing the wallet in `metadata.json` dependencies)
without crashing `logoscore`?

## Verdict

Yes — Universal → Legacy in-process dispatch works for `list_accounts`.

- Both modules load; daemon stays up through pass A and pass B.
- No `Module name mismatch` in daemon logs with the rebuilt wallet `.lgx`.
- `probeStatus` returns `success:[]` (probe invoked wallet `list_accounts` via
  `invokeRemoteMethod` and received a valid `QJsonArray`).

Populated account lists on LEZ still need a wallet opened inside the module;
scaffold `storage.json` did not load via `open` in pass B (see below).

## Pass A — logoscore (no wallet open)

| Step | Result |
|------|--------|
| Build probe + patched wallet `.lgx` | OK (`nix build` / `nix bundle`) |
| `load-module lez_wallet_module` | `{"status":"ok"}` |
| `load-module universal_legacy_probe` | `{"status":"ok"}` |
| `call lez_wallet_module list_accounts` | `{"status":"ok","result":[]}` |
| `call universal_legacy_probe probeStatus` | `{"status":"ok","result":"success:[]"}` |

Load wallet before probe. Calling wallet RPC before `load-module` can destabilize
the daemon (observed segfault when loading probe after a pre-load timeout).

## Pass B — LEZ localnet + scaffold `open`

Environment: `logos-scaffold-workspace`, sequencer `127.0.0.1:3040` ready,
`lgs wallet check-health` OK.

| Step | Result |
|------|--------|
| `open` config + scaffold `storage.json` | RPC OK, `result`: 99; log: failed to load storage |
| `list_accounts` | `{"status":"ok","result":[]}` (null wallet handle) |
| `probeStatus` | `{"status":"ok","result":"success:[]"}` |

LEZ being up does not by itself fix storage: scaffold CLI wallet JSON storage is
not accepted by the module wallet FFI in this pin. Marshaling is still
validated because `list_accounts` and the probe path complete without replica
timeout.

## Patched wallet flake (build fixes)

Path:
`logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/`

- LEZ rev `c37a3c30…` aligned with logos-execution-zone-module PR 19 HEAD
- `lez-python-overlay`: `python3` for pyo3, `lez/wallet-ffi/wallet_ffi.h` install
- Packaging: `lez_wallet_module` id, `PluginInterface::name()` patch, plugin symlink
- `nix build .#lib` and `nix bundle … .#lib -o wallet-lgx-out` succeed (Linux x86_64)

## SDK note

Probe uses `modules().api->getClient("lez_wallet_module")->invokeRemoteMethod`,
not `LogosAPI::callModule` (not on pinned `logos-cpp-sdk`).

## Implication for `payment_streams_module`

Step 6c plumbing is satisfied for dynamic Universal → Legacy wallet calls.
Universal payment streams remains a separate product choice (delivery boundary,
tutorial pattern): `docs/universal-vs-legacy-dilemma.md`.

Follow-up for funded LEZ tests: open or create wallet storage compatible with
module FFI (not only scaffold CLI JSON), then re-run pass B `list_accounts`.

## Artifacts

- Wallet `.lgx`: `wallet-lgx-out/` after `nix bundle` in patched flake
- Probe `.lgx`: `nix build .#lgx` in `logos-universal-legacy-probe`
- Default `MODULES`: `/tmp/logos-probe-modules-isolated`
- LEZ: `lez-related/logos-scaffold-workspace`
