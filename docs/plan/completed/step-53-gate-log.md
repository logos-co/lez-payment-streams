# Step 53 gate log

Packet: [step-53-repository-structure.md](step-53-repository-structure.md).
D53.37 close gate (completeness `rg` + cheap checks + one public module local
run). Store, privacy, testnet, and Qt kit stay on
[step-52-gate-log.md](step-52-gate-log.md).
Living runner: `./verify/e2e.sh`.

## Identity

| Field | Value |
| --- | --- |
| Local harness guest (hex bytes) | `cdc9bfea4fdb6490a99929619bfb2c0eefd36a936668b1b3a22f684e66b44f0c` |
| Guest ELF size | 373916 |
| Product ImageID | `c30781ea9d7cc7b3be36f459ce9094644b984224d3d3119a644bb1b21ba2982a` (unchanged; not rebuilt here) |
| Branch | `feat/step-53-repository-structure` |
| State file after close-gate run | `.scaffold/.lez_payment_streams-state` |

`make build` reproduced the local Docker guest at ImageID `cdc9bfea…`.
Product ELF pin stayed at `.scaffold/program-bins/lez_payment_streams-c30781ea.bin`.

## Ranges

| Range | Commit | Summary |
| --- | --- | --- |
| 1 | `acf91ac` | Move program and module trees to `program/` and `module/` |
| 2 | `222efb4` | Move operational scripts to `verify/` and drop Make aliases |
| 3 | `aad9698` | Colocate catalogues, rename living reference files, add link checker |
| 4 | `a49da8d` | Drop `make status`, retire unused testnet-submit C++ branch, clear product-door packet citations |

## Completeness (2026-08-16)

Living trees outside `docs/plan/`, `docs/archive/`, `docs/external/`, and
`verify/archive/` have no leftover path forms from the move map
(`scripts/`, root `methods/guest`, `examples/`, `tools/lez-testnet-submit`,
old crate dirs, old doc dirs, top-level `fixtures/`).
Product-door `Step [0-9]` / `step-[0-9]` stop-rule `rg` is clean.
`make help` has no `verify-step*`.
Inventory: `verify/README.md` gained `harness_policy.py` /
`test_harness_policy.py` (added on this branch after range 2).

## Close gate (2026-08-16)

Cheap first, then one chain smoke. No `SKIP_BUILD=1` on the module local
command. No Store, testnet, or `OWNER_PRIVACY`.

| Cell | Command | Result | Notes |
| --- | --- | --- | --- |
| guest | `make build` | pass | Docker `risc0-guest-builder:r0.1.88.0`; ImageID `cdc9bfea…`; `/tmp/step53-verify/make-build.log` |
| IDL | `make idl` | pass | `/tmp/step53-verify/make-idl.log`. Committed IDL kept; spel rewrote PDA seed consts from `native` to `\u0000` |
| program-id | `make program-id` | pass | ImageID `cdc9bfea…`; `/tmp/step53-verify/make-program-id.log` |
| clippy | `RISC0_SKIP_BUILD=1 RISC0_DEV_MODE=1 cargo clippy --workspace` | pass | `/tmp/step53-verify/clippy.log`. Packet line without `RISC0_SKIP_BUILD` fails methods embed (`riscv32im-risc0-zkvm-elf` std missing on the host toolchain) |
| tests | `RISC0_SKIP_BUILD=1 RISC0_DEV_MODE=1 cargo test --workspace` | pass | 152 core + 33 FFI; `/tmp/step53-verify/cargo-test.log` |
| check | `make check` | pass | terminology + relative links; `/tmp/step53-verify/make-check.log` |
| module local | `MODE=module ./verify/e2e.sh local run` | pass | Nix rebuilt `.lgx`; reused 45s localnet; vault_init / deposit / create / owner-close / claim; claim `72bc6f5c…`; `/tmp/step53-verify/module-local.log`; JSON-lines `.scaffold/e2e/artifacts/module-e2e-20260816T112346.log` |

`.scaffold/.lez_payment_streams-state` exists after that run
(`SIGNER_ID=zoPSFQw2HzfHjESW5YRaSegCoRMSCCSDGF88hLsQVCw`).
