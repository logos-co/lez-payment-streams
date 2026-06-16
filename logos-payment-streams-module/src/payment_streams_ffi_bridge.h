#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct PsFfiDecodedVaultConfig {
    uint8_t version;
    uint8_t privacy_tier;
    uint8_t _padding[6];
    uint8_t owner[32];
    uint64_t vault_id;
    uint64_t next_stream_id;
    uint64_t total_allocated_lo;
    uint64_t total_allocated_hi;
} PsFfiDecodedVaultConfig;

typedef struct PsFfiDecodedVaultHolding {
    uint8_t version;
    uint8_t _padding[7];
} PsFfiDecodedVaultHolding;

typedef struct PsFfiDecodedStreamConfig {
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
} PsFfiDecodedStreamConfig;

typedef struct PsFfiDecodedClock {
    uint64_t block_id;
    uint64_t timestamp;
} PsFfiDecodedClock;

typedef struct PsFfiStreamFoldAtTime {
    PsFfiDecodedStreamConfig folded_stream;
    uint64_t accrued_lo;
    uint64_t accrued_hi;
    uint64_t unaccrued_lo;
    uint64_t unaccrued_hi;
    uint64_t as_of;
} PsFfiStreamFoldAtTime;

uint32_t ps_ffi_decode_vault_config(const uint8_t* data, size_t len, PsFfiDecodedVaultConfig* out);
uint32_t ps_ffi_decode_vault_holding(const uint8_t* data, size_t len, PsFfiDecodedVaultHolding* out);
uint32_t ps_ffi_decode_stream_config(const uint8_t* data, size_t len, PsFfiDecodedStreamConfig* out);
uint32_t ps_ffi_decode_clock(const uint8_t* data, size_t len, PsFfiDecodedClock* out);

uint32_t ps_ffi_fixed_clock_10_account_id(uint8_t out_account_id_bytes[32]);
uint32_t ps_ffi_authenticated_transfer_program_id(uint8_t out_bytes[32]);
uint32_t ps_ffi_derive_vault_account_ids(const uint8_t program_id_bytes[32],
                                         const uint8_t owner_account_id_bytes[32],
                                         uint64_t vault_id,
                                         uint8_t out_vault_config_account_id_bytes[32],
                                         uint8_t out_vault_holding_account_id_bytes[32]);
uint32_t ps_ffi_derive_stream_config_account_id(const uint8_t program_id_bytes[32],
                                                const uint8_t vault_config_account_id_bytes[32],
                                                uint64_t stream_id,
                                                uint8_t out_stream_config_account_id_bytes[32]);
uint32_t ps_ffi_fold_stream_at(const PsFfiDecodedStreamConfig* stream,
                               uint64_t as_of,
                               PsFfiStreamFoldAtTime* out_fold,
                               uint32_t* guest_error_out);

uint32_t ps_ffi_serialize_initialize_vault(uint64_t vault_id,
                                           uint8_t privacy_tier,
                                           uint8_t* out_ptr,
                                           size_t out_cap,
                                           size_t* out_len);
uint32_t ps_ffi_plan_initialize_vault(const uint8_t program_id_bytes[32],
                                      const uint8_t owner_account_id_bytes[32],
                                      uint64_t vault_id,
                                      uint8_t* accounts_hex_out,
                                      size_t accounts_hex_out_cap,
                                      size_t* accounts_hex_out_len);

uint32_t ps_ffi_serialize_deposit(uint64_t vault_id,
                                  uint64_t amount_lo,
                                  uint64_t amount_hi,
                                  const uint8_t authenticated_transfer_program_id_bytes[32],
                                  uint8_t* out_ptr,
                                  size_t out_cap,
                                  size_t* out_len);
uint32_t ps_ffi_plan_deposit(const uint8_t program_id_bytes[32],
                             const uint8_t owner_account_id_bytes[32],
                             uint64_t vault_id,
                             uint8_t* accounts_hex_out,
                             size_t accounts_hex_out_cap,
                             size_t* accounts_hex_out_len);

uint32_t ps_ffi_serialize_withdraw(uint64_t vault_id,
                                   uint64_t amount_lo,
                                   uint64_t amount_hi,
                                   uint8_t* out_ptr,
                                   size_t out_cap,
                                   size_t* out_len);
uint32_t ps_ffi_plan_withdraw(const uint8_t program_id_bytes[32],
                              const uint8_t owner_account_id_bytes[32],
                              uint64_t vault_id,
                              const uint8_t withdraw_to_account_id_bytes[32],
                              uint8_t* accounts_hex_out,
                              size_t accounts_hex_out_cap,
                              size_t* accounts_hex_out_len);

uint32_t ps_ffi_serialize_create_stream(uint64_t vault_id,
                                        uint64_t stream_id,
                                        const uint8_t provider_account_id_bytes[32],
                                        uint64_t rate,
                                        uint64_t allocation_lo,
                                        uint64_t allocation_hi,
                                        uint8_t* out_ptr,
                                        size_t out_cap,
                                        size_t* out_len);
uint32_t ps_ffi_plan_create_stream(const uint8_t program_id_bytes[32],
                                   const uint8_t owner_account_id_bytes[32],
                                   uint64_t vault_id,
                                   uint64_t stream_id,
                                   const uint8_t clock_account_id_bytes[32],
                                   uint8_t* accounts_hex_out,
                                   size_t accounts_hex_out_cap,
                                   size_t* accounts_hex_out_len);

uint32_t ps_ffi_serialize_pause_stream(uint64_t vault_id,
                                       uint64_t stream_id,
                                       uint8_t* out_ptr,
                                       size_t out_cap,
                                       size_t* out_len);
uint32_t ps_ffi_plan_pause_stream(const uint8_t program_id_bytes[32],
                                  const uint8_t owner_account_id_bytes[32],
                                  uint64_t vault_id,
                                  uint64_t stream_id,
                                  const uint8_t clock_account_id_bytes[32],
                                  uint8_t* accounts_hex_out,
                                  size_t accounts_hex_out_cap,
                                  size_t* accounts_hex_out_len);

uint32_t ps_ffi_serialize_resume_stream(uint64_t vault_id,
                                        uint64_t stream_id,
                                        uint8_t* out_ptr,
                                        size_t out_cap,
                                        size_t* out_len);
uint32_t ps_ffi_plan_resume_stream(const uint8_t program_id_bytes[32],
                                   const uint8_t owner_account_id_bytes[32],
                                   uint64_t vault_id,
                                   uint64_t stream_id,
                                   const uint8_t clock_account_id_bytes[32],
                                   uint8_t* accounts_hex_out,
                                   size_t accounts_hex_out_cap,
                                   size_t* accounts_hex_out_len);

uint32_t ps_ffi_serialize_top_up_stream(uint64_t vault_id,
                                        uint64_t stream_id,
                                        uint64_t increase_lo,
                                        uint64_t increase_hi,
                                        uint8_t* out_ptr,
                                        size_t out_cap,
                                        size_t* out_len);
uint32_t ps_ffi_plan_top_up_stream(const uint8_t program_id_bytes[32],
                                   const uint8_t owner_account_id_bytes[32],
                                   uint64_t vault_id,
                                   uint64_t stream_id,
                                   const uint8_t clock_account_id_bytes[32],
                                   uint8_t* accounts_hex_out,
                                   size_t accounts_hex_out_cap,
                                   size_t* accounts_hex_out_len);

uint32_t ps_ffi_serialize_close_stream(uint64_t vault_id,
                                       uint64_t stream_id,
                                       uint8_t* out_ptr,
                                       size_t out_cap,
                                       size_t* out_len);
uint32_t ps_ffi_plan_close_stream(const uint8_t program_id_bytes[32],
                                  const uint8_t owner_account_id_bytes[32],
                                  uint64_t vault_id,
                                  uint64_t stream_id,
                                  const uint8_t authority_account_id_bytes[32],
                                  const uint8_t clock_account_id_bytes[32],
                                  uint8_t* accounts_hex_out,
                                  size_t accounts_hex_out_cap,
                                  size_t* accounts_hex_out_len);

uint32_t ps_ffi_serialize_claim(uint64_t vault_id,
                                uint64_t stream_id,
                                uint8_t* out_ptr,
                                size_t out_cap,
                                size_t* out_len);
uint32_t ps_ffi_plan_claim(const uint8_t program_id_bytes[32],
                           const uint8_t owner_account_id_bytes[32],
                           uint64_t vault_id,
                           uint64_t stream_id,
                           const uint8_t provider_account_id_bytes[32],
                           const uint8_t clock_account_id_bytes[32],
                           uint8_t* accounts_hex_out,
                           size_t accounts_hex_out_cap,
                           size_t* accounts_hex_out_len);

#ifdef __cplusplus
}
#endif
