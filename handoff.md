# Hand-off — Payment Streams Integration

You are picking up work on integrating LIP-155 payment streams
into the Logos stack as a demo.

Start with `integration-index.md` in this directory.
It contains the full plan, onboarding reading order,
prerequisites, component overview, decisions, notes,
and numbered integration steps (Step 3 splits into 3a/3b; Steps 6–22 after Rust FFI) with definitions of done.
See [integration-index.md](integration-index.md#program-outcomes) for demo, spec, and journey steps 17–22.

On-chain normative text merges in Step 19 (`feat/payment-streams-onchain-part` → `main`).
`logos-architecture-overview.md` in this directory
covers the architectural context
(hosts vs. modules, FFI boundaries, Qt roles, LEZ chain side).

## Store access via `delivery_module`

Paid Store demo uses our delivery forks: `logosdelivery_store_query` (Step 15) and
`delivery_module.storeQuery` (Step 16) on branch `feat/payment-streams-store-eligibility`.
See [integration-index.md](integration-index.md#store-query-dependency) and [N6](docs/reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure).
Do not pin the retired `feat/liblogosdelivery-query-store` / early `queryStore` PR stack.

Wallet pins: [docs/feature-branch-pins.md](docs/feature-branch-pins.md).

Doc index: [`docs/README.md`](docs/README.md).

Operator and runtime (Steps 7, 9–11): [`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md).

## Naming

Reuse the vocabulary of LIP-155, the SPEL program, and `lez-payment-streams-core`
for the same object on every layer (types, fields, predicates, wire messages).
Do not introduce a second label for a concept that already has a canonical name
unless the code is truly a different view (then say so in the name).

Across the Rust FFI boundary, keep predictable suffixes so call sites read clearly:
`payment_streams_ffi_*` and `PaymentStreamsFfi*` for exported C symbols and `repr(C)`
views; `_from_ffi` when turning those structs into core types;
`_from_ffi_bytes` when the input is raw bytes (fixed layout or proto frame);
`_repr` for stable numeric encodings of enums for C;
split wide balances with `*_lo`/`*_hi` and helpers named `*_from_lo_hi` / `balance_pair`,
not vague synonyms such as `limbs`.

Prefer one obvious conversion name per pair of shapes (e.g. unified
`stream_config_from_ffi` instead of parallel `decoded` vs `view` helpers)
rather than accumulating near-synonyms.
