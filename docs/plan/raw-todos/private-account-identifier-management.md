# Raw TODO — private account identifier management

Ideas not yet scheduled as a plan step. Index: [../index.md](../index.md).

Related: [Step 36](../completed/step-36-payer-funder-unlinkability.md) (D36.3 account-id
fields), [Step 37](../completed/step-37-payee-receiver-privacy.md) (D37.4 / D37.10 —
claim uses `provider` account id only; no `provider_private_identifier` on claim).

## Problem

LEZ private `AccountId` values are derived from `(NPK, identifier)`. Distinct
identifiers under the same NPK are distinct accounts. Spending many such
accounts later costs linear proving and transaction size, so operators are
encouraged to reuse one `(NPK, identifier)` (one account id) for a claim chain.

Payment-streams `chainAction` today passes full account ids (`signer`, `owner`,
`provider`), matching Step 36 D36.3. The wallet creates private accounts
(`create_account_private`) and resolves them by account id
(`resolve_private_account`). Guest `claim` requires
`provider.account_id == StreamConfig.provider`, so the payee account is fixed
at `createStream`; claim cannot credit a different identifier under the same
NPK without failing authorization.

An optional `provider_private_identifier` on `claim` was considered in early
Step 37 drafts. It was deferred because:

- the stream already binds one provider account id
- a claim-time identifier is either redundant with that id or suggests an
  illegal alternate destination
- “wallet chooses identifier if omitted” conflicts with explicit submit-path
  selection (D37.9)
- Step 36 deliberately avoided raw identifier fields on payment-streams ops

Step 37 MVP documents consolidation as hygiene: create one private provider
account and reuse that account id on create and claim.

## Gap this TODO tracks

There is still no first-class operator tooling in this repo for:

- choosing or recording the 16-byte identifier when creating a private account
- listing private accounts under one NPK and their identifiers
- guiding “reuse this account id for the next stream/claim” vs “rotate”
- (future) richer consolidation or shared-custody patterns (GMS), which remain
  out of scope for Steps 36–38

Wallet JSON key export in the current stack often has no `identifier` field;
send paths that need a foreign recipient identifier use wallet-internal
fallback behavior. That is enough for MVP flows, not for deliberate
identifier lifecycle management.

## Potential future directions (not committed)

These are idea sketches only; none are scheduled.

1. Wallet create-with-identifier  
   Extend `logos_execution_zone` / wallet FFI so `create_account_private` (or a
   sibling) accepts an explicit identifier and returns the derived account id.
   Payment-streams keeps account-id-only `chainAction` fields.

2. Wallet inventory / hygiene helpers  
   APIs or CLI to list private accounts for an NPK, show identifiers, and
   recommend reuse vs rotate. Docs and User Journey could point operators here
   instead of inventing module fields.

3. Setup-time only in payment-streams (still no claim identifier)  
   Optional helper that returns a stable provider account id for a relationship
   (create-or-reuse under one NPK), used before `createStream`. Claim continues
   to take only `provider`.

4. Claim-time identifier field (discouraged unless guest model changes)  
   Revisit only if the on-chain claim model gains a supported way to credit a
   different private account than `StreamConfig.provider`. As of LIP-155 guest
   checks today, this fights the protocol.

5. Cross-relationship rotation policy  
   Document or optionally enforce rotating provider (or vault owner) account
   ids across counterparties; related to Step 36 vault-rotation hygiene and
   Step 37 provider NPK linkability notes.

6. GMS / shared private accounts  
   Already called out as out of scope in Step 37. Track only if product
   requirements change.

## Consistency rule of thumb

Prefer identifier control at wallet account-creation (or inventory) time.
Keep `payment_streams_module` `chainAction` on full account ids. Do not add
raw identifier parameters to `claim` unless the guest authorization model
changes.

## Promotion

Fold into a small wallet/module UX step if identifier tooling becomes a
product need; otherwise keep as journey-doc hygiene under Step 37 / Step 38.
Link from Step 37 when the claim-field deferral is recorded (D37.10).
