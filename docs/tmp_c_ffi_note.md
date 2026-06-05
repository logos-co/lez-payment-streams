FIXME: we need to adapt this document in view of integration plan v2.
What changes for architecture based on Logos Core modules?


## Summary

- C FFI is a binary interface contract based on the C ABI.
- It is not about writing code in C.
- It is about exposing callable symbols with C-compatible signatures so different languages can interoperate safely at compiled-binary level.

## What C FFI means in Rust and Nim interop

- Rust exposes functions with `extern "C"` and stable symbol names.
- These functions use C-compatible types at the boundary.
- Nim declares matching external functions and calls them.
- If ABI matches, calls work reliably across language boundary.
- If ABI mismatches, you get runtime failures or corrupted data.

## Preferred approach in Logos stack

Given mixed Rust and Nim components, the practical pattern is:

- keep cryptographic and transaction-critical semantics in Rust LEZ/NSSA code
- expose narrow capabilities through C FFI
- let Nim-side `logos-delivery` orchestrate protocol logic, policy, and flow

This aligns with the generic wallet FFI direction in
[logos-execution-zone PR 491](https://github.com/logos-blockchain/logos-execution-zone/pull/491)
and the module bridge in
[logos-execution-zone-module PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19).
Superseded: [PR 429](https://github.com/logos-blockchain/logos-execution-zone/pull/429),
[PR 16](https://github.com/logos-blockchain/logos-execution-zone-module/pull/16).

491 illustrates “resolve accounts + program ELF bundle + instruction words, submit once”
rather than per-program bespoke wallet FFI wrappers.

## Impact on payment streams work

For payment streams, this strongly suggests:

- do not reimplement NSSA account/signature semantics in Nim
- use Rust as source of truth for:
  - `PublicKey -> AccountId` mapping
  - signature verification semantics and encoding
  - transaction signing/submission primitives where applicable
- keep Nim responsible for:
  - Store request lifecycle
  - payment policy checks
  - pending proposal/session state
  - LEZ RPC reads and decision wiring

## Preferred way forward

- Define a minimal FFI surface for crypto-critical operations needed by Step 2.
- Reuse generic transaction submission path for on-chain public writes where available.
- Lock behavior with cross-language test vectors (key bytes, account id outputs, canonical payload bytes, signature pass/fail cases).
- Keep the FFI boundary small and stable; keep higher-level business logic in Nim.

This gives you consistency with LEZ/NSSA, avoids duplicated crypto logic, and fits the existing Logos architecture trajectory.