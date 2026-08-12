# Naming conventions

Use this vocabulary consistently in product docs and runbooks.

## Role layers (LIP-155 / module)

Do not collapse service, chain, submitter, and funding into one word.
Formerly `payer` / `payee` (journey slang) map as below; do not use them on
living surfaces (see Step 47).

| Layer | Terms | Use for |
| --- | --- | --- |
| Service / protocol (LIP Roles, Store hosts) | `user`, `provider` | Who requests service vs who delivers. `provider` is the same party as the chain-layer provider (cross-layer term). |
| On-chain identity (guest, module JSON, manifests) | `owner`, `provider` | `VaultConfig.owner`, `StreamConfig.provider` / claim account. |
| Tx submitter (derived) | `submitter` | Who signs/submits this tx. Not a module JSON key. Precedence for close/claim: `provider` > `owner`. Owner writes (`createStream`, …) keep owner as submitter even when `provider` appears as instruction data. |
| Privacy funding (ops only) | `funder` / `PUBLIC_FUNDER` | Public account that shields into a private owner — not the vault owner. |
| Informal (retired) | `payer`, `payee` | Historical journey slang only. |

Module `chainAction` JSON (writes + close): vault owner id is `owner`; stream
provider id is `provider` (close). Store verify peer param is `userPeerId`
(symmetry with `providerPeerId`). LEZ `#[account(signer)]` / IDL `"signer":
true` means must-sign only — not the vault owner id.

Account-id write params accept base58 or 64-hex via one helper (fixed order):

1. Trimmed input exactly 64 hex characters (`[0-9a-fA-F]{64}`) → decode hex to 32 bytes.
2. Else → decode as base58; require decoded length 32.
3. Otherwise → error.

Inventory (not renamed): localnet fixture state still keys `SIGNER_ID`; scripts
alias it to local `OWNER` at the state-file boundary.

## External product names

| Term | Meaning |
| --- | --- |
| Module verification | Single-host `payment_streams_module` happy path (`MODE=module`). |
| Store integration | Dual-host Store demo with eligibility (`MODE=store`, default). |

## Verification flows (`MODE`)

| Term | Meaning |
| --- | --- |
| User Journey | `MODE=module` on `scripts/e2e.sh`. Payment streams in isolation (no Store, no eligibility gate). |
| Developer Journey | `MODE=store` (default). Dual-host demo with `delivery_module` and LIP-155 eligibility on Store. |

Makefile targets use the Journey names directly where applicable.

## N18 demo tracks (plan index)

| Term | Meaning |
| --- | --- |
| N18 | Developer Journey (in-repo Store track; Step 20 formal logos-docs publish wontfix — logos-docs#369) vs User Journey (Step 22, complete — logos-docs#370) vs User Journey UI (Step 21, upcoming) vs forum progress report (Step 46, upcoming; high-level both tracks). |

## Logos and protocol names

| Term | Use |
| --- | --- |
| payment streams module | Prose description of the Logos plugin. |
| `payment_streams_module` | Runtime module id. |
| Store | Waku/Logos Store protocol (capitalize). |
| `logos-delivery` | Repository for Store and liblogosdelivery. |
| `delivery_module` | Logos plugin for Delivery/Store. |
| `logos_execution_zone` | LEZ wallet Logos module id. |
| LIP-155 | Hyphenated spec name. |

## Makefile targets

Primary (step-free):

| Make target | Matrix cell |
| --- | --- |
| `make verify-module-local` | User Journey × localnet |
| `make verify-store-local` | Developer Journey × localnet |
| `make verify-store-testnet` | Developer Journey × testnet (advanced) |
| `make verify-store-local-lifecycle` | Maintainer only (two runs, one ledger) |

Legacy aliases: `verify-step17`, `verify-step18`, `verify-step17-back-to-back`.

Canonical commands: [verification-matrix.md](verification-matrix.md),
[scripts/README.md](../../scripts/README.md).

## Scaffold layout

Gitignored state under `$REPO_ROOT/.scaffold/`. Path helpers live in
`scripts/lib/common.sh` (`ps_e2e_*`, `ps_scaffold_*`).

| Path | Journey | Role |
| --- | --- | --- |
| `e2e/user/modules` | User (+ Store client install) | `lgpm` install tree (`MODULES_USER`) |
| `e2e/user/logoscore`, `e2e/user/persist` | User / Store client host | Dual-host logoscore daemon state |
| `e2e/user/wallet-local` | User (localnet module E2E) | Isolated wallet; reset each `module-e2e.sh` local run (`WALLET_E2E_DIR`) |
| `e2e/provider/modules`, `e2e/provider/logoscore`, `e2e/provider/persist` | Developer (Store provider) | Provider host |
| `e2e/testnet-wallet` | User + Store (testnet) | Testnet wallet home (`ps_chain_wallet_home` when `CHAIN=testnet`) |
| `e2e/artifacts` | E2E verification | JSONL logs (`module-e2e-*.log`, `e2e-*.log`, …) |
| `e2e/provider-advertisement.json` | Developer | Off-band provider ad file (orchestrator) |
| `wallet` | Localnet (scaffold) | Default localnet wallet when not using `e2e/user/wallet-local` |

Override defaults with env vars (`MODULES_USER`, `MODULES_PROVIDER`, `WALLET_E2E_DIR`,
`TESTNET_WALLET_DIR`, …). Manual User Journey on testnet uses the same paths as
`MODE=module` E2E: [USER_JOURNEY.md](../journeys/USER_JOURNEY.md).
