---
marp: true
theme: default
paginate: true
size: 16:9
---

<!--
  PDF: npx @marp-team/marp-cli docs/journeys/SLIDES.md --pdf
  HTML: npx @marp-team/marp-cli docs/journeys/SLIDES.md -o docs/journeys/SLIDES.html
  Timing: 10–12 min (15 max); demo ~3–4 min.
  Slides are guidance, not speech notes.
-->

<!-- _class: lead -->

# Incentivizing Logos services with LEZ-based payment streams

Sergei Tikhomirov (AnonComms)

IFT Townhall

2026-07-07

---

# Overview

This talk covers:

- Why Logos needs paid services
- Payment streams (LIP-155)
- Integration with incentivization framework (LIP-175)
- Live demo: paid Store in Logos Delivery

---

# Why incentivized services

- Logos services should be economically self-sufficient
- Relay is peer-to-peer: peers serve each other, no payments
- Other Delivery protocols are request-response: user and provider roles,
  and the provider can be paid

---

# Incentivization for Logos services (LIP-175)

- Pattern for request–response protocols in Logos Delivery
  (Store, Lightpush, Filter)
- User attaches `EligibilityProof` to the request
- Provider returns `EligibilityStatus` in the response
- Proof forms: payment, membership, service credential

---

# Payment mechanism requirements

Goal: implement payments as form of eligibility proof.

We need:

- Scalability — no on-chain tx per request
- Security — bounded exposure if provider drops off
- Privacy — funder unlinkability
- Extendability — simple basic protocol

---

# Payment streams

- Time-based funds acctual (not per-message)
- accrued = rate × elapsed, capped at allocation

Example (allocation 500, rate 10/s):

```
t = 20s
████████████████░░░░░░░░░░░░░░░░░░
   accrued 200 │  unaccrued 300

t = 40s
████████████████████████████████░░
   accrued 400 │  unaccrued 100
```

---

# Vaults and streams

```
vault (deposit 1000)
├─ unallocated                    300
└─ allocated                      700
    ├─ stream A ──► provider P1
    │     rate 10/s, accrued 120 of 500
    └─ stream B ──► provider P2
          rate 5/s, accrued 80 of 200
```

See LIP-155 for full lifecycle.

---

# Payment streams based eligibility

```
user                         provider                          chain
 │                               │                               │
 │ ── first request + proposal ─►│                               |
 │ ◄── response (first unit) ─── │                               |
 │                               │                               │
 │ ── open stream ──────────────────────────────────────────────►│
                                 │ ◄── stream state ─────────────│
 │                               │                               │
 │ ── request + proof ──────────►│ ── read + fold ──────────────►│
 │ ◄── response ──────────────── │
 │                               │                               │
 │ ── close ────────────────────────────────────────────────────►│
                                 │ ── claim ────────────────────►│
```

---

# Streams + Store within Logos

```
user                                         provider     
┌──────────────────────────┐     libp2p      ┌──────────────────────────┐
│ delivery_module          │ ◄─────────────► │ delivery_module          │
│   │                      │                 │   │                      │
│   │                      │                 │   │                      │
│ payment_streams_module   │                 │ payment_streams_module   │
│   │                      │                 │   │                      │
│   │                      │                 │   │                      │
│ wallet_module            │                 │ wallet_module            │
└───┼──────────────────────┘                 └───┼──────────────────────┘
    │                                            │       
    ▼                                            ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        Payment streams LEZ program                     │
└────────────────────────────────────────────────────────────────────────┘
```

---

# Privacy

- LEZ: same program, public or private execution
- Today vaults and streams are public; the payment graph is visible
- Vault owner is a fresh key, not the funder's identity
- Funder links only via the funding path; pre-shielding is a manual extra step
- Future work: private funding and claims, no extra step

---

# Demo scenario

- Dual-host end-to-end run
- Composed Core modules on each host
- Paid query served
- Missing proof declined
- Teardown and claim when accrued

<!--
  Prefer a pre-recorded clip or a short live run.
  Localnet: make verify-store-local.
  TestNet v0.2: make verify-store-testnet.
-->

---

# Takeaways and outlook

- Payment streams is a scalable payment protocol in the Logos stack
- Streams incentivize Logos services, or work as standalone payment rails
- Demo: a working Store integration inside the Logos Core module framework
- Next: support for LEZ private execution for better unlinkability

---

# Links

- Code — https://github.com/logos-co/lez-payment-streams
- LIP-175 — https://lip.logos.co/messaging/core/raw/incentivization.html
- LIP-155 — https://lip.logos.co/anoncomms/raw/payment-streams.html

---

# Extra: protocol extensions (LIP-155)

- Auto-pause — stream pauses after a set duration unless resumed
- Delivery receipts — claims require user-signed proof of service
- Auto-claim on close — accrued funds paid out automatically
- Activation fee — upfront accrue on activation to deter pause/resume abuse
- Load cap — cumulative resource limit per stream per time window
- Multi-round stream params negotiation — provider counter-proposes
