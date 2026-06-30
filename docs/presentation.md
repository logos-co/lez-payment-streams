---
theme: default
title: Payment Streams
---

# Payment Streams

From LIP-155 to Working Integration

Two verification flows: module-only lifecycle and Store-integrated eligibility.

---

## What payment streams enable

Continuous micropayments with per-second granularity from an owner to a provider.

- Vaults — per-owner fund containers holding a set of streams
- Streams — directional channels with a rate and allocation
- Accrual — value flows to the provider every second the stream is active
- Pause / resume / top-up / claim — owner and provider lifecycle operations

---

## Implementation architecture

```mermaid
graph TB
  subgraph Core["Logos Core modules"]
    PSM["payment_streams_module"]
    DM["delivery_module"]
    LEZ["logos_execution_zone"]
  end
  subgraph Backends["Module backends"]
    PSC["lez-payment-streams-core"]
    LD["logos-delivery (patched)"]
    LEZB["Logos Execution Zone"]
  end
  GUEST["SPEL guest program (LIP-155)"]
  PSM -->|chainAction / FFI| PSC
  PSC -->|program calls| GUEST
  DM -->|Store + hooks| LD
  LEZ -->|wallet FFI| LEZB
  LEZB -->|public tx| GUEST
  PSM -.->|provider mapping / verify| DM
```

---

## User Journey

Direct stream operation via the Logos Core module. Single host, no Store.

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

Artifact (`.scaffold/e2e/artifacts/module-e2e-*.log`):

```jsonl
{"phase":"vault_init","ok":true}
{"phase":"create_stream","ok":true}
{"phase":"topup_stream","ok":true}
{"phase":"claim","ok":true}
{"phase":"module_e2e_complete","ok":true}
```

Phases:

| Phase | localnet | testnet v0.2 |
| --- | --- | --- |
| vault / stream / lifecycle / claim | done | planned |

---

## Developer Journey

Store integration with LIP-155 eligibility proofs. Dual host.

RFC 73 wire format (Store tag `30`):

- Request carries opaque `EligibilityProof` bytes
- Response carries nested `eligibility_status` (code + desc)
- Failure: `BAD_REQUEST` (400), empty messages, verdict in tag `30`
- Codes: `OK`, `PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`

```bash
./scripts/e2e.sh local run
```

Phases:

| Phase | localnet | testnet v0.2 |
| --- | --- | --- |
| store_query_success | done | planned |
| store_query_missing_proof | done | planned |
| claim | done | planned |

---

## Developer Journey — sequence

```mermaid
sequenceDiagram
  autonumber
  participant U as User host
  participant PSM as payment_streams_module
  participant DM as delivery_module
  participant PM as Provider host
  participant Hook as verifyEligibility hook
  U->>PSM: initializeVault / deposit / createStream
  U->>PSM: registerProviderMapping(peerId, payee)
  U->>PSM: prepareEligibilityProofWithStreamProofForStoreQuery(n8, peer, id)
  PSM-->>U: proofBytesHex
  U->>DM: storeQuery(queryJson, providerAddr)
  DM->>PM: Store request + tag 30
  PM->>Hook: verifyEligibilityForStoreQuery(proofBytes, requesterPeerId)
  Hook-->>PM: OK | PROOF_INVALID | STREAM_NOT_ACTIVE
  alt OK
    PM-->>DM: response + messages
  else failure
    PM-->>DM: BAD_REQUEST 400, empty
  end
  DM-->>U: response
```

---

## Documentation deliverables

| | User Journey | Developer Journey |
| --- | --- | --- |
| File | `USER_JOURNEY.md` | `DEVELOPER_JOURNEY.md` |
| Audience | end users operating streams | integrators building paid services |
| Hosts | single | dual (user + provider) |
| Modules | `logos_execution_zone`, `payment_streams_module` | + `delivery_module` |
| Mode | `MODE=module` | `MODE=store` |
| localnet | done | done |
| testnet v0.2 | planned | planned |

---

## Current status

| Milestone | Status |
| --- | --- |
| LIP-155 spec (on-chain part) | done |
| SPEL guest program | done |
| `payment_streams_module` — chainAction, prepare, verify | done |
| Delivery fork — Store + eligibility hooks | done |
| User and Developer Journeys verified on localnet | done |
| Testnet v0.2 deployment (LEZ rc5 pin) | in progress |
| User and Developer Journeys on testnet v0.2 | planned post-deployment |

---

## Summary

One protocol, two patterns.

- On-chain program — LIP-155 rules in the SPEL guest
- Core module — `payment_streams_module` exposes rules as `chainAction` and proof methods
- Integration hooks — `delivery_module` + RFC 73 wire format bring eligibility to paid services

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run   # User Journey
./scripts/e2e.sh local run                             # Developer Journey
```
