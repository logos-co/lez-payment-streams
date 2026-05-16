#ifndef LEZ_PAYMENT_STREAMS_FFI_H
#define LEZ_PAYMENT_STREAMS_FFI_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>



/**
 * Byte length of one account entry in `plan_*_instruction_accounts` output.
 *
 * Each account is encoded as **64** lowercase ASCII hex nibbles for the **32** raw id bytes
 * (`[0-9a-f]`, no `0x`, no separators), suitable for `send_public_transaction` JSON `accounts`
 * entries alongside `logos-rln-module`.
 */
#define PaymentStreamsFfiPAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN 64

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
   * Malformed or unusable inputs (truncated payloads, unexpected protobuf shape, invalid fixed
   * sizes, invalid public key bytes, etc.).
   */
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_MALFORMED = 2,
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_BAD_VERSION = 3,
  /**
   * Step 3b policy predicates rejected; inspect [`PaymentStreamsFfiPolicyRejectReason`] out-parameters.
   */
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_POLICY_REJECTED = 4,
  /**
   * [`fold_stream`] could not evaluate (non-policy guest failure); inspect optional `guest_error_out`.
   */
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_STREAM_FOLD_FAILED = 5,
  /**
   * Step 4 off-chain proof failed (owner binding or Schnorr). There is no secondary out-reason enum;
   * distinction between owner mismatch and bad signature is only available through core Rust APIs or
   * by decomposing checks (verify digest vs verify full proposal).
   */
  PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_PROOF_INVALID = 6,
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
 * Decoded `VaultProof` fields (`owner_signature` included for verification helpers).
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiDecodedVaultProof {
  uint64_t vault_id;
  uint8_t provider_id[32];
  uint8_t owner_public_key[32];
  uint8_t owner_signature[64];
} PaymentStreamsFfiPaymentStreamsFfiDecodedVaultProof;

/**
 * Decoded protobuf `StreamProposal` mirrored for C hosts.
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProposal {
  struct PaymentStreamsFfiPaymentStreamsFfiDecodedVaultProof vault_proof;
  struct PaymentStreamsFfiPaymentStreamsFfiStreamParams params;
  uint8_t session_public_key[32];
} PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProposal;

/**
 * Decoded protobuf `StreamProof`.
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProof {
  uint64_t stream_id;
  uint8_t signature[64];
} PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProof;

/**
 * Borrowed byte range supplied by the host (interpreted as UTF-8 for string fields).
 *
 * Safety contract (matches [`borrow_input`]):
 * - When `len > 0`, `ptr` must reference `len` contiguous readable bytes for the duration of the call.
 * - When `len == 0`, `ptr` may be null or dangling (empty slice).
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiByteSpan {
  const uint8_t *ptr;
  uintptr_t len;
} PaymentStreamsFfiPaymentStreamsFfiByteSpan;

/**
 * Store query inputs used to build the canonical eligibility payload (integration plan N8).
 */
typedef struct PaymentStreamsFfiPaymentStreamsFfiCanonicalStoreQuery {
  struct PaymentStreamsFfiPaymentStreamsFfiByteSpan request_id;
  uint8_t include_data;
  uint8_t has_pubsub_topic;
  struct PaymentStreamsFfiPaymentStreamsFfiByteSpan pubsub_topic;
  const struct PaymentStreamsFfiPaymentStreamsFfiByteSpan *content_topics;
  uint32_t content_topics_len;
  uint8_t has_start_time;
  int64_t start_time;
  uint8_t has_end_time;
  int64_t end_time;
  const uint8_t *message_hashes;
  uint32_t message_hashes_len;
  uint8_t has_pagination_cursor;
  uint8_t pagination_cursor[32];
  uint8_t pagination_forward;
  uint8_t has_pagination_limit;
  uint64_t pagination_limit;
} PaymentStreamsFfiPaymentStreamsFfiCanonicalStoreQuery;

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
 * Writes the system authenticated-transfer [`ProgramId`] as **32** little-endian bytes (NSSA wire
 * layout: eight `u32` words), suitable for [`payment_streams_ffi_serialize_deposit_instruction`]'s
 * `authenticated_transfer_program_id_bytes` argument.
 *
 * Equivalent to reading `nssa::program::Program::authenticated_transfer_program().id()` on the Rust
 * side and flattening with `u32::to_le_bytes` in program-id order.
 *
 * # Safety
 *
 * `out_bytes` must be non-null and address **32** writable bytes.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_authenticated_transfer_program_id_bytes(uint8_t *out_bytes);

/**
 * Serializes an `initialize_vault` instruction for wallet JSON `instruction` (LE instruction bytes).
 *
 * `privacy_tier`: `0` = [`VaultPrivacyTier::Public`], `1` = [`VaultPrivacyTier::PseudonymousFunder`].
 * Any other value yields [`PaymentStreamsFfiStatus::Malformed`].
 *
 * # Safety
 *
 * See module-level FFI contracts (`out_ptr`, `out_cap`, `out_len`, two-phase sizing).
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_initialize_vault_instruction(uint64_t vault_id,
                                                                                                    uint8_t privacy_tier,
                                                                                                    uint8_t *out_ptr,
                                                                                                    uintptr_t out_cap,
                                                                                                    uintptr_t *out_len);

/**
 * Plans ordered account ids for `initialize_vault` (hex layout per module docs).
 *
 * # Safety
 *
 * See module-level FFI contracts (`program_id_bytes`, `owner_account_id_bytes`, account hex output).
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_plan_initialize_vault_instruction_accounts(const uint8_t *program_id_bytes,
                                                                                                        const uint8_t *owner_account_id_bytes,
                                                                                                        uint64_t vault_id,
                                                                                                        uint8_t *accounts_hex_out,
                                                                                                        uintptr_t accounts_hex_out_cap,
                                                                                                        uintptr_t *accounts_hex_out_len);

/**
 * Serializes a `deposit` instruction. `amount_lo` / `amount_hi` form LEZ `Balance` (`u128`).
 *
 * `authenticated_transfer_program_id_bytes`: NSSA wire `ProgramId` (**32** bytes). Hosts may fill this
 * with [`payment_streams_ffi_authenticated_transfer_program_id_bytes`] for standard deposits.
 *
 * # Safety
 *
 * See module-level FFI contracts (`authenticated_transfer_program_id_bytes`, `out_ptr`, `out_len`, two-phase sizing).
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_deposit_instruction(uint64_t vault_id,
                                                                                           uint64_t amount_lo,
                                                                                           uint64_t amount_hi,
                                                                                           const uint8_t *authenticated_transfer_program_id_bytes,
                                                                                           uint8_t *out_ptr,
                                                                                           uintptr_t out_cap,
                                                                                           uintptr_t *out_len);

/**
 * Plans ordered account ids for `deposit`.
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_plan_deposit_instruction_accounts(const uint8_t *program_id_bytes,
                                                                                               const uint8_t *owner_account_id_bytes,
                                                                                               uint64_t vault_id,
                                                                                               uint8_t *accounts_hex_out,
                                                                                               uintptr_t accounts_hex_out_cap,
                                                                                               uintptr_t *accounts_hex_out_len);

/**
 * Serializes a `withdraw` instruction (`amount_lo` / `amount_hi` → `Balance`).
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_withdraw_instruction(uint64_t vault_id,
                                                                                            uint64_t amount_lo,
                                                                                            uint64_t amount_hi,
                                                                                            uint8_t *out_ptr,
                                                                                            uintptr_t out_cap,
                                                                                            uintptr_t *out_len);

/**
 * Plans ordered account ids for `withdraw` (includes `withdraw_to_account_id_bytes`).
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_plan_withdraw_instruction_accounts(const uint8_t *program_id_bytes,
                                                                                                const uint8_t *owner_account_id_bytes,
                                                                                                uint64_t vault_id,
                                                                                                const uint8_t *withdraw_to_account_id_bytes,
                                                                                                uint8_t *accounts_hex_out,
                                                                                                uintptr_t accounts_hex_out_cap,
                                                                                                uintptr_t *accounts_hex_out_len);

/**
 * Serializes `create_stream` (`allocation_lo` / `allocation_hi` → `Balance`).
 *
 * # Safety
 *
 * See module-level FFI contracts (`provider_account_id_bytes`, output buffers).
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_create_stream_instruction(uint64_t vault_id,
                                                                                                 uint64_t stream_id,
                                                                                                 const uint8_t *provider_account_id_bytes,
                                                                                                 uint64_t rate,
                                                                                                 uint64_t allocation_lo,
                                                                                                 uint64_t allocation_hi,
                                                                                                 uint8_t *out_ptr,
                                                                                                 uintptr_t out_cap,
                                                                                                 uintptr_t *out_len);

/**
 * Plans ordered account ids for `create_stream` (vault owner stream layout + clock).
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_plan_create_stream_instruction_accounts(const uint8_t *program_id_bytes,
                                                                                                     const uint8_t *owner_account_id_bytes,
                                                                                                     uint64_t vault_id,
                                                                                                     uint64_t stream_id,
                                                                                                     const uint8_t *clock_account_id_bytes,
                                                                                                     uint8_t *accounts_hex_out,
                                                                                                     uintptr_t accounts_hex_out_cap,
                                                                                                     uintptr_t *accounts_hex_out_len);

/**
 * Serializes `pause_stream`.
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_pause_stream_instruction(uint64_t vault_id,
                                                                                                uint64_t stream_id,
                                                                                                uint8_t *out_ptr,
                                                                                                uintptr_t out_cap,
                                                                                                uintptr_t *out_len);

/**
 * Plans ordered account ids for `pause_stream`.
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_plan_pause_stream_instruction_accounts(const uint8_t *program_id_bytes,
                                                                                                    const uint8_t *owner_account_id_bytes,
                                                                                                    uint64_t vault_id,
                                                                                                    uint64_t stream_id,
                                                                                                    const uint8_t *clock_account_id_bytes,
                                                                                                    uint8_t *accounts_hex_out,
                                                                                                    uintptr_t accounts_hex_out_cap,
                                                                                                    uintptr_t *accounts_hex_out_len);

/**
 * Serializes `resume_stream`.
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_resume_stream_instruction(uint64_t vault_id,
                                                                                                 uint64_t stream_id,
                                                                                                 uint8_t *out_ptr,
                                                                                                 uintptr_t out_cap,
                                                                                                 uintptr_t *out_len);

/**
 * Plans ordered account ids for `resume_stream`.
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_plan_resume_stream_instruction_accounts(const uint8_t *program_id_bytes,
                                                                                                     const uint8_t *owner_account_id_bytes,
                                                                                                     uint64_t vault_id,
                                                                                                     uint64_t stream_id,
                                                                                                     const uint8_t *clock_account_id_bytes,
                                                                                                     uint8_t *accounts_hex_out,
                                                                                                     uintptr_t accounts_hex_out_cap,
                                                                                                     uintptr_t *accounts_hex_out_len);

/**
 * Serializes `top_up_stream` (`vault_total_allocated_increase_*` → `Balance`).
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_top_up_stream_instruction(uint64_t vault_id,
                                                                                                 uint64_t stream_id,
                                                                                                 uint64_t vault_total_allocated_increase_lo,
                                                                                                 uint64_t vault_total_allocated_increase_hi,
                                                                                                 uint8_t *out_ptr,
                                                                                                 uintptr_t out_cap,
                                                                                                 uintptr_t *out_len);

/**
 * Plans ordered account ids for `top_up_stream`.
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_plan_top_up_stream_instruction_accounts(const uint8_t *program_id_bytes,
                                                                                                     const uint8_t *owner_account_id_bytes,
                                                                                                     uint64_t vault_id,
                                                                                                     uint64_t stream_id,
                                                                                                     const uint8_t *clock_account_id_bytes,
                                                                                                     uint8_t *accounts_hex_out,
                                                                                                     uintptr_t accounts_hex_out_cap,
                                                                                                     uintptr_t *accounts_hex_out_len);

/**
 * Serializes `close_stream`.
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_close_stream_instruction(uint64_t vault_id,
                                                                                                uint64_t stream_id,
                                                                                                uint8_t *out_ptr,
                                                                                                uintptr_t out_cap,
                                                                                                uintptr_t *out_len);

/**
 * Plans ordered account ids for `close_stream` (`authority_account_id_bytes` signs).
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_plan_close_stream_instruction_accounts(const uint8_t *program_id_bytes,
                                                                                                    const uint8_t *owner_account_id_bytes,
                                                                                                    uint64_t vault_id,
                                                                                                    uint64_t stream_id,
                                                                                                    const uint8_t *authority_account_id_bytes,
                                                                                                    const uint8_t *clock_account_id_bytes,
                                                                                                    uint8_t *accounts_hex_out,
                                                                                                    uintptr_t accounts_hex_out_cap,
                                                                                                    uintptr_t *accounts_hex_out_len);

/**
 * Serializes `claim`.
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_claim_instruction(uint64_t vault_id,
                                                                                         uint64_t stream_id,
                                                                                         uint8_t *out_ptr,
                                                                                         uintptr_t out_cap,
                                                                                         uintptr_t *out_len);

/**
 * Plans ordered account ids for `claim` (`provider_account_id_bytes` signs).
 *
 * # Safety
 *
 * See module-level FFI contracts.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_plan_claim_instruction_accounts(const uint8_t *program_id_bytes,
                                                                                             const uint8_t *owner_account_id_bytes,
                                                                                             uint64_t vault_id,
                                                                                             uint64_t stream_id,
                                                                                             const uint8_t *provider_account_id_bytes,
                                                                                             const uint8_t *clock_account_id_bytes,
                                                                                             uint8_t *accounts_hex_out,
                                                                                             uintptr_t accounts_hex_out_cap,
                                                                                             uintptr_t *accounts_hex_out_len);

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

/**
 * Deserialize a protobuf `StreamProposal` into the flattened FFI view (LEZ width limits enforced).
 *
 * # Safety
 *
 * `(data_ptr, data_len)` must be a readable range; `ffi_out_proposal` must be non-null.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_parse_stream_proposal_bytes(const uint8_t *data_ptr,
                                                                                         uintptr_t data_len,
                                                                                         struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProposal *ffi_out_proposal);

/**
 * Serialize a `StreamProposal` protobuf frame from the flattened FFI view.
 *
 * When `out_ptr` is null this returns [`PaymentStreamsFfiStatus::Success`] after writing the required
 * buffer size to `out_len` (sizing pass).
 *
 * # Safety
 *
 * `ffi_proposal` must be non-null. When `out_ptr` is non-null it must address `out_cap` writable bytes.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_stream_proposal_bytes(const struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProposal *ffi_proposal,
                                                                                             uint8_t *out_ptr,
                                                                                             uintptr_t out_cap,
                                                                                             uintptr_t *out_len);

/**
 * Deserialize a protobuf `StreamProof`.
 *
 * # Safety
 *
 * `(data_ptr, data_len)` must be readable; `ffi_out_proof` must be non-null.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_parse_stream_proof_bytes(const uint8_t *data_ptr,
                                                                                      uintptr_t data_len,
                                                                                      struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProof *ffi_out_proof);

/**
 * Serialize a protobuf `StreamProof`.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_serialize_stream_proof_bytes(const struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProof *ffi_proof,
                                                                                          uint8_t *out_ptr,
                                                                                          uintptr_t out_cap,
                                                                                          uintptr_t *out_len);

/**
 * Write the 32-byte vault-owner canonical payload digest for a decoded proposal (`VaultProof.owner_signature` signs it).
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_vault_owner_auth_canonical_payload_digest_from_decoded_proposal(const struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProposal *ffi_proposal,
                                                                                                                             uint8_t *out_canonical_payload_digest);

/**
 * Verify `VaultProof.owner_signature` + derived owner binding against `VaultConfig.owner`.
 *
 * # Safety
 *
 * Inputs must follow [`borrow_input`] rules; `vault_owner_id` must address 32 readable bytes.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_verify_stream_proposal_vault_proof_bytes(const uint8_t *proposal_ptr,
                                                                                                      uintptr_t proposal_len,
                                                                                                      const uint8_t *vault_owner_id);

/**
 * Write the 32-byte Store eligibility `canonical_payload_digest` described in integration plan N8.
 *
 * # Safety
 *
 * `query` must be non-null and all nested spans must satisfy [`borrow_input`] rules.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_store_eligibility_canonical_payload_digest(const struct PaymentStreamsFfiPaymentStreamsFfiCanonicalStoreQuery *query,
                                                                                                        uint8_t *out_canonical_payload_digest);

/**
 * Verify `StreamProof.signature` over the canonical Store query described by `query`.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_verify_stream_proof_for_store_query(const uint8_t *proof_ptr,
                                                                                                 uintptr_t proof_len,
                                                                                                 const uint8_t *session_public_key,
                                                                                                 const struct PaymentStreamsFfiPaymentStreamsFfiCanonicalStoreQuery *query);

/**
 * Sign a 32-byte `canonical_payload_digest` with a 32-byte NSSA private key (Schnorr signature writes 64 bytes).
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_sign_canonical_payload_digest(const uint8_t *secret_key,
                                                                                           const uint8_t *canonical_payload_digest,
                                                                                           uint8_t *out_signature);

/**
 * Verify a 32-byte `canonical_payload_digest` against a 64-byte Schnorr signature and 32-byte public key.
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_verify_canonical_payload_digest(const uint8_t *public_key,
                                                                                             const uint8_t *canonical_payload_digest,
                                                                                             const uint8_t *signature);

#endif  /* LEZ_PAYMENT_STREAMS_FFI_H */
