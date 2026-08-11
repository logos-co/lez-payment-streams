# Step 45 — revisit dependencies and patches

Upcoming. Index: [index.md](../index.md).

Promotes and widens:

- [raw-todos/upstream-wallet-and-patch-inventory.md](../raw-todos/upstream-wallet-and-patch-inventory.md)
- [raw-todos/upgrade-interactive-wallet-to-lez-pin.md](../raw-todos/upgrade-interactive-wallet-to-lez-pin.md)
- Optionally evaluate [raw-todos/upstream-spel-generate-idl-canonical-paths.md](../raw-todos/upstream-spel-generate-idl-canonical-paths.md)
  and [raw-todos/rebase-eligibility-fork-onto-delivery-module-master.md](../raw-todos/rebase-eligibility-fork-onto-delivery-module-master.md)

Pin map today: [feature-branch-pins.md](../../reference/feature-branch-pins.md).

## Goal

Revisit the dependency and patch surface for the payment-streams stack as it stands.
Upgrade or drop what is safe; document what must stay patched and why, so wrap-up leaves an
intentional freeze rather than accidental drift.

## Context

Operational LEZ pin is `scaffold.toml` `[repos.lez]` (today `a58fbce` / tag `v0.2.0`).
Wallet still builds via `logos-execution-zone-module-patched`.
Paid Store stays on long-lived delivery forks `feat/payment-streams-store-eligibility`
(or successors). Step 47 hard-cut eligibility C ABI
`requester_peer_id` → `user_peer_id` on branches `feat/step-47-user-peer-id`
(logos-delivery + logos-delivery-module); fold that rename into the eligibility
fork pin when rebasing / re-locking (do not leave `requester_*` on the living
Store path).
Guest ImageID freeze baseline moves with [Step 44](step-44-payer-and-payee-close.md)
(D44.19): after Step 44 lands, freeze on the Step 44 ImageID / gate log, not the
retired Step 39 id (`072a26cc…`).

## Scope

In scope:

1. Inventory every in-repo patch / wrapper flake against current upstream at the chosen pins.
2. Decide per item: drop, keep, or upstream (issue/PR) — especially redundant JSON submit vs
   `sign_public_payload` (Store-only).
3. Align interactive `wallet` on PATH with the LEZ pin (scaffold cache prepend and/or
   `cargo install --path` from that pin — not tip).
4. Optionally bump LEZ / wallet / delivery / spel pins when a bump is justified; re-lock flakes
   and re-verify matrix cells that load the changed pieces.
5. Update [feature-branch-pins.md](../../reference/feature-branch-pins.md) with the post-step
   freeze (rev table + “must stay patched” list).

Out of scope:

- New protocol or guest features.
- Replacing patched wallet in Store runs before upstream exposes signing APIs (unless this
  step lands those APIs upstream first).
- Changing LIP-155 wire names.
- Mandatory delivery-module rebase onto `master` solely because free `storeQuery` (PR #76)
  landed — only when other `master` changes are needed.

## Suggested order

1. Document current pins and patch list (no bumps yet).
2. Drop or verify no-op patches on the module-only path.
3. PATH / interactive wallet pin hygiene.
4. Decide bump vs freeze for LEZ, delivery forks, spel; if bumping, re-verify Required
   (and Advanced if delivery/wallet move).
5. Write the freeze into pins + handoff notes (handoff README is a later docs-consistency step).

## Verification

- After any bump or patch drop: `MODE=module` local Required green.
- If delivery or Store signing path changes: Store local Required (and testnet Advanced if
  claimed).
- Pins doc matches what E2E actually uses.

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D45.1 | Packet ownership | New Step 45; wrap-up maintenance, not new product. |
| D45.2 | Default stance | Prefer freeze + document unless a bump removes patch debt or fixes breakage. |
| D45.3 | ImageID | Do not rebuild guest casually; release ELF is identity. Baseline is the Step 44 ImageID once Step 44 completes (was Step 39 `072a26cc…`). |

## Done when

- Patch inventory is explicit (keep / drop / upstream).
- Interactive wallet pin guidance is written and workable.
- `feature-branch-pins.md` reflects the freeze (or deliberate bumps).
- Promoted raw TODOs closed or pointed here.
