# Summary

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
