#ifndef LEZ_PAYMENT_STREAMS_FFI_H
#define LEZ_PAYMENT_STREAMS_FFI_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Result codes for FFI entry points (`#[repr(C)]` for stable ABI).
 */
typedef enum PaymentStreamsFfiError {
  PAYMENT_STREAMS_FFI_ERROR_SUCCESS = 0,
  PAYMENT_STREAMS_FFI_ERROR_NULL_POINTER = 1,
  PAYMENT_STREAMS_FFI_ERROR_INTERNAL = 2,
} PaymentStreamsFfiError;

/**
 * Placeholder exported symbol to validate linking, `cbindgen`, and CMake include paths.
 */
enum PaymentStreamsFfiError payment_streams_ffi_ping(void);

#endif  /* LEZ_PAYMENT_STREAMS_FFI_H */
