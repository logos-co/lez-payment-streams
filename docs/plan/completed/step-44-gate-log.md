# Step 44 gate log

ImageID freeze moves here from Step 39 (`072a26cc…`).
Step 39 prove greens are ImageID-scoped and superseded by rows below (D44.18).

New ImageID: `ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1`
(release ELF 366716 bytes).

| Date | Commit | Cell | Artifact | Result | ImageID | RISC0_DEV_MODE | Close role | Clock |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-08-12 | 9d5db40 | module-local-soft-owner | .scaffold/e2e/artifacts/module-e2e-20260812T020638.log | pass | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | 1 | owner | (empty; extraction fixed post-run) |
| 2026-08-12 | 9d5db40 | module-local-soft-provider | .scaffold/e2e/artifacts/module-e2e-20260812T021236.log | pass | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | 1 | provider | 2f4c455a2f436c6f636b50726f6772616d4163636f756e742f30303030303031 |
| 2026-08-12 | 9d5db40 | module-local-soft-owner-privacy | .scaffold/e2e/artifacts/module-e2e-20260812T021729.log | pass | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | 1 | owner | 2f4c455a2f436c6f636b50726f6772616d4163636f756e742f30303030303031 |
| 2026-08-12 | 9d5db40 | module-local-soft-provider-pf | .scaffold/e2e/artifacts/module-e2e-20260812T022358.log | pass | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | 1 | provider | 2f4c455a2f436c6f636b50726f6772616d4163636f756e742f30303030303031 |
| 2026-08-12 | 9d5db40 | module-local-negative | .scaffold/e2e/artifacts/module-e2e-20260812T023010.log | pass | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | 1 | (negatives) | — |
| 2026-08-12 | 700b449 | store-local | .scaffold/e2e/artifacts/e2e-20260812T023920.log | pass | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | 1 | provider | — |
| 2026-08-12 | c9dd6bb | module-local-real-prove-pf-owner | .scaffold/e2e/artifacts/module-e2e-20260812T040819.log | pass | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | 0 | owner | 2f4c455a2f436c6f636b50726f6772616d4163636f756e742f30303030303530 |
| 2026-08-12 | c9dd6bb | deploy-testnet + fixture sync | (fixtures/scripts tip = ee2cfb74…) | pass | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | — | — | deploy-program exit 0; ELF 366716; operational 072a26cc grep clean |
| 2026-08-12 | f5b5948+ | module-testnet (soft) | .scaffold/e2e/artifacts/module-e2e-20260812T053832.log | fail | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | 1 | owner | vault_init included; deposit/create/close getTransaction null — live sequencer AT=v0.2.2+ (fe96c422…) while logos_execution_zone module still LEZ v0.2.0; wallet CLI split home + AT hex override green |
| 2026-08-12 | 3359bb2+ | module-testnet (soft owner-close) | .scaffold/e2e/artifacts/module-e2e-20260812T062806.log | pass | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | 1 | owner | LEZ module v0.2.4 + statistics open + PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX=fe96c422…; claim vault_drop ok |
| 2026-08-12 | 926aaff+ | store-testnet (soft payee-close) | .scaffold/e2e/artifacts/e2e-20260812T074309.log | pass | ee2cfb7466c239c51ef5c60222f4216d45df4de4664bae5c976c1b29f3da2ed1 | 1 | provider | dual-host Store eligibility + chainAction close/claim; SEED_ALLOCATION honored; tip ~3869 |
