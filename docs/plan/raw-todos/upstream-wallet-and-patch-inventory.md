# Raw TODO — upstream alignment and patch inventory

Ideas not yet scheduled as a plan step. Index: [../index.md](../index.md).

## Goal

Simplify or unify lez-payment-streams dependencies to use **upstream** wallet/module builds where
they are sufficient, and maintain an explicit list of **patch-only** behavior for discussions with
logos-execution-zone / logos-execution-zone-module maintainers.

## Context

Step 26 pins LEZ v0.2.0 and wallet module on upstream `main`. Generic public program transactions
(`send_generic_public_transaction`, `send_generic_public_transaction_json`) exist upstream.

This repo still builds wallet via
`logos-execution-zone-module-patched` (portable `.lgx`, codegen headers, Nix env).

## Patch-only surface (enumerate for upstream)

| Patch / wrapper | What it adds | Needed for `MODE=module` user journey? |
| --- | --- | --- |
| `wallet-qt-send-generic-public-transaction-json.patch` | JSON IPC wrapper for instruction submit | Largely redundant with upstream `main`; verify and drop if patch applies cleanly or is no-op |
| `wallet-qt-sign-public-payload.patch` + `lez-rust-sign-public-payload.patch` | `sign_public_payload` Qt + `wallet_ffi_sign_public_payload` | **No** — Store eligibility only |
| `addSdkApiHeaders` / portable bundler in wrapper flake | Build `payment_streams_module` static dep + lgpm-compatible LGX | Build/packaging, not chain semantics |
| `lez-wallet-ffi-patched` | LEZ pin, python3/pyO3, LBC paths, header install | Reproducible Nix build |

## Proposed work (when promoted to a step)

1. Document canonical consumer path: upstream wallet at v0.2.0 + `lgpm install` when variants match.
2. Try E2E with catalogue or upstream-built wallet only (module-only matrix).
3. Open upstream issues/PRs for: `sign_public_payload` on wallet module + FFI (if eligibility stays
   in-tree); confirm JSON submit API ownership.
4. Remove redundant patches after upstream parity is confirmed.

## Out of scope for first pass

- Replacing patched wallet in Store/eligibility runs before upstream exposes signing APIs.
- Changing LIP-155 wire names.
