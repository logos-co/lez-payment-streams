#ifndef LEZ_PAYMENT_STREAMS_FFI_H
#define LEZ_PAYMENT_STREAMS_FFI_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>



/**
 * Outcome codes returned from `payment_streams_*` FFI functions (`Success` plus failures; stable
 * `repr(u32)` enumerators in `lez_payment_streams_ffi.h` from cbindgen).
 *
 * Rust-only helpers also use this as the `E` in `Result<T, E>` for recoverable failures (`Success`
 * is never used in `Err`).
 */
enum PaymentStreamsFfiPaymentStreamsFfiStatus {
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_SUCCESS = 0,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER = 1,
  /**
   * Malformed/unusable inputs (truncated payloads, unexpected wire shape, invalid fixed sizes).
   */
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_MALFORMED = 2,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_BAD_VERSION = 3,
  /**
   * Policy predicates rejected cleanly; inspect [`PaymentStreamsFfiPolicyRejectReason`] out-parameters.
   */
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_POLICY_REJECTED = 4,
  /**
   * [`fold_stream`] could not evaluate (non-policy guest failure); inspect optional `guest_error_out`.
   */
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_STREAM_FOLD_FAILED = 5,
};
typedef uint32_t PaymentStreamsFfiPaymentStreamsFfiStatus;

enum PaymentStreamsFfiClockAccountChoice {
  PAYMENT_STREAMS_FFI_CLOCK_ACCOUNT_CHOICE_CLOCK01 = 0,
  PAYMENT_STREAMS_FFI_CLOCK_ACCOUNT_CHOICE_CLOCK10 = 1,
  PAYMENT_STREAMS_FFI_CLOCK_ACCOUNT_CHOICE_CLOCK50 = 2,
};
typedef uint32_t PaymentStreamsFfiClockAccountChoice;

/**
 * Stable policy rejection codes for FFI consumers.
 *
 * Values `0..=8` mirror [`lez_payment_streams_core::PolicyRejectReason`] today (`repr(u32)`).
 * `Unknown` (`9`) is reserved for forward compatibility when core adds `#[non_exhaustive]` variants
 * before this FFI crate’s rejection mapping catches up.
 *
 * Hosts map these to Store-style eligibility buckets (see payment streams integration docs / LIP‑155):
 * most predicate outcomes map to `PARAMS_REJECTED`, `StreamNotActive` maps to `STREAM_NOT_ACTIVE`,
 * and proof-layer failures use `PROOF_INVALID`. Until a host defines finer rules, treat `Unknown` like `PARAMS_REJECTED`.
 */
enum PaymentStreamsFfiPaymentStreamsFfiPolicyRejectReason {
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_POLICY_REJECT_REASON_RATE_BELOW_POLICY_MIN = 0,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_POLICY_REJECT_REASON_ALLOCATION_BELOW_POLICY_MIN = 1,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_POLICY_REJECT_REASON_CREATE_STREAM_DEADLINE_INVALID = 2,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_POLICY_REJECT_REASON_UNALLOCATED_INSUFFICIENT = 3,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_POLICY_REJECT_REASON_RATE_BELOW_ACCEPTED_PARAMS = 4,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_POLICY_REJECT_REASON_ALLOCATION_BELOW_ACCEPTED_PARAMS = 5,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_POLICY_REJECT_REASON_PROVIDER_MISMATCH = 6,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_POLICY_REJECT_REASON_STREAM_NOT_ACTIVE = 7,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_POLICY_REJECT_REASON_RESPONSE_TOO_LARGE = 8,
  /**
   * Core [`PolicyRejectReason`] variant not yet surfaced by this FFI layer.
   */
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_POLICY_REJECT_REASON_UNKNOWN = 9,
};
typedef uint32_t PaymentStreamsFfiPaymentStreamsFfiPolicyRejectReason;

/**
 * Decoded `VaultConfig` fields exposed across the C ABI boundary.
 *
 * `version` and `privacy_tier` are paired with explicit `_padding` so all
 * padding bytes are named in the generated C header (compiler-inserted padding
 * would be invisible to `cbindgen` callers), `owner` sits at an 8-byte offset,
 * and the same two-`u8` header shape is shared with
 * [`PaymentStreamsFfiDecodedStreamConfig`].
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiDecodedVaultConfig {
  uint8_t version;
  uint8_t privacy_tier;
  uint8_t _padding[6];
  uint8_t owner[32];
  uint64_t vault_id;
  uint64_t next_stream_id;
  uint64_t total_allocated_lo;
  uint64_t total_allocated_hi;
} PaymentStreamsFfiPaymentStreamsFfiDecodedVaultConfig;

typedef struct PaymentStreamsFfiPaymentStreamsFfiDecodedVaultHolding {
  uint8_t version;
  uint8_t _padding[7];
} PaymentStreamsFfiPaymentStreamsFfiDecodedVaultHolding;

/**
 * Decoded `StreamConfig` fields exposed across the C ABI boundary.
 *
 * `_padding` matches [`PaymentStreamsFfiDecodedVaultConfig`]: explicit padding
 * for a stable, fully described `repr(C)` layout in C bindings.
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig {
  uint8_t version;
  uint8_t stream_state;
  uint8_t _padding[6];
  uint64_t stream_id;
  uint8_t provider[32];
  uint64_t rate_tokens_per_second;
  uint64_t allocation_lo;
  uint64_t allocation_hi;
  uint64_t accrued_lo;
  uint64_t accrued_hi;
  uint64_t accrued_as_of;
} PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig;

typedef struct PaymentStreamsFfiPaymentStreamsFfiDecodedClock {
  uint64_t block_id;
  uint64_t timestamp;
} PaymentStreamsFfiPaymentStreamsFfiDecodedClock;

/**
 * [`lez_payment_streams_core::StreamFoldedAtTime`] mirrored for C callers.
 *
 * Numeric fields split LEZ balances into deterministic little-endian `lo` / `hi` halves.
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiStreamFoldAtTime {
  struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig folded_stream;
  uint64_t accrued_lo;
  uint64_t accrued_hi;
  uint64_t unaccrued_lo;
  uint64_t unaccrued_hi;
  uint64_t as_of;
} PaymentStreamsFfiPaymentStreamsFfiStreamFoldAtTime;

/**
 * Accepted / proposed [`StreamParams`] fields without heap indirection (`service_id` prefix + fixed buffer tail).
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiStreamParams {
  uint64_t rate;
  uint64_t allocation_lo;
  uint64_t allocation_hi;
  uint64_t create_stream_deadline;
  uint32_t service_id_len;
  uint32_t _padding;
  uint8_t service_id_bytes[128];
} PaymentStreamsFfiPaymentStreamsFfiStreamParams;

/**
 * [`StreamProviderPolicy`] snapshot crossing the FFI (wide balances split as `lo` / `hi` `u64` halves).
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiStreamProviderPolicy {
  uint64_t min_rate;
  uint64_t min_allocation_lo;
  uint64_t min_allocation_hi;
  uint64_t max_create_stream_deadline_delay;
  uint64_t vault_proof_max_response_bytes;
} PaymentStreamsFfiPaymentStreamsFfiStreamProviderPolicy;

typedef struct PaymentStreamsFfiPaymentStreamsFfiProposalCheckInputs {
  struct PaymentStreamsFfiPaymentStreamsFfiStreamParams params;
  struct PaymentStreamsFfiPaymentStreamsFfiStreamProviderPolicy policy;
  uint64_t vault_holding_balance_lo;
  uint64_t vault_holding_balance_hi;
  uint64_t vault_total_allocated_lo;
  uint64_t vault_total_allocated_hi;
  uint64_t now;
} PaymentStreamsFfiPaymentStreamsFfiProposalCheckInputs;

/**
 * Pinned session terms surfaced on the wire as [`lez_payment_streams_core::AcceptedStreamTerms`].
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiAcceptedStreamTerms {
  struct PaymentStreamsFfiPaymentStreamsFfiStreamParams params;
  uint8_t provider_id[32];
  struct PaymentStreamsFfiPaymentStreamsFfiStreamProviderPolicy policy_at_acceptance;
} PaymentStreamsFfiPaymentStreamsFfiAcceptedStreamTerms;

/**
 * Placeholder linkage smoke (may be removed once the FFI surface is fully wired).
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_ping(void);

/**
 * Decode serialized `VaultConfig` bytes copied from sequencer account payload.
 *
 * `vault_cfg_decoded` is Borsh + version-checked core state; writes the flattened `repr(C)` struct via
 * `ffi_out_decoded` / `ffi_out_decoded_mut`.
 *
 * # Safety
 *
 * - `(data_ptr, data_len)` form a readable range: when `data_len > 0`, `data_ptr` must be valid for
 *   `data_len` bytes of immutable access until return; when `data_len == 0`, `data_ptr` may be null
 *   or dangling.
 * - When `ffi_out_decoded` is non-null it must reference memory valid for a full overwrite of
 *   [`PaymentStreamsFfiDecodedVaultConfig`] until this function returns.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_decode_vault_config_bytes(const uint8_t *data_ptr,
                                                                                       uintptr_t data_len,
                                                                                       struct PaymentStreamsFfiPaymentStreamsFfiDecodedVaultConfig *ffi_out_decoded);

/**
 * Decode serialized `VaultHolding` payload.
 *
 * `vault_holding_decoded` is Borsh + version-checked core state; fills `ffi_out_decoded*`.
 *
 * # Safety
 *
 * - `(data_ptr, data_len)` form a readable range: when `data_len > 0`, `data_ptr` must be valid for
 *   `data_len` bytes of immutable access until return; when `data_len == 0`, `data_ptr` may be null
 *   or dangling.
 * - When `ffi_out_decoded` is non-null it must reference memory valid for a full overwrite of
 *   [`PaymentStreamsFfiDecodedVaultHolding`] until this function returns.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_decode_vault_holding_bytes(const uint8_t *data_ptr,
                                                                                        uintptr_t data_len,
                                                                                        struct PaymentStreamsFfiPaymentStreamsFfiDecodedVaultHolding *ffi_out_decoded);

/**
 * Decode serialized `StreamConfig` payload.
 *
 * `stream_cfg_decoded` is Borsh + version-checked core state; fills `ffi_out_decoded*`.
 *
 * # Safety
 *
 * - `(data_ptr, data_len)` form a readable range: when `data_len > 0`, `data_ptr` must be valid for
 *   `data_len` bytes of immutable access until return; when `data_len == 0`, `data_ptr` may be null
 *   or dangling.
 * - When `ffi_out_decoded` is non-null it must reference memory valid for a full overwrite of
 *   [`PaymentStreamsFfiDecodedStreamConfig`] until this function returns.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_decode_stream_config_bytes(const uint8_t *data_ptr,
                                                                                        uintptr_t data_len,
                                                                                        struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig *ffi_out_decoded);

/**
 * Decode serialized `ClockAccountData` (`block_id` + timestamp seconds).
 *
 * `clock_decoded` is Borsh core payload; fills `ffi_out_decoded*`.
 *
 * # Safety
 *
 * - `(data_ptr, data_len)` form a readable range: when `data_len > 0`, `data_ptr` must be valid for
 *   `data_len` bytes of immutable access until return; when `data_len == 0`, `data_ptr` may be null
 *   or dangling.
 * - When `ffi_out_decoded` is non-null it must reference memory valid for a full overwrite of
 *   [`PaymentStreamsFfiDecodedClock`] until this function returns.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_decode_clock_account_data_bytes(const uint8_t *data_ptr,
                                                                                             uintptr_t data_len,
                                                                                             struct PaymentStreamsFfiPaymentStreamsFfiDecodedClock *ffi_out_decoded);

/**
 * Copy deterministic CLOCK literal account ids enforced by genesis.
 *
 * # Safety
 *
 * `out_account_id_bytes` must be non-null and valid for writes of exactly 32 bytes until return.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_fixed_clock_account_id(PaymentStreamsFfiClockAccountChoice selector,
                                                                                    uint8_t *out_account_id_bytes);

/**
 * Derive `(vault_config, vault_holding)` account ids for `(owner, vault_id)`.
 *
 * # Safety
 *
 * - `program_id_bytes` must address 32 contiguous bytes readable until return (NSSA [`ProgramId`]
 *   wire encoding, eight little-endian `u32` words).
 * - `owner_account_id_bytes` must address 32 contiguous bytes readable until return.
 * - Both `out_vault_config_account_id_bytes` / `out_vault_holding_account_id_bytes` must be non-null,
 *   distinct or non-overlapping writable regions spanning 32 bytes each until return (the function
 *   writes independently to both).
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_derive_vault_account_ids(const uint8_t *program_id_bytes,
                                                                                      const uint8_t *owner_account_id_bytes,
                                                                                      uint64_t vault_id,
                                                                                      uint8_t *out_vault_config_account_id_bytes,
                                                                                      uint8_t *out_vault_holding_account_id_bytes);

/**
 * Derive `stream_config` account identifier for `(vault_cfg, stream_id)`.
 *
 * # Safety
 *
 * - `program_id_bytes` must address 32 contiguous bytes readable until return (NSSA [`ProgramId`]
 *   wire encoding, eight little-endian `u32` words).
 * - `vault_config_account_id_bytes` must address 32 contiguous raw account id bytes readable until
 *   return.
 * - `out_stream_config_account_id_bytes` must be non-null and writable for 32 bytes until return.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_derive_stream_config_account_id(const uint8_t *program_id_bytes,
                                                                                             const uint8_t *vault_config_account_id_bytes,
                                                                                             uint64_t stream_id,
                                                                                             uint8_t *out_stream_config_account_id_bytes);

/**
 * Fold lazy accrual from decoded `StreamConfig` data carried as [`PaymentStreamsFfiDecodedStreamConfig`]
 * (the struct produced by [`payment_streams_ffi_decode_stream_config_bytes`]).
 *
 * On [`PaymentStreamsFfiStatus::StreamFoldFailed`], writes optional precise context to
 * `guest_error_out` when non-null using stable [`lez_payment_streams_core::ErrorCode`] `repr(u32)` values (`6001+`).
 *
 * # Safety
 *
 * - `ffi_decoded_stream`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiDecodedStreamConfig`].
 * - `ffi_out_fold`: non-null, aligned pointer valid for writable access spanning one [`PaymentStreamsFfiStreamFoldAtTime`].
 * - `guest_error_out`: either null or a non-null, aligned pointer writable for exactly one `uint32_t`.
 * - Required null pointers return [`PaymentStreamsFfiStatus::NullPointer`] instead of touching outputs.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_fold_stream(const struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig *ffi_decoded_stream,
                                                                         uint64_t as_of,
                                                                         struct PaymentStreamsFfiPaymentStreamsFfiStreamFoldAtTime *ffi_out_fold,
                                                                         uint32_t *guest_error_out);

/**
 * Proposal-phase policy gate (runs on payer + provider before signing).
 *
 * On [`PaymentStreamsFfiStatus::PolicyRejected`], `ffi_out_policy_reject` carries a
 * [`crate::PaymentStreamsFfiPolicyRejectReason`] code (`0..=8` mirrors core; `Unknown` covers
 * future [`lez_payment_streams_core::PolicyRejectReason`] variants not yet mapped explicitly).
 *
 * # Safety
 *
 * - `ffi_inputs`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiProposalCheckInputs`].
 * - `ffi_out_policy_reject`: non-null, aligned pointer writable for exactly one [`PaymentStreamsFfiPolicyRejectReason`].
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_proposal_satisfies_policy(const struct PaymentStreamsFfiPaymentStreamsFfiProposalCheckInputs *ffi_inputs,
                                                                                       PaymentStreamsFfiPaymentStreamsFfiPolicyRejectReason *ffi_out_policy_reject);

/**
 * Deadline-only predicate extracted from proposal checks (`create_stream_deadline` clock band).
 *
 * # Safety
 *
 * - `ffi_out_policy_reject`: non-null, aligned pointer writable for exactly one [`PaymentStreamsFfiPolicyRejectReason`].
 * - A null `ffi_out_policy_reject` returns [`PaymentStreamsFfiStatus::NullPointer`] instead of touching the slot.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_create_stream_deadline_satisfies_policy_as_of(uint64_t params_create_stream_deadline,
                                                                                                           uint64_t policy_max_create_stream_deadline_delay,
                                                                                                           uint64_t check_time,
                                                                                                           PaymentStreamsFfiPaymentStreamsFfiPolicyRejectReason *ffi_out_policy_reject);

/**
 * First service proof binds folded on-chain state to accepted negotiation terms.
 *
 * # Safety
 *
 * - `ffi_decoded_stream`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiDecodedStreamConfig`].
 * - `ffi_accepted_params`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiStreamParams`].
 * - `proposal_provider_id_bytes`: non-null, aligned pointer valid for immutable reads spanning 32 bytes.
 * - `ffi_out_policy_reject`: non-null, aligned pointer writable for exactly one [`PaymentStreamsFfiPolicyRejectReason`].
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_new_stream_satisfies_proposal(const struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig *ffi_decoded_stream,
                                                                                           const struct PaymentStreamsFfiPaymentStreamsFfiStreamParams *ffi_accepted_params,
                                                                                           const uint8_t *proposal_provider_id_bytes,
                                                                                           PaymentStreamsFfiPaymentStreamsFfiPolicyRejectReason *ffi_out_policy_reject);

/**
 * Ongoing proofs must respect the pinned policy snapshot + active stream state.
 *
 * # Safety
 *
 * - `ffi_decoded_stream`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiDecodedStreamConfig`].
 * - `ffi_accepted_terms_snapshot`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiAcceptedStreamTerms`].
 * - `ffi_out_policy_reject`: non-null, aligned pointer writable for exactly one [`PaymentStreamsFfiPolicyRejectReason`].
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_stream_satisfies_policy(const struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig *ffi_decoded_stream,
                                                                                     const struct PaymentStreamsFfiPaymentStreamsFfiAcceptedStreamTerms *ffi_accepted_terms_snapshot,
                                                                                     PaymentStreamsFfiPaymentStreamsFfiPolicyRejectReason *ffi_out_policy_reject);

/**
 * Outbound vault proof payload sizing guard enforced by MVP providers (`response_within_policy`).
 *
 * Argument order mirrors core: serialized response byte length first, then policy snapshot.
 *
 * # Safety
 *
 * - `ffi_policy_snapshot`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiStreamProviderPolicy`].
 * - `ffi_out_policy_reject`: non-null, aligned pointer writable for exactly one [`PaymentStreamsFfiPolicyRejectReason`].
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_response_within_policy(uint64_t response_payload_byte_len,
                                                                                    const struct PaymentStreamsFfiPaymentStreamsFfiStreamProviderPolicy *ffi_policy_snapshot,
                                                                                    PaymentStreamsFfiPaymentStreamsFfiPolicyRejectReason *ffi_out_policy_reject);

#endif  /* LEZ_PAYMENT_STREAMS_FFI_H */
