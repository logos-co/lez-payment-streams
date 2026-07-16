# Privacy-enhanced Journey — payment streams with LEZ private execution

Status: draft — payer section reflects Step 36; payee section is a placeholder for Step 37.

This document describes the privacy-enhanced payment-streams flow where the payer
uses a `PseudonymousFunder` vault and the payee claims to a shielded address.
It does not modify the existing `USER_JOURNEY.md` or `DEVELOPER_JOURNEY.md`.
Instead, it is the source of truth for the privacy-enhanced track and will be
extended as Steps 36 and 37 land.

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
| Funding source | Public account | Pre-shielded private account sharing the vault owner NPK |
| Submit path | `submitGenericPublic` | `submitGenericPrivate` for all vault-touching operations |
| Owner signature | `sign_public_payload` (public key) | `sign_private_payload` (private account NSK) |
| Provider claim | Public account | Shielded private account (Step 37) |

## Payer privacy-enhanced flow (Step 36)

### Step 1 — Create a private account key pair

The payer needs an NPK/VPK pair for the vault owner. The private account id is
derived from the NPK plus an identifier.

Example wallet CLI command (verify exact syntax before use):

```bash
wallet account create-private
```

or

```bash
wallet account create-private-key
```

Record the returned NPK and the private account id. The NSK stays in the wallet.

### Step 2 — Pre-shield funds

Move funds from the primary public account into a private account owned by the
vault owner NPK. This is a generic wallet public-to-private transfer, not a
`payment_streams_module` operation.

Example wallet CLI command:

```bash
wallet transfer shielded-owned \
  --from <public_account_id> \
  --to <private_account_id> \
  --amount <lo>
```

The funding private account must share the NPK with the vault owner account that
will be created in Step 3. The PP `deposit` path performs an NPK preflight check
and rejects the deposit if the funding account's NPK does not match the vault
owner NPK.

### Step 3 — Initialize a PseudonymousFunder vault

Create the vault with `privacy_tier = PseudonymousFunder` and the vault owner
set to the NPK-derived account id from Step 1.

Example `chainAction` JSON:

```bash
logoscore call payment_streams_module chainAction initializeVault \
  '{"signer":"<private_account_id>","vault_id":<id>}'
```

The `signer` field is the vault owner private account id. The field shape is
resolved as D36.3: overload the existing `signer` field with the private account
id.

### Step 4 — PP deposit

Deposit from the pre-shielded private account into the vault.

Example `chainAction` JSON:

```bash
logoscore call payment_streams_module chainAction deposit \
  '{"signer":"<funding_private_account_id>","vault_id":<id>,"amount_lo":<lo>}'
```

The `signer` here is the funding private account, which must share the vault
owner NPK. The deposit amount is public because `vault_holding` is a public PDA.

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

Close the stream with the vault owner private account id, then claim as usual.
For the privacy-enhanced payee path, see the Step 37 section below.

## Payee privacy-enhanced flow (Step 37)

This section is a placeholder. After Step 37 lands, it will describe:

- How the provider publishes an NPK/VPK pair (`wallet account show-keys`),
  mirroring the public-mode flow where the provider shares a public account id.
- How the user creates the stream with the NPK-derived `provider_id`, using the
  same `registerProviderMapping` logic as public mode but with a private identity.
- How the provider claims accrued funds to a private receiving account via
  `submitGenericPrivate`.
- How the provider reuses one `(npk, identifier)` for consolidation.

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
| Create private account | `wallet account create-private` |
| Pre-shield funds | `wallet transfer shielded-owned --from <public> --to <private> --amount <lo>` |
| Initialize vault | `logoscore call payment_streams_module chainAction initializeVault '{"signer":"<private>","vault_id":<id>}'` |
| PP deposit | `logoscore call payment_streams_module chainAction deposit '{"signer":"<private>","vault_id":<id>,"amount_lo":<lo>}'` |
| Create stream | `logoscore call payment_streams_module chainAction createStream '{...}'` |
| Close stream | `logoscore call payment_streams_module chainAction closeStream '{"signer":"<private>","vault_id":<id>,"stream_id":<id>,"authority":"<provider>"}'` |
| Claim | `logoscore call payment_streams_module chainAction claim '{"owner":"<private>","provider":"<provider>","vault_id":<id>,"stream_id":<id>}'` |
