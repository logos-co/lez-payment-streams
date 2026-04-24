//! Stable numeric codes for SPEL custom errors (`6001`–`6026`).
//!
//! `ErrorCode` is `#[repr(u32)]` for type-safe use in core return types and test assertions.
//! Code 6011 (`InvalidMockTimestamp`) was removed; all codes that were above it shifted down by one.
//! Code 6026 (`InvalidPrivacyTier`) is reserved: unknown `InitializeVault` privacy tier bytes are
//! rejected during host-side instruction deserialization (before the guest runs), so this code is
//! not emitted by current guest logic.

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ZeroDepositAmount = 6001,
    VersionMismatch = 6002,
    VaultIdMismatch = 6003,
    InsufficientFunds = 6004,
    ArithmeticOverflow = 6005,
    ZeroWithdrawAmount = 6006,
    ZeroStreamRate = 6007,
    ZeroStreamAllocation = 6008,
    StreamIdMismatch = 6009,
    TotalAllocatedOverflow = 6010,
    AllocationExceedsUnallocated = 6011,
    NextStreamIdOverflow = 6012,
    TimeRegression = 6013,
    StreamExceedsAllocation = 6014,
    VaultOwnerMismatch = 6015,
    StreamNotActive = 6016,
    StreamNotPaused = 6017,
    ResumeZeroUnaccrued = 6018,
    StreamClosed = 6019,
    ZeroTopUpAmount = 6020,
    TotalAllocatedUnderflow = 6021,
    CloseUnauthorized = 6022,
    ZeroClaimAmount = 6023,
    ClaimUnauthorized = 6024,
    InvalidClockAccount = 6025,
    /// Reserved. Unknown `InitializeVault` privacy tier bytes are rejected when the instruction
    /// is deserialized (before the guest runs), so this code is not emitted by current program logic.
    InvalidPrivacyTier = 6026,
}
