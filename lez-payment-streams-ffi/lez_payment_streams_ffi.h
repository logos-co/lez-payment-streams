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
};
typedef uint32_t PaymentStreamsFfiPaymentStreamsFfiStatus;

enum PaymentStreamsFfiClockAccountChoice {
  PAYMENT_STREAMS_FFI_CLOCK_ACCOUNT_CHOICE_CLOCK01 = 0,
  PAYMENT_STREAMS_FFI_CLOCK_ACCOUNT_CHOICE_CLOCK10 = 1,
  PAYMENT_STREAMS_FFI_CLOCK_ACCOUNT_CHOICE_CLOCK50 = 2,
};
typedef uint32_t PaymentStreamsFfiClockAccountChoice;

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
 * Placeholder linkage smoke exported in Step 1 (can be removed in later steps).
 */
PaymentStreamsFfiPaymentStreamsFfiStatus payment_streams_ffi_ping(void);

/**
 * Decode serialized `VaultConfig` bytes copied from sequencer account payload.
 *
 * `vault_cfg_decoded` is Borsh + version-checked core state; writes the flattened FFI view via
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

#endif  /* LEZ_PAYMENT_STREAMS_FFI_H */
