Promoted to a plan step — see [raw-todos README](README.md).

# Raw TODO — upgrade interactive wallet to LEZ pin

Ideas not yet scheduled as a plan step. Index: [../index.md](../index.md).

Deferred during Step 39 Phase 4 close.
Continue Store privacy E2E with current harness wiring; do not block on this.

## Problem

Default `PATH` resolves `wallet` to `~/.cargo/bin/wallet` (Cargo `0.1.0`, Mar 2026).
That build honors `NSSA_WALLET_HOME_DIR` only and does not match the v0.2 storage /
env contract (see Step 27 note).

Payment-streams operational pin is LEZ `scaffold.toml` `[repos.lez]`
(today `a58fbce` / tag `v0.2.0`). Store E2E already prepends

`~/.cache/logos-scaffold/repos/lez/<pin>/target/release`

for `wallet auth-transfer send`, so the long privacy gate can proceed without
replacing the Cargo binary.

logos-docs prefers `cargo install --path lez/wallet --force` from the LEZ tag;
this repo prefers the scaffold-pin `target/release/wallet` on `PATH`.

## When returning

1. Confirm current preferred rev from `scaffold.toml` `[repos.lez].pin`
   (or the then-current TestNet LEZ tag if the pin moved).
2. Prefer PATH hygiene first: ensure interactive shells and ad-hoc scripts
   prepend `$(ps_lez_cache)/target/release` (or rebuild that pin wallet).
3. Optional: `cargo install --path lez/wallet --force` from that same pin/tag
   into `~/.cargo/bin` so default `command -v wallet` matches.
   Do not install from LEZ tip ahead of the pin.
4. Smoke before any long E2E: `wallet auth-transfer send` → `getTransaction`
   non-null on a throwaway wallet home; confirm `LEE_WALLET_HOME_DIR` (and
   dual `NSSA_*` if still needed).
5. Watch for storage-format breakage on existing `.scaffold` / seed homes;
   use a fresh home or restore if the old `0.1.0` storage will not load.

## Out of scope here

- Changing logoscore install (releases / Nix `logos-logoscore-cli`).
- Reopening Step 39 Phase 1–3 greens.
