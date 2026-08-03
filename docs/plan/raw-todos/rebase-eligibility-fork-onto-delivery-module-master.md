# Raw TODO — rebase eligibility fork onto delivery-module master

Ideas not yet scheduled as a plan step. Index: [../index.md](../index.md).

Related: [N6](../../reference/integration-decisions.md#n6-delivery-module-store-query-exposure),
[D2](../../reference/integration-decisions.md#d2-delivery-module-hook-design),
[feature-branch-pins.md](../../reference/feature-branch-pins.md),
upstream [PR #76](https://github.com/logos-co/logos-delivery-module/pull/76)
(`storeQuery` on `master`, merge `2701c44`).

## Context

Paid Store stays on the long-lived eligibility branch
`feat/payment-streams-store-eligibility` in `logos-delivery-module`
(and the matching `logos-delivery` pin).
Eligibility is not expected to merge to upstream `master` soon.

Upstream `master` now exposes a free Store path via
`storeQuery(jsonQuery, peerAddr, timeoutMs)` (sync, kernel `waku_store_query`).
Our fork already has a differently shaped
`storeQuery(queryJson, providerAddr)` (async, `storeQueryCompleted`,
eligibility hooks and tag-`30` bridge).
PR #76 alone does not unlock paid Store and is not a reason to rebase.

## Potential task

When we next need upstream module changes, rebase
`feat/payment-streams-store-eligibility` onto current
`logos-co/logos-delivery-module` `master` and reconcile the two `storeQuery`
surfaces (same name, different arity, sync vs async, kernel vs
`logosdelivery_store_query` plus eligibility).

Also re-lock the module flake’s `logos-delivery` input if that pin must move
with the rebase, then re-verify Store E2E (`scripts/e2e.sh` / matrix cells that
load `delivery_module`).

## When to do it

Do not rebase solely because PR #76 landed.

Consider rebasing when we need something else from `master`, for example

- per-instance node data path fix (#72)
- logos-delivery flake bumps (#77 / #78)
- Reliable Channels API (#68)
- `version()` removal (#80) or other breaking module API drift
- release / CI / RPC e2e harness updates that operators or docs start depending on

Defer while Step 39 (and other Store gates) only need the current eligibility
fork tip recorded in `feature-branch-pins.md`.

## Out of scope here

- Switching paid E2E or Developer Journey to upstream PR #76 `storeQuery`
- Upstreaming eligibility (D1 / D2) onto org `master`
