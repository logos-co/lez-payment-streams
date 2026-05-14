//! C ABI bindings for LEZ payment streams (LIP-155).
//!
//! Step 1 provides a wiring-only stub (`payment_streams_ffi_ping`) and the `Error`
//! enum so `cbindgen` and linkage can be exercised before real functionality lands.

/// Result codes for FFI entry points (`#[repr(C)]` for stable ABI).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Success = 0,
    NullPointer = 1,
    Internal = 2,
}

/// Placeholder exported symbol to validate linking, `cbindgen`, and CMake include paths.
#[no_mangle]
pub extern "C" fn payment_streams_ffi_ping() -> Error {
    Error::Success
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_returns_success() {
        assert_eq!(payment_streams_ffi_ping(), Error::Success);
    }
}
