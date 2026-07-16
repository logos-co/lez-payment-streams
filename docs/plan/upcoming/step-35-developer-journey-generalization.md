# Step 35 — plan excerpt

Active-work packet for agents. Index: [index.md](../index.md).

Active — rewrite `DEVELOPER_JOURNEY.md` from a Store end-to-end verification runbook
into a protocol-agnostic guide for adding LIP-155 payment-stream eligibility to a
request-response protocol, with Store as the worked example. The runnable Store recipes
already live in [E2E.md](../../journeys/E2E.md); this step repurposes the journey doc and
deduplicates.

## Problem

`DEVELOPER_JOURNEY.md` today duplicates the runnable Store verification
(`make verify-store-*`, JSONL phases, env-var table, failure modes) that
[E2E.md](../../journeys/E2E.md) now holds as SSOT. The doc reads as an operator validation
runbook, not a journey for an integrator adding eligibility to their own protocol. The
integration is already layered (RFC 73 abstract pattern; opaque hook bytes; reusable
on-chain program and module) so a generalized guide is feasible without a second
implemented protocol.

## Architectural context

| Track | Audience | Doc | Runnable |
| --- | --- | --- | --- |
| E2E verification | Maintainers, integrators re-running gates | E2E.md | Yes — `make verify-store-*` |
| Developer Journey | Integrators adding eligibility to a protocol | DEVELOPER_JOURNEY.md | No — cross-links E2E.md |
| User Journey | End users learning payment streams | USER_JOURNEY.md | Walkthrough only |

RFC 73 is the general pattern (proof on request, status on response, extensible proof
types). The `liblogosdelivery` eligibility hooks are opaque and payment-streams-agnostic
([D2](../../reference/integration-decisions.md#d2-delivery-module-hook-design)); future
eligibility mechanisms can register a different module without changing the hook contract.
The on-chain LIP-155 program, `payment_streams_module` chain writes, the `EligibilityProof`
protobuf, provider mapping, and verdict vocabulary are reusable verbatim. Only the
canonical request bytes, wire codec tags, and per-protocol method names are
protocol-specific.

## Scope decisions (resolved)

- No second protocol implemented; Store is the sole worked example.
- Stay protocol-agnostic in prose; use Delivery and Store only as the concrete instantiation.
- The eligibility pattern is described abstractly (proof on request, status on response,
  opaque bytes, verifier callback) without assuming Delivery.
- Canonical payload step stays high-level: state the encoding and decoding requirements
  (both sides byte-match, pinned test vector) without prescribing Nim or Rust.
- Domain prefix is the reader's choice.
- Module surface: add new `*For<P>Query` methods next to the Store ones, reusing internal
  verify and prepare helpers; note the codegen constraint (one LogosAPI name per method,
  single-line declarations). No refactor in this step.
- Keep existing Store-named methods as-is; note that the current naming format may
  generalize in a future version if needed.
- Policy and `service_id` are assumed documented in the payment-streams spec and module
  docs; this guide points to them.
- Success criterion is a self-check, not an automated test (no test for an unknown
  protocol).
- Cross-link to E2E.md for the runnable Store recipe, using the absolute GitHub URL
  (`https://github.com/logos-co/lez-payment-streams/blob/main/docs/journeys/E2E.md`).
  No runnable content in DEVELOPER_JOURNEY.md.
- No appetite to implement a second protocol in this step.

## Deliverables

- [ ] Rewrite [docs/journeys/DEVELOPER_JOURNEY.md](../../journeys/DEVELOPER_JOURNEY.md) as
  the protocol-agnostic eligibility integration guide per the structure below.
- [ ] Move E2E-useful content out of DEVELOPER_JOURNEY.md into
  [E2E.md](../../journeys/E2E.md): the environment-variable reference, the failure-modes
  table, and the configuration-details block.
- [ ] Update [docs/plan/index.md](../index.md) upcoming table and program outcomes with
  Step 35.
- [ ] Update [AGENTS.md](../../AGENTS.md) active-work pointer with Step 35.
- [ ] Fix the `E2E_CLAIM_OPTIONAL` link in
  [docs/store-integration/README.md](../../store-integration/README.md) to point at E2E.md.

### DEVELOPER_JOURNEY.md structure

Follow the highest-level heading skeleton of the published journey doc
(logos-docs issue #369) so the in-repo draft reads as a journey of the same shape.
The 14 top-level headings, adapted to the protocol-agnostic content:

1. What the user achieves — eligibility on the reader's own request-response protocol,
   Store as worked example.
2. Why it matters — self-sustaining networks; payment streams as a universal eligibility
   mechanism.
3. Key components — reusable pieces (on-chain program, module, wallet, `EligibilityProof`)
   and protocol-specific pieces (canonical bytes, wire codec, method names, hooks,
   policy).
4. Repository — `https://github.com/logos-co/lez-payment-streams`.
5. Runtime target — the guide is not runnable; the runnable Store reference is in E2E.md.
6. Prerequisites — Store reference setup (repo README) and, for the reader's own protocol,
   control over its codec and transport plus a deployed LIP-155 program.
7. Commands and expected outputs — the five protocol-specific steps as the developer's
   action sequence, each with its expected output; runnable Store commands live in E2E.md.
8. Expected result — the manual self-check (no automated test for an unknown protocol).
9. Configuration details — naming format, codegen constraints, policy and service identity.
10. Failure modes and limits — verdict failure modes and hook-registration failure.
11. GitHub handle.
12. Discord handle.
13. Existing docs or specs — LIP-155, RFC 73, and a minimal set of repo reference docs.
14. Additional context — sibling repositories, estimated time, security notes.

Link policy: keep links to a minimum. Any link to a file in this repo uses an absolute
GitHub URL (`https://github.com/logos-co/lez-payment-streams/blob/main/...`); the runnable
Store reference cross-links E2E.md this way. External spec links (LIP-155, RFC 73) stay as
absolute URLs.

## Definition of done

- [ ] DEVELOPER_JOURNEY.md rewritten; no runnable Store commands remain (cross-link only).
- [ ] E2E.md holds the env-var reference and failure-modes table.
- [ ] index.md and AGENTS.md list Step 35.
- [ ] store-integration/README.md `E2E_CLAIM_OPTIONAL` link points at E2E.md.
- [ ] Completed step packets (17-33) not edited.

## Not in scope

- Implementing eligibility for a second protocol.
- Refactoring `payment_streams_module` toward a protocol-agnostic method surface.
- logos-docs publication.
- Step 20, 22, 32, 34 packet edits (active steps untouched except index and AGENTS
  pointers).
- Changing Store method names or FFI symbols.
