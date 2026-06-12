# Step 10a — progress handoff and follow-up

Status: fixture tooling and LEZ 491 guest alignment are in tree. **Step 10a DoD** is green when
`./scripts/verify-step10a-dod.sh` exits 0 after a full seed on PR 491 localnet (see runbook).

## What is done

- Committed operator path: `scaffold.toml`, `spel.toml`, `scripts/seed-localnet-fixture.sh`,
  `scripts/verify-step10a-dod.sh`, `scripts/reinit-scaffold-wallet.sh`,
  `examples/src/bin/seed_localnet_fixture.rs`, `fixtures/localnet.json.example`.
- Runbook: [`step10a-local-chain-fixture.md`](step10a-local-chain-fixture.md).
- **Public PDA prefix (LEE vs NSSA):** vendored
  [`vendor/spel-framework-core`](../vendor/spel-framework-core) patched to derive PDAs via
  `lee_core::AccountId::for_public_pda`, wired through workspace and guest `[patch]` (see
  [`Cargo.toml`](../Cargo.toml), [`methods/guest/Cargo.toml`](../methods/guest/Cargo.toml)).
  Host-side PDAs in `lez-payment-streams-core` already match LEZ 491.
- With the PDA patch, **`initialize_vault` can validate and confirm** on 491 (observed in local
  runs after guest rebuild).
- NSSA in-process **program tests** (`cargo test -p lez-payment-streams-core`) that execute
  the guest on [`nssa::V03State`] are **`#[ignore]`** while the guest targets LEZ 491 (LEE PDAs
  and `authenticated_transfer` enum). Unit tests outside that harness still run.

## What blocked green DoD

| DoD item | Typical failure |
| --- | --- |
| 2 — program id in manifest | `fixtures/localnet.json` stale after `make build` (new ImageID); seed aborts before rewrite |
| 4 — vault/stream PDAs | On-chain seed stops at **deposit** or never writes manifest |

### Root cause A — deposit chained-call encoding (guest vs LEZ 491)

LEZ 491 `authenticated_transfer` deserializes an enum
`Instruction::Transfer { amount }` / `Instruction::Initialize` (see LEZ
`programs/authenticated_transfer/core/src/lib.rs`).

NSSA v0.1.2 (and SPEL v0.5.0 guest) still emit a **bare `u128`** in the deposit
`ChainedCall`. On 491 the first word is read as a variant index, which produced sequencer/guest
errors such as `invalid value: integer 500, expected variant index 0 <= i < 2` when depositing
500 units.

**Required code change:** in `methods/guest/src/bin/lez_payment_streams.rs`, serialize
`authenticated_transfer_core::Instruction::Transfer { amount }` (LEZ 491) for deposit
`ChainedCall` data. NSSA in-process program tests are `#[ignore]` until SPEL-on-LEE or a
LEE executor.

### Root cause B — sequencer / wallet poller (symptom, not always root)

After applying the enum fix, one run **confirmed `initialize_vault`** then failed on **deposit**
with:

```text
confirm transaction: Transaction not found in preconfigured amount of blocks
```

That usually means the tx was **never included** (validation/execution skip) or the poller gave
up before the next block. It is **not** proof the enum fix is wrong; check sequencer logs for the
tx hash first.

### Root cause C — operator state drift

- **Stale manifest:** PDAs in `fixtures/localnet.json` are tied to `program_id_hex`. After every
  guest rebuild, run `make program-id` and either complete a full seed or regenerate manifest
  with `write-manifest` using the **current** `.bin`.
- **Partial seed:** If vault `0` exists but stream `0` does not, re-run seed without skipping
  deposit (seed binary resumes deposit + `create_stream` when stream PDA is empty).
- **`SEED_FORCE=1`:** Forces a full lifecycle attempt including `initialize_vault`; it **fails** if
  vault `0` already exists. Prefer chain reset or partial resume, not force, after a partial run.

## Follow-up when the issue is the local sequencer

1. Confirm scaffold owns `:3040`: `lgs localnet status` (not `foreign`), `lgs wallet -- check-health`.
2. Reproduce one failing tx hash from seed stdout; search
   `.scaffold/logs/sequencer.log` for that hash, `skip`, `panic`, `ProgramExecution`,
   `MismatchedPdaClaim`, `deserialize`.
3. **Clean chain + manifest** (recommended before the next seed attempt):

   ```bash
   lgs localnet stop
   rm -rf .scaffold/state/
   rm -f fixtures/localnet.json .lez_payment_streams-state .lez_payment_streams-fixture-provider
   ```

4. Rebuild and deploy once per session:

   ```bash
   export LEE_WALLET_HOME_DIR="$PWD/.scaffold/wallet"
   make build idl deploy
   make program-id   # note ImageID hex
   ./scripts/seed-localnet-fixture.sh
   ./scripts/verify-step10a-dod.sh
   ```

5. If deposit still never lands in a block:
   - Compare builtin `authenticated_transfer` ImageID in wallet health vs LEZ pin in
     `scaffold.toml` (foreign or mixed LEZ builds).
   - Increase visibility: run `seed-onchain` manually with `RUST_LOG=debug` and watch
     sequencer/mempool during the deposit submit window.
   - Escalate to LEZ/scaffold: chained public execution + `authenticated_transfer` on the pinned
     rev (`a999563…`); attach sequencer log excerpt and guest ImageID.

6. If only the **wallet poller** times out but the tx appears in a later block, adjust wallet
   poll settings in the LEZ 491 `wallet` crate or retry `getTransaction` via RPC before changing
   guest logic.

## Version bump does not remove the patches

- `nssa_core` `v0.1.2` == `v0.2.0-rc3`; older/newer rcs still use `/NSSA/v0.2/` public PDAs and
  bare-`u128` transfer in the NSSA guest.
- Host pin `a999563` (PR 491) is **ahead of published tags** (LEE rename, `/LEE/v0.2/` public
  PDA, transfer enum).
- SPEL `v0.5.0` pins `nssa_core` v0.1.2; no published SPEL targets `lee_core` yet.

Long-term: upstream SPEL on LEE (integration plan upstream note) to drop `vendor/spel-framework-core`
and the deposit enum shim.

## Suggested order for the next owner

1. Re-apply deposit enum serialization in the guest (section above).
2. `#[ignore]` NSSA deposit/claim program tests with a one-line reason.
3. Full reset + seed + `verify-step10a-dod.sh` exit 0.
4. Do not commit `fixtures/localnet.json` (gitignored).
5. Proceed to Step 10b only after 10a DoD is green.
