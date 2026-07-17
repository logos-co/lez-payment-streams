# Raw TODO — wrapped native token for a single asset path

Ideas not yet scheduled as a plan step. Index: [../index.md](../index.md).

Captured while designing multi-token vaults and provider payment policies
for LIP-155. The working payment-streams design keeps a dual asset path:
native via `authenticated_transfer` and `Account.balance`, custom fungibles
via the LEZ Token program. This note records an alternative long-term
simplification.

## Idea

Introduce a wrapped-native fungible (WETH-style) so native value is represented
as a Token definition and holdings, the same shape as custom tokens.
Payment streams would then use only the Token custody path
(program PDA vault holding, chained `Token::Transfer`), including for what
is economically “native” denomination.

Users wrap native into the canonical wrapped token before deposit and unwrap
after claim when they want raw native again.

## Why it is attractive

- One guest and policy path: always Token holdings and `TokenStreamPolicy`.
- No all-zeroes native sentinel or forked deposit/claim logic in payment streams.
- Aligns with AMM and other Token-only apps that do not treat native as a pool asset today.

## Why it is deferred

- Needs a wrap/unwrap program that locks native (AT) and mints or credits
  wrapped Token one-to-one, and the reverse on unwrap.
- Extra user steps and transactions (costlier under private execution).
- Custody, mint authority, and canonical definition deployment are platform
  concerns, not payment-streams core logic.
- Dual-path with an explicit native `token_id` encoding is enough for the
  multi-token policy investigation and a first implementation.

## Prefer LEZ-owned canonical wrap

This path becomes much more feasible if LEZ developers design, deploy, and
maintain a canonical wrapping program and wrapped-native Token definition on
the networks we target.

In that world payment streams only consume the Token program and pin the
published wrap definition id. We should avoid owning a payment-streams-specific
wrap program unless LEZ clearly will not provide one and product priority
still demands a single asset path.

## Relation to current multi-token work

Keep the dual-path design (native AT vs Token custom, immutable vault
`token_id`, provider `accepted_tokens` policy) as the near-term plan.
Mention wrapped-native in the RFC only as future work or residual linkage
if useful. Revisit this raw TODO when a LEZ wrap program exists or when
unifying the guest asset path becomes a scheduled goal.

## Promotion

Promote to a plan step only after:

- a canonical wrap program and definition id are available on the target
  network, or
- an explicit decision that payment streams (or another Logos repo) will
  ship and operate wrap itself.

Close this note when wrapped-native is adopted, rejected with rationale,
or superseded by a LEZ platform doc.
