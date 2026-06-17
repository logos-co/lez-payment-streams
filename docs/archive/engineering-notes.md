# Engineering notes (misc)

## C FFI interop (draft)

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
Superseded paths: [superseded-wallet-pr-429-16.md](superseded-wallet-pr-429-16.md).

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

## SPEL PDA account seeds issue

`spel pda` does not reliably support IDL-defined PDAs that include `account(...)` seeds
in the current pinned version used via scaffold.
In practice, deriving `stream_config` for `lez-payment-streams` fails even though
IDL contains sufficient seed metadata.

# Context

Project context:
- repo: `lez-payment-streams`
- program deployed successfully
- IDL generation works and includes PDA seed definitions

Relevant IDL seed shape for `stream_config`:
- const `"stream_config"`
- account seed path `"vault_config"`
- arg seed path `"stream_id"`

Observed command style:
- `lgs spel -- --idl "$IDL_PATH" --program "$PROGRAM_ID" pda stream_config ...`

Expected:
- CLI computes PDA using IDL seed definitions and provided account/arg seed inputs

Actual:
- CLI appears to enter raw PDA mode and treats tokens as raw seed strings
- error example: `Seed '5iyX...xu8i' is 44 bytes, max 32`

# Reproduction

1. Generate IDL for a program whose PDA includes `account(...)` seed.
2. Deploy program and capture `program_id`.
3. Derive upstream PDA successfully for account without account-seed dependencies
   (for example `vault_config`).
4. Attempt to derive dependent PDA:

   `spel --idl <idl.json> --program <program_id_hex> pda stream_config --vault-config-account <base58-vault-config> --seed-arg 0`

5. Command fails with raw-seed length error instead of resolving account seed.

# Why this matters

- Blocks deterministic derivation of account-seeded PDAs from IDL
  in automated workflows.
- Forces callers to use ad-hoc fallback logic or wait for on-chain writes
  to reveal the account id indirectly.
- Undermines the intended value of IDL-driven tooling for PDA computation.

# Root cause hypothesis

Two issues likely combine here:

1. Mode routing ambiguity:
   when `--program <hex>` is supplied with `pda`, CLI routes into raw PDA mode
   instead of IDL PDA mode.

2. Missing account-seed input plumbing in IDL path:
   account seeds are resolved from an internal account map, but caller-provided
   account ids are not wired into that map for `compute_pda_from_seeds`.

# Proposed fix direction

1. Make mode selection explicit and predictable

- If `--idl` is present, `pda <account-name>` should always use IDL mode.
- Raw PDA derivation should require explicit marker, for example:
  `pda --raw --program <hex> <seed> ...`
  or only when `<account-name>` is omitted.

2. Add first-class account-seed inputs for IDL mode

- Accept account seed values as named flags and map them into account map.
- Keep arg seeds in parsed args as today.
- Validate account seed values as base58/hex account ids with clear errors.

3. Improve diagnostics and help

- On missing seed inputs, print required seeds grouped by kind:
  const, account, arg.
- Include concrete example command in error output for the current account name.
- Return success exit code for pure help invocations.

4. Add regression tests

- IDL PDA with const+arg seeds.
- IDL PDA with const+account+arg seeds.
- Case with `--idl` + `--program <hex>` must not enter raw mode.
- Ensure derived PDA matches on-chain/runtime derivation for fixture program.

# Suggested acceptance criteria

- `spel --idl <idl> --program <hex> pda stream_config --vault-config-account <id> --seed-arg 0`
  returns a base58 PDA and exit code 0.
- Same command without required account seed fails with actionable message
  listing required account seed names.
- Raw mode remains available but only through explicit invocation.

# Additional notes

- This issue is separate from recent `generate_idl!` path-dependency fixes.
- IDL generation itself can be correct while PDA derivation UX still fails
  for account-seeded PDAs.
