# Step 24 — plan excerpt (complete)

Normative packet for agents. Index: [index.md](../index.md).

Status: complete on `lez-payment-streams` `master` (LEE @ PR 510 pin `62d9ba10…`, transparent
`program_tests`, vendored SPEL on `lee_core`).

Engineering track in `lez-payment-streams` only. Does not change LIP-155, Store wire,
delivery forks, or Logos module ABIs. May run in parallel with Steps 18–20 once Step 17
regression is green.

### Step 24, LEZ `lee` harness alignment (NSSA v0.1.2 → 510)

Goal: align Rust host tests and guest LEZ type dependencies with the LEZ revision already
used for localnet, wallet, and E2E (`scaffold.toml`, Nix `lez-wallet-ffi-patched`, Step 17).
Re-enable in-process `program_tests` that call `V03State::transition_from_public_transaction`
without `IGNORE_LEZ_GUEST_ON_NSSA_HARNESS`.

Background:

- The payment-streams guest targets LEZ 491 semantics (LEE `/v0.2/` public PDAs, generic
  public transactions, deposit via `authenticated_transfer`).
- `lez-payment-streams-core` still pins git tag `v0.1.2` crates `nssa` / `nssa_core` for the
  in-process harness. That executor validates NSSA-era public PDAs, so ignored tests fail with
  `MismatchedPdaClaim` when run.
- Upstream `logos-execution-zone` at PR 510 merge (`62d9ba10…`) exposes the same simulator as
  crates `lee` / `lee_core` with 491-compatible `V03State`.
- SPEL upstream `v0.5.0` still depends on LEZ `v0.1.2`; this repo vendors
  `vendor/spel-framework-core` with an LEE PDA override. After `lee_core` @ 510, vendor SPEL
  must use the same rev (and may drop the inlined LEE `public_pda` shim in favor of
  `AccountId::for_public_pda`).

Canonical LEZ pin for this step (match existing integration):

```text
62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60
```

(PR 510 merge on `github.com/logos-blockchain/logos-execution-zone`.)

Do not pin `v0.2.0-rc*` LEZ tags for this work. Those RCs predate 491 and still use `nssa`
with NSSA public PDA prefixes. Do not pin floating `main` without recording an explicit
`rev` in every `Cargo.toml` and re-verifying demos.

Scheduling:

- Prerequisite: Step 17 definition of done on the current Nix LEZ pin (local LEZ E2E).
- Safe to start after Step 12–13 off-chain tests pass (N8 parity unchanged by this step).
- Step 19 LIP PR does not depend on Step 24; Step 24 helps LIP review by restoring
  `program_tests` as a guest binding gate.
- Optional: bump Nix / `scaffold.toml` to a newer LEZ `rev` only in the same PR as the Rust
  pin, with `make verify-step10a` and `make verify-step17` re-run.

#### Scope (in repo)

| Area | Action |
| --- | --- |
| `lez-payment-streams-core/Cargo.toml` | `lee` + `lee_core` + `clock_core` @ `62d9ba10…`; rename `host` feature dep |
| `lez-payment-streams-ffi/Cargo.toml` | Same LEZ rev; `lee` / `lee_core` |
| `methods/guest/Cargo.toml` | `nssa_core` → `lee_core` @ same rev (guest already uses `authenticated_transfer_core` @ 510) |
| `examples/Cargo.toml` | Remove duplicate `nssa` @ `v0.1.2`; single rev for all LEZ git deps |
| `vendor/spel-framework-core` | `lee_core` @ 510; update `use nssa_core` → `lee_core`; simplify `pda.rs` if `for_public_pda` matches host |
| Rust sources | `nssa` → `lee`, `nssa_core` → `lee_core`, `NssaError` → `LeeError` |
| `program_tests/*` | Remove `#[ignore = …IGNORE_LEZ_GUEST…]` incrementally; fix remaining harness gaps |
| Docs | Record unified pin in [feature-branch-pins.md](../../reference/feature-branch-pins.md) if Nix moves; cross-link [integration-decisions.md](../../reference/integration-decisions.md) SPEL-on-LEE |

Out of scope:

- Upstream SPEL release (`logos-co/spel` v0.5.0 still lists LEZ `v0.1.2`; we keep path patch until upstream bumps LEZ).
- logos-delivery / logos-delivery-module.
- Rewriting N8 or Store eligibility tests (should stay green).

#### Step-by-step plan

1. **Freeze the target rev**
   - Use `62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60` unless the team explicitly advances Nix and
     `scaffold.toml` together.
   - Add a workspace-level comment or shared constant in docs (this packet + pins table) so
     agents do not mix tag `v0.1.2` with `rev` 510.

2. **Workspace Cargo.toml pins**
   - Replace `nssa` / `nssa_core` / `clock_core` git deps with `lee`, `lee_core`, `clock_core`
     at the frozen `rev`.
   - Keep `spel-framework-*` at tag `v0.5.0` with `[patch]` → `vendor/spel-framework-core`.

3. **Vendor SPEL**
   - Bump `vendor/spel-framework-core/Cargo.toml` to `lee_core` (and optional `lee` for
     `host`) @ the same `rev`.
   - Replace `nssa_core` imports in vendor sources.
   - In `vendor/spel-framework-core/src/pda.rs`, prefer upstream-style
     `AccountId::for_public_pda` once linked to `lee_core` @ 510; delete the duplicated LEE
     prefix + `Impl::hash_bytes` block only after a unit test or PDA cross-check against
     `lez-payment-streams-core/src/pda.rs`.

4. **Guest crate**
   - Set `lee_core` @ 510 in `methods/guest/Cargo.toml`.
   - Rebuild guest: `cargo build -p lez_payment_streams-methods` (or project Makefile target).
   - Confirm `read_guest_program_bytecode()` path still finds the updated `.bin`.

5. **Mechanical Rust migration (core + ffi)**
   - Update imports and types across `test_helpers.rs`, `instruction_wire.rs`, `off_chain/`,
     `program_tests/`, `instruction_accounts.rs`, FFI crates.
   - Rename `NssaError` → `LeeError` and adjust `assert_execution_failed_with_code` if variant
     paths changed.

6. **Compile gate**
   - `cargo test -p lez-payment-streams-core --features host --no-run`
   - `cargo test -p lez-payment-streams-ffi` (if applicable)
   - Fix duplicate LEZ versions in `Cargo.lock` (only one `lee_core` semver/git entry for the
     workspace).

7. **Re-enable transparent `program_tests`**
   - Remove ignore on `program_tests/initialize.rs` first; run
     `cargo test -p lez-payment-streams-core --features host initialize -- --ignored` then
     without `--ignored` once fixed.
   - Continue with create_stream, pause/resume, top_up, withdraw, close_stream, claim.
   - Run `program_tests/deposit.rs` last (chained `authenticated_transfer`); fix account
     lists or genesis if the harness rejects deposit shape.

8. **Leave PP tests for a follow-up within Step 24**
   - `pp-program-tests` and privacy-tier paths touched LEZ key protocol heavily (#362, #474).
   - Either keep ignore on PP-only tests until transparent ladder is green, or enable with
     `RISC0_DEV_MODE=1` after explicit PP fix pass.

9. **Regression gates (must pass before closing Step 24)**
   - `cargo test -p lez-payment-streams-core --features host` (transparent; PP per team choice)
   - Off-chain / N8 tests unchanged: `store_eligibility_digest_matches_n8_reference_fixture`,
     `n8_canonical_wire_hex` output
   - `make verify-step10a`
   - `make verify-step12` and `make verify-step13`
   - `make verify-step17` (or documented equivalent) on unchanged module/Nix pins

10. **Documentation cleanup**
    - Update [architecture.md](../../on-chain/README.md) or harness comments that say
      `nssa::V03State` where `lee::V03State` is now accurate.
    - Note completion in [index.md](../index.md) step map.
    - Optional decision note N16 in [integration-decisions.md](../../reference/integration-decisions.md)
      when the pin lands (not required for DoD).

#### Pitfalls and mitigations

| Pitfall | Symptom | Mitigation |
| --- | --- | --- |
| Mixed LEZ revs in one workspace | Type errors: two `AccountId` types; link failures | One `rev` for every LEZ git dep in core, ffi, guest, vendor, examples; refresh `Cargo.lock` |
| Host on `lee` @ 510, guest still `nssa_core` @ v0.1.2 | Subtle guest/host drift; confusing PDAs in tests | Bump guest `lee_core` in the same PR; rebuild guest ELF |
| Vendor SPEL left on v0.1.2 | Guest NSSA PDAs, host LEE PDAs → `MismatchedPdaClaim` | Bump vendor `lee_core` with core; align `compute_pda` with `for_public_pda` |
| Dropping vendor patch before `lee_core` bump | Same PDA mismatch | Order: bump `lee_core` first, then simplify `pda.rs` |
| Using LEZ `v0.2.0-rc*` tags | Tests still NSSA-era; ignore reason unchanged | Use `62d9ba10…` or a post-491 `rev` only |
| Removing all `#[ignore]` at once | Large failure surface | Re-enable by submodule: initialize → streams → deposit |
| Deposit tests fail after PDA fix | Errors in chained call / `authenticated_transfer` | Read `program_tests/deposit.rs` and LEZ `Program::authenticated_transfer_program`; match 510 account metas |
| `LeeError` rename missed | Compile errors in `instruction_wire`, `common.rs` | Ripgrep `NssaError` and `nssa::` before merge |
| Heavier LEZ deps / slower CI | Longer `cargo test` | Expected; same graph as `seed_localnet_fixture` @ 510 |
| Bumping only Rust pin, not Nix | Demos on 510, tests on newer LEZ | Keep Rust pin equal to `scaffold.toml` / Nix unless intentionally coordinated |
| Assuming upstream SPEL v0.5.0 fixes PDAs | Unpatched SPEL + v0.1.2 → NSSA prefix | Keep `[patch]` vendor until SPEL upstream depends on `lee_core` @ 491+ |
| PP / PQ tests flaky | Failures in `pp-program-tests` | Defer PP; document which tests remain ignored and why |

#### Definition of done

- All LEZ git dependencies in `lez-payment-streams` use a single recorded `rev` (default
  `62d9ba10…`), with crate names `lee` / `lee_core` (no `nssa` @ `v0.1.2` in host paths).
- Transparent `program_tests` run without `IGNORE_LEZ_GUEST_ON_NSSA_HARNESS` (or the constant
  and comment are removed as obsolete).
- Guest ELF rebuilt from `lee_core` @ the same rev.
- Vendor SPEL uses `lee_core` @ that rev; LEE public PDAs match localnet (no
  `MismatchedPdaClaim` on `initialize_vault` in-process).
- Step 10a, 12, 13 verify scripts pass; Step 17 E2E passes on the matching Nix/scaffold pin.
- Remaining ignored tests, if any, are listed in this packet or a short comment in
  `program_tests/mod.rs` with a concrete owner (typically PP-only).

#### References

- Harness ignore: `lez-payment-streams-core/src/program_tests/common.rs`
  (`IGNORE_LEZ_GUEST_ON_NSSA_HARNESS`)
- LEZ repo: `github.com/logos-blockchain/logos-execution-zone` — `lee/state_machine`,
  `lee/state_machine/core`
- SPEL-on-LEE vendor rationale:
  [integration-decisions.md](../../reference/integration-decisions.md) (SPEL-on-LEE cleanup)
- Nix / scaffold pin:
  [feature-branch-pins.md](../../reference/feature-branch-pins.md),
  [scaffold.toml](../../../scaffold.toml) `[repos.lez].pin`
