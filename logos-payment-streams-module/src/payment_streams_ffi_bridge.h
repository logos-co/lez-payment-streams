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

uint32_t ps_ffi_decode_vault_config(const uint8_t* data, size_t len, PsFfiDecodedVaultConfig* out);
uint32_t ps_ffi_decode_vault_holding(const uint8_t* data, size_t len, PsFfiDecodedVaultHolding* out);
uint32_t ps_ffi_decode_stream_config(const uint8_t* data, size_t len, PsFfiDecodedStreamConfig* out);
uint32_t ps_ffi_decode_clock(const uint8_t* data, size_t len, PsFfiDecodedClock* out);

#ifdef __cplusplus
}
#endif
