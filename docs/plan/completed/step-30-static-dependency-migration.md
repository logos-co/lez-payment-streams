# Step 30 — static dependency migration

> Status: complete (2026-07-01). `payment_streams_module` lists
> `logos_execution_zone` in `metadata.json` `"dependencies"` and the ~20+
> wallet call sites use codegen-emitted typed
> `modules().logos_execution_zone` wrappers (Qt-free `lp` API style). Three
> repo-local / complex-type methods stay on a minimal dynamic-dispatch
> fallback through `modules().api` — see Findings. D6's revisit condition is
> closed.
>
> Verification matrix:
>
> | Gate | Result |
> |------|--------|
> | `nix build ./logos-payment-streams-module#lgx` | succeeds |
> | `MODE=module CHAIN=local ./scripts/e2e.sh local run` | all phases pass, incl. `claim` |
> | `MODE=store CHAIN=local ./scripts/e2e.sh local run` | `store_query_success ok=True`, `demo_close_stream ok=True`, `demo_claim ok=True` |
> | `make verify-step17-back-to-back` | two Store runs on one ledger (stream 0 accrued 200, stream 1 accrued 750), exit 0 |

Completed packet. Index: [index.md](../index.md).

### Step 30, static dependency migration

Migrate `payment_streams_module`'s wallet-call surface from dynamic
`api->getClient("logos_execution_zone")->invokeRemoteMethod(...)` dispatch to
codegen-emitted typed `modules().logos_execution_zone()` wrappers, now that the
wallet module is Universal on upstream `main`.

This closes [D6](../../reference/integration-decisions.md#d6-universal-module-interface)
line 224's revisit condition ("Revisit a static dependency when the wallet
module is Universal upstream and codegen supports it"). The condition was met
in Step 26, which moved the wallet wrapper flake from the closed PR 19
(Legacy) onto upstream `main` (Universal) and confirmed the wrapper builds
against the Universal wallet.

Prerequisite: [Step 26](step-26-testnet-v02-migration.md) complete — the wallet
wrapper flake must build against upstream `main` and the four wallet patches
must apply cleanly. This step does not depend on live testnet verification and
can parallelize with Steps 27-29.

#### Migration scope

| Component | From | To |
|-----------|------|-----|
| `metadata.json` `"dependencies"` | `[]` (dynamic dispatch) | `["logos_execution_zone"]` (static, codegen) |
| Wallet call sites (~20+) | `invokeWalletString(client, "method", arg)` and `invokeWalletMulti(...)` | Typed `modules().logos_execution_zone()->method(arg)` wrappers emitted by `logos-cpp-generator` |
| Codegen | Regenerated against the wallet module's typed API header | New `logos_execution_zone_*` wrappers in the PS module's generated sources |
| `walletClientOrNull()` helper | Returns `LogosAPIClient*` from `api->getClient(...)` | Removed or replaced by `modules().logos_execution_zone()` (codegen-owned) |

Files touched:

- `logos-payment-streams-module/metadata.json` (`"dependencies"` array)
- `logos-payment-streams-module/src/payment_streams_module_impl.{h,cpp}`
  (~10 call sites: `account_id_from_base58`, `get_account_public`,
  `get_public_account_key`, `sync_to_block`, `send_generic_public_transaction_json`,
  etc.)
- `logos-payment-streams-module/src/payment_streams_module_eligibility.{h,cpp}`
  (~10 call sites: `account_id_from_base58`, `get_account_public`,
  `get_public_account_key`)
- `logos-payment-streams-module/src/payment_streams_module_writes.cpp`
  (`submitGenericPublicViaFfi` wallet invocation)
- `logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/flake.nix`
  (if codegen input paths need adjustment for the typed API header)

Out of scope:

- The JSON wrapper patch
  (`wallet-qt-send-generic-public-transaction-json.patch`) stays. The blocker
  for dropping it is RISC0 serialization (`wallet_ffi_serialization_helper`
  not exposed as a Q_INVOKABLE), not IPC shape. See
  [N10](../../reference/integration-decisions.md#n10-step-11b-module-writes-decisions).
- The `lez-testnet-submit` helper retirement (deferred to live-testnet
  follow-up, see Step 26).

#### Work branch and commit policy

Branch name: `refactor/static-dependency-migration`. Commit but do not push
until localnet gates pass.

Commit split (2 commits, by concern):

1. `metadata+codegen` — `metadata.json` dependency declaration + regenerated
   codegen sources.
2. `call-sites` — all `invokeWalletString` / `invokeWalletMulti` → typed
   wrapper conversions across the three `.cpp` files.

#### Deliver

- Add `"logos_execution_zone"` to `metadata.json` `"dependencies"` array.
- Regenerate codegen against the wallet module's typed API header (the
  `logos-cpp-generator` invocation in the wrapper flake's `addSdkApiHeaders`
  phase should now emit `modules().logos_execution_zone()` accessors).
- Convert every `invokeWalletString(client, "method", arg)` and
  `invokeWalletMulti(client, "method", a1, a2)` call site to the
  corresponding typed wrapper. The conversion must preserve:
  - Argument marshaling semantics (QString vs QByteArray vs QList<uint8_t>).
  - Error handling: `invokeWalletString` returns `{}` on null client or
    invalid result; typed wrappers may return `std::optional<T>` or throw.
    Match the existing empty-string-as-error contract or adjust callers
    uniformly.
- Remove `walletClientOrNull()`, `invokeWalletString()`, and
  `invokeWalletMulti()` helpers from both `payment_streams_module_impl.cpp`
  and `payment_streams_module_eligibility.cpp` once all call sites are
  converted.
- Audit for any wallet methods we call that are **not** on the typed API
  surface (e.g., patched methods like `send_generic_public_transaction_json`
  and `sign_public_payload`). These will require either:
  - Keeping a dynamic-dispatch fallback for patched methods only, or
  - Extending the codegen input to include the patched header signatures.
  Flag the chosen approach in the step packet's findings.

#### Verification gates

| Gate | Command | Pass Criteria |
|------|---------|---------------|
| Module bundle | `nix build ./logos-payment-streams-module#lgx` | `.lgx` builds with static dependency + regenerated codegen |
| Localnet E2E (module) | `MODE=module CHAIN=local ./scripts/e2e.sh local run` | All phases succeed, including `claim` |
| Localnet E2E (store) | `MODE=store CHAIN=local ./scripts/e2e.sh local run` | Store integration dual-host happy path |
| Localnet lifecycle | `make verify-step17-back-to-back` | Two Store runs on one ledger |
| Pin sweep (no-op) | `grep -rl 27360cb7 . ...` | Unchanged from Step 26 (no pin bump in this step) |

#### Definition of done

- [x] `"logos_execution_zone"` listed in `metadata.json` `"dependencies"`
- [x] Codegen regenerated; `modules().logos_execution_zone()` wrappers
  compile and link
- [x] All `invokeWalletString` / `invokeWalletMulti` call sites converted to
  typed wrappers (~20+ sites across three `.cpp` files)
- [x] `walletClientOrNull`, `invokeWalletString`, `invokeWalletMulti`
  helpers removed (reduced to a minimal patched-method fallback — see Findings)
- [x] Patched-method handling (`send_generic_public_transaction_json`,
  `sign_public_payload`) documented — dynamic fallback (see Findings)
- [x] `nix build ./logos-payment-streams-module#lgx` succeeds
- [x] `MODE=module CHAIN=local ./scripts/e2e.sh local run` passes
- [x] `MODE=store CHAIN=local ./scripts/e2e.sh local run` passes
- [x] `make verify-step17-back-to-back` passes
- [x] [D6](../../reference/integration-decisions.md#d6-universal-module-interface)
  amended: revisit condition marked resolved, dynamic-dispatch rationale
  archived as Legacy-specific
- [x] [feature-branch-pins.md](../../reference/feature-branch-pins.md)
  updated: note the static dependency migration is complete

#### Findings

Static dependency migration landed on the `lp` (Qt-free, `std::string`-based)
codegen API style. The module's `logos-cpp-generator` invocation emits
`LogosExecutionZone` with `std::string` parameters and returns; the generated
`logos_sdk.h` holds a `LogosExecutionZone logos_execution_zone` member (direct
reference, not an accessor call: `modules().logos_execution_zone`). Call sites
therefore convert `QString` → `std::string` (`.toStdString()`) on the way in
and `std::string` → `QString` (`QString::fromStdString(...)`) on the way out.
The empty-string-as-error contract from `invokeWalletString` is preserved: a
typed wrapper returning an empty `std::string` is treated as failure, matching
the legacy helper's `{}` return on null client or invalid result. None of the
typed wrappers used here surface `logos::CallError*` out-parameters to callers;
the empty-string check is sufficient for the existing call shapes.

Three wallet methods could not move to the typed wrapper and stay on a
minimal dynamic-dispatch fallback routed through `modules().api` (the Qt
`LogosAPI*` / `LogosAPIClient*` path). Each file keeps a file-local
`walletQtClientOrNull(LogosAPI*)` + `invokeWalletQtString(...)` pair in its
anonymous namespace for these calls only:

- `sign_public_payload` (N1) — repo-local Qt-only patch; not in the codegen
  input header, so no typed wrapper exists. Used by `signVaultOwnerDigest`
  in `payment_streams_module_eligibility.cpp`.
- `send_generic_public_transaction_json` (N10) — repo-local Qt-only patch.
  Used by `submitGenericPublicViaFfi` in `payment_streams_module_writes.cpp`.
  The multi-arg `send_generic_public_transaction` fallback in the same
  function also dispatches dynamically: the `lp` typed wrapper would require
  marshaling `LogosList` / `LogosMap` (`nlohmann::json`) for the
  `account_ids` / `signing_requirements` / `instruction` / `program_id_hex`
  arguments, and the existing caller already shapes these as `QVariant`
  lists for the Qt path. Keeping both submits on dynamic dispatch avoids a
  `QList<uint8_t>` ↔ `LogosList` marshaling layer for this MVP write path
  without changing the wire payload.
- `authenticated_transfer_elf` — present in the `lp` typed API, but returns
  `LogosMap` (`nlohmann::json`). The existing caller
  (`walletAuthenticatedTransferElfBytes`) does `QVariant`-shaped byte
  extraction (`QList<uint8_t>` / `QByteArray` / `QStringList`) from the
  response to assemble the deposit dependency ELF. Routing this through the
  `lp` wrapper would require a `LogosMap` → `QList<uint8_t>` decode that
  duplicates the Qt path's parsing. Dynamic dispatch preserves the existing
  byte-extraction shape unchanged.

All remaining wallet calls (`account_id_from_base58`, `get_account_public`,
`get_public_account_key`, `sync_to_block`, etc.) use the typed
`modules().logos_execution_zone` wrapper with `QString` ↔ `std::string`
conversions at the boundary. The `walletClientOrNull` / `invokeWalletString` /
`invokeWalletMulti` / `invokeWalletTwo` helpers are removed; the file-local
Qt fallback helpers are the only dynamic-dispatch surface left, scoped to the
three methods above. The JSON wrapper patch
(`wallet-qt-send-generic-public-transaction-json.patch`) is unchanged, in line
with the Out-of-scope note: the blocker for dropping it is RISC0 serde, not IPC
shape.

#### Non-regression guard

Localnet verification must keep passing throughout:

```bash
make verify-step17
make verify-step17-back-to-back
```

The wallet call surface must behave identically before and after the
migration. The JSON wrapper patch and Path 1 patch shape are unchanged by
this step.

#### Related

- [step-26-testnet-v02-migration.md](step-26-testnet-v02-migration.md) —
  prerequisite; moved wallet wrapper to Universal `main`
- [D6](../../reference/integration-decisions.md#d6-universal-module-interface) —
  Universal module interface decision; revisit condition closed by this step
- [N10](../../reference/integration-decisions.md#n10-step-11b-module-writes-decisions) —
  JSON wrapper rationale (unchanged by this step; wrapper persists for RISC0
  serde)
- [feature-branch-pins.md](../../reference/feature-branch-pins.md) —
  dependency versions
