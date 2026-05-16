//! Errors raised while parsing LIP‑155 protobuf frames or verifying LEZ canonical proofs.

/// Failures while decoding length-delimited protobuf fields or enforcing LEZ width limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// The input ended before a complete field or message could be read.
    UnexpectedEof,
    /// The protobuf wire encoding does not match the expected LIP‑155 message shape.
    InvalidWireFrame,
    /// `StreamParams.service_id` bytes are not valid UTF‑8 where LIP‑155 requires UTF‑8.
    InvalidServiceIdUtf8,
    /// `StreamParams.service_id` length exceeds the LEZ demo cap ([`crate::MAX_SERVICE_ID_LEN`]).
    ServiceIdTooLong,
    /// Stream `allocation` does not fit in the LIP‑155 protobuf `uint64` field (wire encode).
    AllocationExceedsProtobufUint64,
}

/// Top-level failures for Step 4 helpers (wire + NSSA verification + owner binding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OffChainError {
    Wire(WireError),
    /// A 32-byte x-only secp256k1 key (owner or session) failed NSSA validation.
    InvalidPublicKey,
    /// Derived public-account id from `owner_public_key` does not match the on-chain vault owner.
    OwnerKeyMismatch,
    /// Schnorr verification failed for the expected 32-byte LEZ canonical payload digest.
    BadSignature,
}

impl From<WireError> for OffChainError {
    fn from(value: WireError) -> Self {
        Self::Wire(value)
    }
}

impl From<super::canonical::VaultOwnerAuthCanonicalError> for OffChainError {
    fn from(value: super::canonical::VaultOwnerAuthCanonicalError) -> Self {
        match value {
            super::canonical::VaultOwnerAuthCanonicalError::InvalidServiceIdUtf8 => {
                Self::Wire(WireError::InvalidServiceIdUtf8)
            }
            super::canonical::VaultOwnerAuthCanonicalError::Borsh(_) => {
                Self::Wire(WireError::InvalidWireFrame)
            }
        }
    }
}
