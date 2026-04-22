//! Stable numeric codes for SPEL custom errors (`6001`–`6027`).
//!
//! Core and guest code return these values from fallible operations.
//! Human-readable text is supplied at each `spel_custom` site in the guest program,
//! so messages can vary by context while codes stay fixed.

pub const ERR_ZERO_DEPOSIT_AMOUNT: u32 = 6001;
pub const ERR_VERSION_MISMATCH: u32 = 6002;
pub const ERR_VAULT_ID_MISMATCH: u32 = 6003;
pub const ERR_INSUFFICIENT_FUNDS: u32 = 6004;
pub const ERR_ARITHMETIC_OVERFLOW: u32 = 6005;
pub const ERR_ZERO_WITHDRAW_AMOUNT: u32 = 6006;
pub const ERR_ZERO_STREAM_RATE: u32 = 6007;
pub const ERR_ZERO_STREAM_ALLOCATION: u32 = 6008;
pub const ERR_STREAM_ID_MISMATCH: u32 = 6009;
pub const ERR_TOTAL_ALLOCATED_OVERFLOW: u32 = 6010;
pub const ERR_INVALID_MOCK_TIMESTAMP: u32 = 6011;
pub const ERR_ALLOCATION_EXCEEDS_UNALLOCATED: u32 = 6012;
pub const ERR_NEXT_STREAM_ID_OVERFLOW: u32 = 6013;
pub const ERR_TIME_REGRESSION: u32 = 6014;
pub const ERR_STREAM_EXCEEDS_ALLOCATION: u32 = 6015;
pub const ERR_VAULT_OWNER_MISMATCH: u32 = 6016;
pub const ERR_STREAM_NOT_ACTIVE: u32 = 6017;
pub const ERR_STREAM_NOT_PAUSED: u32 = 6018;
pub const ERR_RESUME_ZERO_UNACCRUED: u32 = 6019;
pub const ERR_STREAM_CLOSED: u32 = 6020;
pub const ERR_ZERO_TOP_UP_AMOUNT: u32 = 6021;
pub const ERR_TOTAL_ALLOCATED_UNDERFLOW: u32 = 6022;
pub const ERR_CLOSE_UNAUTHORIZED: u32 = 6023;
pub const ERR_ZERO_CLAIM_AMOUNT: u32 = 6024;
pub const ERR_CLAIM_UNAUTHORIZED: u32 = 6025;
pub const ERR_INVALID_CLOCK_ACCOUNT: u32 = 6026;
/// Reserved. Unknown `InitializeVault` privacy tier bytes are rejected when the instruction is
/// deserialized (before the guest runs), so this code is not emitted by current program logic.
pub const ERR_INVALID_PRIVACY_TIER: u32 = 6027;
