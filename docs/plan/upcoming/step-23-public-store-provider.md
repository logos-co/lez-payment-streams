# Step 23 — plan excerpt

Active-work packet for agents. Index: [index.md](../index.md).

Optional track. Execute only if the program ships a **hosted** paid-Store provider on the public
Logos network (infra-operated node). Step 18 already proves eligibility against **public LEZ**
with **local** user and provider `logoscore` hosts.

### Step 23, Public Store provider (payment-stream eligibility)

Prerequisite: Step 18 definition of done (public sequencer fixture + local dual-host E2E on
testnet chain state). Step 17 remains the local-LEZ regression gate.

Architectural context:

Steps 17–18 register eligibility on a **provider `logoscore`** that runs Store + relay and
calls `setEligibilityVerifier(payment_streams_module)`. The user dials that provider’s libp2p
address (Step 17 `E2E_PROVIDER_AD`, typically `127.0.0.1`).

Step 23 moves the **provider role** to a long-lived deployment reachable on the public network
(public IP or DNS, firewall/NAT documented). The **user** may still run locally (or anywhere)
with forked modules; Store queries target the **published** provider multiaddr. Chain access
remains the public LEZ from Step 18 (same manifest program and payee semantics).

This step is a **deployment and operations** deliverable for the node team plus a thin
verification script or runbook. It does not change LIP-155, Store tag `30`, or hook ABIs.

#### Provider host must run (infra)

| Component | Source | Notes |
| --- | --- | --- |
| `logoscore` daemon | Org standard install | Persistent `--config-dir`, `--persistence-path`, `-m` module dir |
| `logos_execution_zone` | Patched wrapper pin ([feature-branch-pins.md](../../reference/feature-branch-pins.md)) | Provider wallet keys; `sequencer_addr` = Step 18 testnet |
| `payment_streams_module` | `logos-payment-streams-module#lgx` | Inbound verify via wallet reads on testnet |
| `delivery_module` | Integration branch `#lgx` | Store service + relay; bundled `liblogosdelivery` ≥ pinned rev ([N13](../../../reference/decisions-historical.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18)) |

On startup (or documented ops playbook):

- `load-module` order as Step 17; `open` wallet with provider `wallet_config.json` /
  `storage.json`.
- `createNode` + `start` with Store enabled, SQLite archive path on durable disk, retention
  policy (same fields as [archive/steps/local-store-dual-host-runbook.md](../../archive/steps/local-store-dual-host-runbook.md#delivery-createnode-defaults)).
- `setEligibilityVerifier` → `payment_streams_module` (paid mode for every inbound Store).
- Sync wallet to testnet height before serving.

#### Configuration synchronized with Step 18

| Item | Provider (hosted) | User (operator / demo) |
| --- | --- | --- |
| `sequencer_addr` / manifest | Step 18 testnet | Same testnet |
| `program_id_hex`, stream/vault ids | Step 18 `fixtures/testnet.json` | Same `FIXTURE_MANIFEST` |
| `provider_account_id` | Manifest payee; provider keys in wallet | Same id in `registerProviderMapping` |
| libp2p dial target | **Published** `provider_peer_id` + multiaddr (service advertisement) | User `staticnode` / `storeQuery` second arg from that ad |
| `payment_streams_state.json` | Provider persist dir (`provider_acceptances`, etc.) | User persist dir (session keys, mappings) — not shared |
| Delivery `portsShift` / listen addrs | Infra firewall rules | User may use local `portsShift` unrelated to provider |

Publish a stable **service advertisement** document (JSON shape aligned with Step 17
`E2E_PROVIDER_AD`): `provider_peer_id`, dial multiaddr(s), `content_topic`, `service_id`
(`/vac/waku/store-query/3.0.0`). Hostname/IP must match what remote users can reach.

Optional: `preset: logos.dev` on the hosted node for peer discovery; Store clients still need
the explicit provider multiaddr for paid queries in the MVP.

#### Deliver

- Infra handoff doc (can live under `docs/`): artifact pins, `createNode` JSON, module load
  order, eligibility registration, disk paths, restart policy, advertisement update process.
- Published advertisement URL or committed `fixtures/public-store-provider-ad.example.json`
  (peer id + multiaddr template, no secrets).
- User-side runbook snippet: install user modules locally, `FIXTURE_MANIFEST` = testnet,
  `registerProviderMapping` from ad file, `setEligibilityProvider`, paid `storeQuery` to public
  multiaddr.
- Verification: script or checklist proving remote user host succeeds on happy path and
  missing-proof failure against the **hosted** provider (same assertions as Step 17/18).

#### Definition of done

- Hosted provider runs integration-branch `delivery_module` with verifier registered; Store
  archives and serves on the public dial multiaddr.
- Independent operator (second machine) completes paid Store query + observes inbound
  eligibility failure without proof, using Step 18 testnet chain state and published ad.
- Advertisement and manifest `provider_account_id` stay consistent across doc, chain, and proofs.

Not in scope: merging delivery forks to upstream `master`; changing Step 18 (local provider on
testnet chain); User Journey UI (optional Step 21).
