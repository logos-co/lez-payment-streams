# Hand-off — Payment Streams Integration

You are picking up work on integrating LIP-155 payment streams
into the Logos stack as a demo.

Start with `integration-plan-v2.md` in this directory.
It contains the full plan, onboarding reading order,
prerequisites, component overview, decisions, notes,
and numbered integration steps (Step 3 splits into 3a/3b; Steps 6–18 after Rust FFI) with definitions of done.

LIP-155 is normative in `rfc-index`; LEZ wire bindings are in
`rfc-index/docs/ift-ts/raw/payment-streams.md` (LEZ off-chain integration).

`logos-architecture-overview.md` in this directory
covers the architectural context
(hosts vs. modules, FFI boundaries, Qt roles, LEZ chain side).

## Store access via `delivery_module`

Paid Store demo steps (Step 16 onward) need a supported way to issue Store
queries through `logos-delivery-module` (for example `queryStore` on upstream
`master` once the Delivery team lands their roadmap design).

We do not integrate against our own `logosdelivery_query_store` /
`queryStore` PR stack. That approach is superseded by the upstream plan.
Open PRs against `logos-delivery` and `logos-delivery-module` may remain for
reference but must not be pinned in flakes, forked locally, or treated as the
integration path.

Until upstream Store query exposure is on `master`, continue work that does not
call `delivery_module` for Store retrieval (Rust FFI Steps 1–5, Step 6 closed
decision, Steps 7–15 including Universal module bootstrap in Step 9).
Step 6 in the integration plan records no local Store query exposure
(done, won't fix).

See integration-plan-v2.md (N6, D3 for wallet 491 + 19, and the component overview for
`logos-delivery-module`) and docs/feature-branch-pins.md for wallet pins.

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
