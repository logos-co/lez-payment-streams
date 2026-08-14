#include "payment_streams_ffi_bridge.h"

#include <lez_payment_streams_ffi.h>
#include <string.h>

static uint32_t map_status(PaymentStreamsFfiPaymentStreamsFfiStatus status) {
    return (uint32_t)status;
}

uint32_t ps_ffi_decode_vault_config(const uint8_t* data, size_t len, PsFfiDecodedVaultConfig* out) {
    if (out == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    PaymentStreamsFfiPaymentStreamsFfiDecodedVaultConfig decoded;
    const PaymentStreamsFfiPaymentStreamsFfiStatus status =
        payment_streams_ffi_decode_vault_config_bytes(data, len, &decoded);
    if (status == PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_SUCCESS) {
        memcpy(out, &decoded, sizeof(*out));
    }
    return map_status(status);
}

uint32_t ps_ffi_decode_vault_holding(const uint8_t* data, size_t len, PsFfiDecodedVaultHolding* out) {
    if (out == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    PaymentStreamsFfiPaymentStreamsFfiDecodedVaultHolding decoded;
    const PaymentStreamsFfiPaymentStreamsFfiStatus status =
        payment_streams_ffi_decode_vault_holding_bytes(data, len, &decoded);
    if (status == PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_SUCCESS) {
        memcpy(out, &decoded, sizeof(*out));
    }
    return map_status(status);
}

uint32_t ps_ffi_decode_stream_config(const uint8_t* data, size_t len, PsFfiDecodedStreamConfig* out) {
    if (out == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig decoded;
    const PaymentStreamsFfiPaymentStreamsFfiStatus status =
        payment_streams_ffi_decode_stream_config_bytes(data, len, &decoded);
    if (status == PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_SUCCESS) {
        memcpy(out, &decoded, sizeof(*out));
    }
    return map_status(status);
}

uint32_t ps_ffi_decode_clock(const uint8_t* data, size_t len, PsFfiDecodedClock* out) {
    if (out == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    PaymentStreamsFfiPaymentStreamsFfiDecodedClock decoded;
    const PaymentStreamsFfiPaymentStreamsFfiStatus status =
        payment_streams_ffi_decode_clock_account_data_bytes(data, len, &decoded);
    if (status == PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_SUCCESS) {
        memcpy(out, &decoded, sizeof(*out));
    }
    return map_status(status);
}
