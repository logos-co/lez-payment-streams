# Step 48 — program-graph LEZ unify and AT config drop

Upcoming. Index: [index.md](../index.md).
Depends on: [Step 45](step-45-dependencies-and-patches.md) freeze complete
(operator vs program-graph LEZ split; documented AT hex config).

## Goal

When a **published** spel release depends on a LEZ rev whose authenticated-transfer
ImageID matches live testnet, bump the **program graph** to that pairing and drop
`PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX` as required testnet config.

After this step, module FFI `authenticated_transfer().id()` matches live AT, so
Phase 3-class testnet greens with the env **unset**.

## Trigger

Do not start until:

1. Upstream spel tag/SHA pins LEZ whose AT ImageID equals live testnet
   (today live AT is `fe96c422…` from LEZ `v0.2.2+`; Step 45 operator artifacts
   are already on `v0.2.4`).
2. Prefer tag-only spel (no vendor, no LEZ `[patch]`) unless a new decision says
   otherwise.

If Step 45 took the C-fails exit (stayed on vendored spel `v0.5.0`), this step
also absorbs that deferred spel bump together with the graph LEZ bump.

## Pin targets

| Layer | After Step 48 |
| --- | --- |
| Program graph (`core`, `ffi`, `guest`, `examples`, `nix/payment-streams-ffi.nix`) | Same published spel + LEZ pairing as the trigger (AT id = live) |
| Operator stack (`scaffold.toml [repos.lez]`, Nix wallet, PATH wallet, submit helper if still present) | Unchanged unless already on that same LEZ tag |

Restore `scaffold.toml [repos.lez].pin` ≡ `nix/payment-streams-ffi.nix` **only if**
operator and graph share that LEZ tag. If the operator pin has moved ahead,
document the remaining delta — do not promise equality.

## Decisions

| ID | Decision |
| --- | --- |
| D48.1 | Follow-up to Step 45 split; not new protocol product. |
| D48.2 | Start only on the trigger above. |
| D48.3 | Bump program-graph LEZ + spel together; rebuild guest; ImageID; redeploy; fixture sweep; drop Required AT hex. Scaffold≡ffi.nix equality only when pins match (see Pin targets). |
| D48.4 | Prove module + store testnet with `PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX` **unset**. Remove or demote split-era auto-fill script paths. Env may remain as rare override, not Required. |
| D48.5 | Module runtime `getProgramIds`: out of scope unless the trigger never arrives and a separate decision reopens it. |
| D48.6 | Verification: ImageID-cut DoD (module local + dual-close + negatives; soft privacy + real-prove; Store hermetic; module + store testnet, AT env unset). Gate log: `docs/plan/completed/step-48-gate-log.md`. |

## Out of scope

- Re-doing Step 45 operator-stack work.
- Script-only AT auto-fill as a substitute for unify.
- Module runtime `getProgramIds` unless unify is abandoned.
- New LIP-155 features.
- Required removal of `lez-testnet-submit` (optional leftover from Step 45 Tier C).

## Implementor order

1. Confirm trigger (spel release + AT id == live); record hexes.
2. Pre-cut: drain valued streams on old ImageID if any.
3. Bump spel + program-graph LEZ (and `payment-streams-ffi.nix`); build guest;
   record ImageID + ELF; IDL two-run identical; `make full-reset-localnet`;
   deploy; sweep fixtures (same file set as Step 45 manifest).
4. Local module + hermetic Store verification; then module + store testnet with
   AT env **unset**.
5. Rewrite pins (single LEZ story or documented remaining operator delta);
   retire Step 45 “AT hex required under split” language; move packet to
   `completed/`; update index / AGENTS.

## Done when

- Program-graph spel + LEZ pins match a published pairing whose AT id equals
  live testnet (recorded in the gate log).
- New ImageID deployed and fixtures swept (or explicitly unchanged — unlikely).
- Required module + store testnet greens with AT env unset.
- Pins doc no longer requires AT hex as split-era config.
- Packet in `docs/plan/completed/`; index / AGENTS updated.
