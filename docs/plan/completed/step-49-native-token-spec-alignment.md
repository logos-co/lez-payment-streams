# Step 49 — LIP-155 multi-token type alignment (native-only demo)

Index: [index.md](../index.md). Status: complete (2026-08).
Gate log: [step-49-gate-log.md](step-49-gate-log.md).

Types, wire, FFI, module, and living-doc delta landed on
`feat/step-49-native-token-spec-alignment`.
Fast verification, local-public, and testnet-public dogfood passed 2026-08-13.
Closes [Step 41](step-41-non-native-token-policy-spec.md) D41.6 for
types, wire, and policy shape.
Does not implement a non-native Token custody path.

Prerequisite: [Step 46](step-46-docs-unify-and-forum-post.md) (living docs already
on reproduce / integrate surfaces).
[Step 48](../wontfix/step-48-program-graph-lez-unify.md) is not a prerequisite.

Spec pin: LIP-155 `docs/anoncomms/raw/payment-streams.md` on `logos-lips` `master`
at a rev that contains [logos-lips#379](https://github.com/logos-co/logos-lips/pull/379)
(`f09f9e9e` or a later tip that also contains Step 40 `435a6f18`).
Update [feature-branch-pins.md](../../reference/feature-branch-pins.md) and
[context-manifest.json](../../context-manifest.json) in this step (D49.9).

## Goal

Align guest, core, FFI, module, and off-chain proofs with LIP-155 vault-token
identity and `StreamProviderPolicy.accepted_tokens` so the implementation matches
the published spec.

The runnable demo stays native-token only.
`chainAction` happy path, Store eligibility E2E, and operator commands do not
gain a token picker or a second asset path.
Callers that omit `token_id` keep today’s native behavior.

## Why this is an ImageID cut

LIP-155 requires:

- `VaultConfig.token_id` (32 octets; all-zeroes = LEZ native), fixed at init.
- `VaultHolding` PDA from `VaultConfig` address and `token_id` (32 raw octets).
- `VaultProof.token_id` REQUIRED, equal to on-chain identity.
- Vault-owner canonical Borsh body includes `token_id` after `service_id`.
- Policy minima live in `TokenStreamPolicy` rows inside `accepted_tokens`.

Today:

- `VaultConfig` has no `token_id`.
- `derive_vault_account_ids` seeds holding with `seed_from_str("native")`, not
  32 zero bytes.
- `VaultProofWire` has no field 5; protobuf comments still cite `ift-ts`.
- `StreamProviderPolicy` has top-level `min_rate` / `min_allocation`.
- `VaultOwnerAuthBorshBody` has no `token_id`.

Native demo addresses therefore change even though the asset is still native.

## Scope

In scope:

- `token_id` on `VaultConfig` (Borsh); `InitializeVault` records it; later ops
  MUST NOT change it.
- Guest rejects any `token_id` other than all-zeroes (D49.1).
- Holding PDA seed is the 32-byte `token_id` (all-zeroes for native) (D49.2).
- `VaultProof` protobuf field 5; encode/decode length check (32 octets).
- Canonical vault-owner digest includes `token_id` in LIP field order.
- `TokenStreamPolicy` + `StreamProviderPolicy.accepted_tokens`; predicates
  match `VaultProof.token_id` (always native in this step) to one row.
- FFI / cbindgen / module decoded structs and policy fill.
- `initializeVault` JSON: optional `token_id` (64-hex or base58). Omit or
  all-zeroes = native. Demo and E2E omit the key (D49.3).
- Spec path comments: `anoncomms`, not `ift-ts`.
- Single LIP citation pin (D49.9).
- Rebuild guest, local reset, testnet deploy, fixture sweep, dogfood (D49.8).
- Short living-doc delta on Step 46 surfaces: optional `token_id`, native
  default, guest rejects non-native. No new reproduce path.

Out of scope:

- Token program deposit / withdraw / claim (non-native custody).
- Module UI or E2E that selects a non-native token.
- Wrapped-native unification ([raw TODO](../raw-todos/wrapped-native-token-unification.md)).
- Discovery / advertised policy on the wire (Step 42 wontfix).
- Demo command or phase changes (still vault → deposit → create → close → claim).
- [Step 50](../upcoming/step-50-consistency-and-clarity.md) refactors (clock helper,
  C++ sharing, orchestrator extract).
- `lee_core` alias removal (Step 48 / spel, wontfix).
- Renaming `user-journey-*.sh` (D46.14).

## Demo contract (must not change)

| Surface | After this step |
| --- | --- |
| `initializeVault` keys used by E2E | still `owner`, `vault_id` (optional `privacy_tier`) |
| Deposit / create / close / claim JSON | unchanged |
| Store prepare / verify | still native vault; proof bytes now carry `token_id` = 32 zeroes |
| `MODE=module` / `MODE=store` recipes | same commands; new ImageID / fixtures |
| Provider demo policy minima | same numeric mins, stored as one native `accepted_tokens` row |

## Implementor order

1. Pin LIP rev (D49.9); fix `ift-ts` citations in core/off-chain comments.
2. Core types: `token_id` constant `NATIVE_TOKEN_ID = [0u8; 32]`;
   `VaultConfig`; `TokenStreamPolicy`; policy predicates + reject reason
   for unknown / missing token (append `PolicyRejectReason` only).
3. PDA: holding seed = `token_id` bytes; tests for native all-zeroes vs old
   `"native"` string (must differ).
4. Guest `InitializeVault` + account checks; reject non-native.
5. Off-chain: protobuf field 5; canonical body; proof verify that
   `VaultProof.token_id` equals on-chain vault token.
6. FFI + module: default native; optional JSON `token_id`; demo policy
   `accepted_tokens = [{ native, existing mins }]`.
7. Program tests + FFI tests: native happy path; non-native init fails;
   policy miss on wrong `token_id`.
8. `make build`; ImageID; `make full-reset-localnet`; local-public module +
   store; deploy-testnet; fixture sweep; testnet-public module + store.
9. Living docs delta (chainAction catalogue, contracts, pins, on-chain README).

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D49.1 | Non-native guest | Reject non-all-zero `token_id` at init and whenever the guest reads vault token identity. No Token program calls. |
| D49.2 | Holding PDA | Match LIP: seed is 32-byte `token_id`, not `seed_from_str("native")`. Native vaults move. ImageID cut. |
| D49.3 | Module JSON | Optional `token_id` on `initializeVault` only. Omit = native. E2E omits. Other ops do not take `token_id`. |
| D49.4 | Policy shape | Remove top-level `min_rate` / `min_allocation`. One `accepted_tokens` entry required; demo fills native + current mins. Max 16 entries in types; demo uses 1. |
| D49.5 | Wire / signatures | `VaultProof.token_id` required. Canonical Borsh order follows LIP (token_id after service_id). Old proofs invalid after cut. |
| D49.6 | Asset path | Native authenticated-transfer only. Non-native custody is a later product step. |
| D49.7 | Wrapped-native | Stays raw TODO. |
| D49.8 | Demo behavior | No new E2E phases or flags. Commands unchanged. ImageID, fixtures, and proof bytes change. |
| D49.9 | Spec pin | `context-manifest.json` `lip155_spec` becomes `logos-lips` `master` at a rev containing #379; drop feature-branch as sole SSOT. Prefer a tip that also contains Step 40. |
| D49.10 | Docs | Delta on Step 46 living surfaces only. Do not revive journey fronts. |
| D49.11 | Verification | ImageID-cut bar: `fast` + `local-public` (module + store) + `testnet-public` (module + store). Gate log records ImageID; do not paste hex into reproduce prose (D46.15). |

## Done when

- LIP `token_id` / `accepted_tokens` / `VaultProof.token_id` / canonical body
  exist in core, guest, FFI, and module with native defaults.
- Guest rejects non-native `token_id`.
- Holding PDAs use 32-byte `token_id` seeds.
- Demo and E2E omit `token_id` and still complete native happy path.
- Spec comments cite `anoncomms`; manifest pin updated (D49.9).
- Gate log: [step-49-gate-log.md](step-49-gate-log.md) with ImageID via README /
  fixtures citation, local-public and testnet-public artifacts.

## Related

- [step-41-non-native-token-policy-spec.md](step-41-non-native-token-policy-spec.md)
- [step-46-docs-unify-and-forum-post.md](step-46-docs-unify-and-forum-post.md)
- [step-50-consistency-and-clarity.md](../upcoming/step-50-consistency-and-clarity.md)
- [wrapped-native-token-unification.md](../raw-todos/wrapped-native-token-unification.md)
- [feature-branch-pins.md](../../reference/feature-branch-pins.md)
