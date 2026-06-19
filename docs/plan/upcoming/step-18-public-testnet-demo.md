# Step 18 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 18, Public sequencer E2E (local Store and relay)

Prerequisite: Step 17 definition of done satisfied on local LEZ
(`scripts/demo-e2e-local.sh`, [N12](../../reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)).

Architectural context:

Step 17 uses a disposable local LEZ sequencer (`127.0.0.1:3040`) and two `logoscore`
processes on one machine. Relay, GossipSub, and Store traffic stay on **localhost** (disjoint
`portsShift`, user `staticnode` dial to provider multiaddr from
[`E2E_PROVIDER_AD`](../../step17-e2e-local.md#provider-service-advertisement-off-band-mimic)).

Step 18 keeps that **same dual-host P2P layout**. Only **LEZ chain access** moves to the org
public testnet sequencer (testnet v0.2 target): deploy `lez_payment_streams` once on that
network, bootstrap vault/stream state, and point wallets and the module fixture at the public
RPC endpoint.

Do not apply local reset-first policy from
[`demo-localnet-recovery.md`](../../demo-localnet-recovery.md). See testnet persistence in
[`step12-user-eligibility.md`](../../step12-user-eligibility.md) (Persistence across runs).

Hosting a Store provider on the public Logos mesh (infra-operated node, dialable from the
internet) is **Step 23** (optional). Step 18 does not require it.

#### What stays local vs what moves to testnet

| Layer | Step 17 | Step 18 |
| --- | --- | --- |
| LEZ sequencer | `127.0.0.1:3040` (`lgs localnet`) | Public testnet URL (documented) |
| `logoscore` user + provider | Two local daemons | Same: two local daemons |
| Delivery P2P / Store | Local TCP ports, provider archives SQLite | Same topology; multiaddrs remain loopback in the default script |
| Eligibility hooks | Local provider verifier + user provider registration | Same module behavior; chain reads/writes hit testnet |
| Program on chain | Deploy per local seed | Deploy once on testnet; stable `program_id_hex` in fixture |

#### Configuration that must match (sync checklist)

All of the following must refer to the **same** public network and deployed program.
Mismatches produce deploy failures, `STREAM_NOT_ACTIVE`, or verify rejections with no Store
symptoms.

| Setting | Where | Purpose |
| --- | --- | --- |
| `sequencer_addr` | Each host `wallet_config.json` used at `logos_execution_zone open` | Wallet RPC: reads, writes, sync |
| `sequencer_url` | `fixtures/testnet.json` (`FIXTURE_MANIFEST`) | Operator docs, scripts, verify parity with wallet |
| LEZ revision | [`feature-branch-pins.md`](../../feature-branch-pins.md) (`scaffold.toml`, `nix/payment-streams-ffi.nix`, wallet flakes) or explicit testnet pin bump | Client must speak the same LEZ as the public sequencer |
| `program_id_hex` | `fixtures/testnet.json` | Module PDA derivation and `chainAction` program binding ([N10](../../reference/decisions-and-notes.md#n10-step-11b-module-writes-decisions)) |
| Owner / provider account ids, vault/stream ids, derived PDAs | `fixtures/testnet.json` | Prepare/verify and on-chain stream state |
| `clock_10_account_id` | `fixtures/testnet.json` | Testnet clock account (do not copy local-only id without validation) |
| Guest program for submits | `PAYMENT_STREAMS_GUEST_BIN` on daemon and/or wallet program registration (510 path) | Writes (`createStream`, `claim`) against **testnet** deployed ELF |
| Provider libp2p identity | `E2E_PROVIDER_AD` (written by script) | User `registerProviderMapping` + `storeQuery` target — still **local** provider peer in Step 18 |
| Provider LEZ payee | `provider_account_id` in manifest | Must match bytes in proofs and on-chain `StreamConfig.provider` ([N5](../../reference/decisions-and-notes.md#n5-provider-identity-mapping)) |
| Module install | `MODULES_USER`, `MODULES_PROVIDER` | Same forked `.lgx` set as Step 17 ([feature-branch-pins.md](../../feature-branch-pins.md)) |
| Off-chain eligibility state | `--persistence-path` per host (`PERSIST_USER`, `PERSIST_PROVIDER`) | Session keys, mappings, provider acceptances ([N4](../../reference/decisions-and-notes.md#n4-persistence-policy)); separate from chain persistence |

Wallet and manifest: [N10 fixture and config](../../reference/decisions-and-notes.md#n10-step-11b-module-writes-decisions).
Delivery `createNode` defaults: [step17-e2e-local.md](../../step17-e2e-local.md#delivery-createnode-defaults) (local ports unchanged).

#### Deliver

- Committed `fixtures/testnet.json.example` (no secrets; may include stable `program_id_hex`
  after org-wide deploy). Operators copy to gitignored `fixtures/testnet.json` or set
  `FIXTURE_MANIFEST`.
- Testnet bootstrap runbook (or section in a new `docs/step18-public-sequencer-e2e.md`):
  fund accounts, deploy program if not already deployed, `initialize_vault` / `deposit` /
  stream setup per Step 12 testnet table; document one-time vs repeatable steps.
- Testnet wallet template(s): `sequencer_addr` aligned with manifest `sequencer_url` (per-host
  copies under `.scaffold/e2e/` or documented paths).
- `scripts/demo-e2e-testnet.sh` or `CHAIN=testnet` in the Step 17 orchestrator: **no**
  `lgs localnet start` / `demo-localnet-fresh.sh` on the chain path; still starts two local
  `logoscore` hosts; CI default remains Step 17 local LEZ.

Optional code changes: env switches in `scripts/e2e/run_local_e2e.py` (fixture path, wallet
paths, skip seed), Makefile target `verify-step18`.

#### Definition of done

- Documented public sequencer URL and LEZ revision aligned with wallet pins
  ([`feature-branch-pins.md`](../../feature-branch-pins.md)) or an explicit documented pin bump
  for testnet v0.2.
- `lez_payment_streams` deployed on that network; demo uses committed or operator manifest with
  matching `program_id_hex` and bootstrapped vault/stream state.
- Dual-host paid Store success and inbound eligibility failure match Step 17 outcomes (same wire
  and modules; chain fixture on testnet).
- Operator can reproduce without wiping **chain** state; persistence rules documented (manifest +
  vault reuse; when to reset local `PERSIST_*` only).

Not in scope: public internet Store provider (Step 23); replacing Step 17 local CI gate;
automatic testnet faucet unless the network ships a supported funding API.

Follow-on: Step 19 (LIP on-chain), Step 20 (developer journey on testnet v0.2 uses this step);
optional Step 23 (hosted provider).
