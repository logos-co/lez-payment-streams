# Raw TODO — upstream spel `generate_idl!` canonical paths

Ideas not yet scheduled as a plan step. Index: [../index.md](../index.md).

Related: commit that fixed local IDL generation (`make idl`), D7
([integration-decisions.md](../../reference/integration-decisions.md)).

## Problem

`spel_framework::generate_idl!` resolves the guest source path from
`CARGO_MANIFEST_DIR`, but emitted `include_str!(...)` dependency tracking used
paths relative to the **IDL binary source file**. With
`examples/src/bin/generate_idl.rs`, `../methods/guest/...` did not compile;
`make idl` also truncated `lez-payment-streams-idl.json` on failure.

## Local fix (in-repo)

- Moved the IDL binary to [examples/src/generate_idl.rs](../../../examples/src/generate_idl.rs)
  (path `../methods/guest/src/bin/lez_payment_streams.rs` from `examples/`).
- Hardened [Makefile](../../../Makefile) `idl` target (temp file + `mv` on success).
- Vendored patch in
  [vendor/spel-framework-macros/src/lib.rs](../../../vendor/spel-framework-macros/src/lib.rs):
  `canonicalize_path_str` for the guest path and for dependency `include_str!`
  paths in `expand_generate_idl`.

This is not upstream spel; it lives only in this repo’s vendor tree until
promoted or replaced by a spel release pin bump.

## Proposed upstream change

Open a PR to [logos-co/spel](https://github.com/logos-co/spel) (or the crate that
ships `spel-framework-macros`) to canonicalize paths in `generate_idl!` output
so `include_str!` works regardless of where the `generate_idl` binary source
lives under the examples crate.

Reference implementation: `canonicalize_path_str` in our vendored
`expand_generate_idl` (guest `resolved_path` + dep tracking loop).

After upstream lands, drop the vendor delta on the next spel pin bump and keep
only the examples path + Makefile guard if still useful.

## Verification

- `make idl` from repo root succeeds and matches committed
  `lez-payment-streams-idl.json`.
- Failed `generate_idl` compile does not clobber the IDL file.

## Promotion

Close this raw TODO when upstream merges and vendor patch is removed, or when
we decide to carry the fork indefinitely (document in D7 or a short note in
scripts/README).
