# Privacy-enhanced Journey — payment streams with LEZ private execution

Status: draft — payer section reflects Step 36; payee section reflects Step 37.

This document describes the privacy-enhanced payment-streams flow where the payer
uses a `PseudonymousFunder` vault and/or the payee claims to a shielded address.
Owner privacy and provider privacy are independent choices; either may be used
alone or together. Automated module checks use `OWNER_PRIVACY=1` and
`PROVIDER_PRIVACY=1` (see [E2E.md](E2E.md)); `PRIVACY=1` is only an alias for
`OWNER_PRIVACY=1`.

It does not modify the existing `USER_JOURNEY.md` or `DEVELOPER_JOURNEY.md`.
Instead, it is the source of truth for the privacy-enhanced track.
Store × privacy E2E is Step 38.

## What this journey achieves

The payer funds and operates a payment stream without linking every vault and
stream operation to their primary public key. The payee receives accrued funds
to a shielded address rather than a public account. Amounts remain public
because `vault_holding` is a public PDA, but the identities are shielded.

## Prerequisites

- Same build and runtime setup as `USER_JOURNEY.md`.
- `payment_streams_module` and `logos_execution_zone` loaded in `logoscore`.
- `PAYMENT_STREAMS_GUEST_BIN` points to the current guest binary.
- The wallet module wrapper includes the `sign_private_payload` patch (Step 36).

## Key differences from the public User Journey

| Aspect | Public User Journey | Privacy-enhanced Journey |
| --- | --- | --- |
| Vault owner | Public account id | NPK-derived private account id (`PseudonymousFunder`) |
| Funding source | Public account | Vault owner private account (same account id) |
| Submit path | `submitGenericPublic` | `submitGenericPrivate` for all vault-touching operations |
| Owner signature | `sign_public_payload` (public key) | `sign_private_payload` (private account NSK) |
| Provider claim | Public account | Shielded private account (Step 37) |

## Payer privacy-enhanced flow (Step 36)

### Step 1 — Create a private account key pair

The payer needs an NPK/VPK pair for the vault owner. The private account id is
derived from the NPK plus an identifier.

Example wallet CLI command (verify exact syntax before use):

```bash
logoscore call logos_execution_zone create_account_private
```

Record the returned account id. To retrieve the NPK and VPK for sharing, use:

```bash
logoscore call logos_execution_zone get_private_account_keys <private_account_id>
```

The NSK stays in the wallet.

### Step 2 — Pre-shield funds

Move funds from the primary public account into the vault owner private account
(the same account id that will own the vault in Step 3). This is a generic
wallet public-to-private transfer, not a `payment_streams_module` operation.

Example wallet CLI command (verify exact syntax against the current CLI):

```bash
# amount_le16_hex is 32 lowercase hex chars (u128 little-endian).
# Prefix with s: so logoscore does not coerce all-digit hex to a float;
# the patched wallet strips the prefix before parsing.
logoscore call logos_execution_zone transfer_shielded_owned \
  <public_account_id_hex> <vault_owner_private_account_id_hex> s:<amount_le16_hex>
```

The PP `deposit` debits the vault owner account directly, so the funds must be
in that account. The module checks that the `signer` passed to `deposit` matches
`VaultConfig.owner` and rejects the deposit if it does not.

Local owner-privacy E2E (`OWNER_PRIVACY=1` / `make verify-module-local-privacy`)
sets `RISC0_DEV_MODE=1` so private submits use stub receipts. Without it,
proving can exceed the module IPC timeout even on localnet.

### Step 3 — Initialize a PseudonymousFunder vault

Create the vault with `privacy_tier = PseudonymousFunder` and the vault owner
set to the NPK-derived account id from Step 1.

Example `chainAction` JSON:

```bash
logoscore call payment_streams_module chainAction initializeVault \
  '{"signer":"<private_account_id>","vault_id":<id>,"privacy_tier":1}'
```

The `signer` field is the vault owner private account id. The `privacy_tier`
field is `1` for `PseudonymousFunder` and `0` for `Public`. The field shape is
resolved as D36.3: overload the existing `signer` field with the private account
id.

### Step 4 — PP deposit

Deposit from the pre-shielded private account into the vault.

Example `chainAction` JSON:

```bash
logoscore call payment_streams_module chainAction deposit \
  '{"signer":"<vault_owner_private_account_id>","vault_id":<id>,"amount_lo":<lo>}'
```

The `signer` here is the vault owner private account; the guest debits it
for the deposit. The deposit amount is public because `vault_holding` is a public
PDA.

### Step 5 — Create a stream

Create the stream as usual, but the vault owner is private.

```bash
logoscore call payment_streams_module chainAction createStream \
  '{"signer":"<private_account_id>","vault_id":<id>,"stream_id":<id>,"provider":"<provider_account_id>","rate":<rate>,"allocation_lo":<lo>}'
```

### Step 6 — Lifecycle operations

Pause, resume, and top-up route through `submitGenericPrivate` automatically
when the vault has `PseudonymousFunder` tier. The `signer` is the vault owner
private account id.

### Step 7 — Eligibility proof signing

When the payer prepares an eligibility proof for a paid Store query, the
`VaultProof.owner_signature` must be signed by the vault owner NSK. The module
calls the repo-local `sign_private_payload` patch on the wallet wrapper.

### Step 8 — Close and claim

Close the stream with the vault owner private account id.
For `claim`, the provider can be a public or private account. The transaction is
shielded because the vault owner is private, not because the provider is. If the
provider is public, the wallet signs the transaction with the provider's
public-account key; if the provider is private, the wallet proves with the NSK.
Step 37 generalizes the payee-side receiver-privacy flow for a private provider.

## Payee privacy-enhanced flow (Step 37)

Provider receiving privacy is independent of vault owner privacy. A stream can
use a public vault and a private provider (`PROVIDER_PRIVACY=1`), or combine
both flags.

### 1. Provider publishes a private receiving identity

The provider creates a private account and exports keys (same wallet surface as
the payer side):

```bash
logoscore call logos_execution_zone create_account_private
logoscore call logos_execution_zone get_private_account_keys <provider_private_account_id_hex>
```

Share the NPK-derived account id (base58 or hex) with the stream creator out of
band, the same way a public payee shares a public account id.

Reuse one `(npk, identifier)` across create and claim so credits land in one
private account chain. Do not invent a claim-time identifier field.

### 2. User creates the stream with that `provider_id`

```bash
logoscore call payment_streams_module chainAction createStream \
  '{"signer":"<owner>","vault_id":<id>,"stream_id":<id>,"provider":"<private_provider>","rate":<rate>,"allocation_lo":<lo>}'
```

On a `Public` vault, create stays on public submit: `provider` is instruction
data, not an account slot. For Store routing, `registerProviderMapping` takes
the same private account id (N5). Encoding smoke is covered in module unit
tests; dual-host Store E2E is Step 38.

### 3. Provider claims to the private account

After close (or when accrued is claimable):

```bash
logoscore call payment_streams_module chainAction claim \
  '{"owner":"<owner>","provider":"<private_provider>","vault_id":<id>,"stream_id":<id>}'
```

The module marks the provider signer slot `private` and calls only
`submitGenericPrivate`, including on a `Public` vault. Public PDAs stay
`public_no_sign`. The `vault_holding` balance drop is visible; the destination
is shielded.

Automated local gate:

```bash
MODE=module CHAIN=local PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run
# or: make verify-module-local-provider-privacy
```

E2E dust-shields a small amount into the private provider before claim so the
wallet has a committed private note (create-only without a note fails private
submit).

### 4. Timing note

Automatic-claim-on-closure (if used) forces the shielded payout to coincide
with close. Prefer delayed or batched claims when timing correlation matters.

## Known limitations

- Deposit and claim amounts are public because `vault_holding` is a public PDA.
- The user who creates a stream knows the `provider_id` and can link streams
  to the same provider.
- Amount and timing correlation across the shielding boundary are side channels,
  not breaks of the nullifier scheme.
- Cross-relationship vault rotation and identifier consolidation are hygiene
  recommendations, not enforced.

## Commands summary

Example commands used in this journey. Verify exact syntax against
the current wallet CLI and `payment_streams_module` before the document is
marked final.

| Operation | Example command |
| --- | --- |
| Create private account | `logoscore call logos_execution_zone create_account_private` |
| Get private account keys (NPK/VPK) | `logoscore call logos_execution_zone get_private_account_keys <private_account_id>` |
| Pre-shield funds | `logoscore call logos_execution_zone transfer_shielded_owned <public_account_id> <vault_owner_private_account_id> "s:<amount_le16_hex>"` (quote the amount so logoscore keeps it a string) |
| Initialize vault | `logoscore call payment_streams_module chainAction initializeVault '{"signer":"<private>","vault_id":<id>,"privacy_tier":1}'` |
| PP deposit | `logoscore call payment_streams_module chainAction deposit '{"signer":"<vault_owner_private>","vault_id":<id>,"amount_lo":<lo>}'` |
| Create stream | `logoscore call payment_streams_module chainAction createStream '{"signer":"<private>","vault_id":<id>,"stream_id":<id>,"provider":"<provider>","rate":<rate>,"allocation_lo":<lo>}'` |
| Close stream | `logoscore call payment_streams_module chainAction closeStream '{"signer":"<private>","vault_id":<id>,"stream_id":<id>,"authority":"<provider>"}'` |
| Claim | `logoscore call payment_streams_module chainAction claim '{"owner":"<private>","provider":"<provider>","vault_id":<id>,"stream_id":<id>}'` |
