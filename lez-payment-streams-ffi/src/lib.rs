//! C ABI for LEZ payment streams (LIP-155).

mod decode;
mod instruction_abi;
mod policy_abi;
mod proof_abi;

pub use instruction_abi::*;
pub use lez_payment_streams_core::{
    derive_stream_config_account_id, derive_vault_account_ids, VaultConfig,
    CLOCK_01_PROGRAM_ACCOUNT_ID, CLOCK_10_PROGRAM_ACCOUNT_ID, CLOCK_50_PROGRAM_ACCOUNT_ID,
};
pub use nssa_core::account::AccountId;
pub use nssa_core::program::ProgramId;
pub use policy_abi::*;
pub use proof_abi::*;

use core::slice;

use decode::DecodeFault;
use lez_payment_streams_core::{StreamState, VaultPrivacyTier};
use nssa_core::account::Balance;

/// Outcome codes returned from `payment_streams_*` FFI functions (`Success` plus failures; stable
/// `repr(u32)` enumerators in `lez_payment_streams_ffi.h` from cbindgen).
///
/// Rust-only helpers also use this as the `E` in `Result<T, E>` for recoverable failures (`Success`
/// is never used in `Err`).
#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PaymentStreamsFfiStatus {
    Success = 0,
    NullPointer = 1,
    /// Malformed or unusable inputs (truncated payloads, unexpected protobuf shape, invalid fixed
    /// sizes, invalid public key bytes, etc.).
    Malformed = 2,
    BadVersion = 3,
    /// Step 3b policy predicates rejected; inspect [`PaymentStreamsFfiPolicyRejectReason`] out-parameters.
    PolicyRejected = 4,
    /// [`fold_stream`] could not evaluate (non-policy guest failure); inspect optional `guest_error_out`.
    StreamFoldFailed = 5,
    /// Step 4 off-chain proof failed (owner binding or Schnorr). There is no secondary out-reason enum;
    /// distinction between owner mismatch and bad signature is only available through core Rust APIs or
    /// by decomposing checks (verify digest vs verify full proposal).
    ProofInvalid = 6,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ClockAccountChoice {
    Clock01 = 0,
    Clock10 = 1,
    Clock50 = 2,
}

/// Decoded `VaultConfig` fields exposed across the C ABI boundary.
///
/// `version` and `privacy_tier` are paired with explicit `_padding` so all
/// padding bytes are named in the generated C header (compiler-inserted padding
/// would be invisible to `cbindgen` callers), `owner` sits at an 8-byte offset,
/// and the same two-`u8` header shape is shared with
/// [`PaymentStreamsFfiDecodedStreamConfig`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiDecodedVaultConfig {
    pub version: u8,
    pub privacy_tier: u8,
    pub _padding: [u8; 6],
    pub owner: [u8; 32],
    pub vault_id: u64,
    pub next_stream_id: u64,
    pub total_allocated_lo: u64,
    pub total_allocated_hi: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiDecodedVaultHolding {
    pub version: u8,
    pub _padding: [u8; 7],
}

/// Decoded `StreamConfig` fields exposed across the C ABI boundary.
///
/// `_padding` matches [`PaymentStreamsFfiDecodedVaultConfig`]: explicit padding
/// for a stable, fully described `repr(C)` layout in C bindings.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiDecodedStreamConfig {
    pub version: u8,
    pub stream_state: u8,
    pub _padding: [u8; 6],
    pub stream_id: u64,
    pub provider: [u8; 32],
    pub rate_tokens_per_second: u64,
    pub allocation_lo: u64,
    pub allocation_hi: u64,
    pub accrued_lo: u64,
    pub accrued_hi: u64,
    pub accrued_as_of: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiDecodedClock {
    pub block_id: u64,
    pub timestamp: u64,
}

/// Mirrors [`lez_payment_streams_core::MAX_SERVICE_ID_LEN`] — array sizes must stay literals for portable C headers.
pub const PAYMENT_STREAMS_FFI_MAX_SERVICE_ID_LEN: usize =
    lez_payment_streams_core::MAX_SERVICE_ID_LEN;

/// [`StreamProviderPolicy`] snapshot crossing the FFI (wide balances split as `lo` / `hi` `u64` halves).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiStreamProviderPolicy {
    pub min_rate: u64,
    pub min_allocation_lo: u64,
    pub min_allocation_hi: u64,
    pub max_create_stream_deadline_delay: u64,
    pub vault_proof_max_response_bytes: u64,
}

/// Accepted / proposed [`StreamParams`] fields without heap indirection (`service_id` prefix + fixed buffer tail).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiStreamParams {
    pub rate: u64,
    pub allocation_lo: u64,
    pub allocation_hi: u64,
    pub create_stream_deadline: u64,
    pub service_id_len: u32,
    pub _padding: u32,
    pub service_id_bytes: [u8; 128],
}

/// Borrowed byte range supplied by the host (interpreted as UTF-8 for string fields).
///
/// Safety contract (matches [`borrow_input`]):
/// - When `len > 0`, `ptr` must reference `len` contiguous readable bytes for the duration of the call.
/// - When `len == 0`, `ptr` may be null or dangling (empty slice).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiByteSpan {
    pub ptr: *const u8,
    pub len: usize,
}

/// Decoded `VaultProof` fields (`owner_signature` included for verification helpers).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiDecodedVaultProof {
    pub vault_id: u64,
    pub provider_id: [u8; 32],
    pub owner_public_key: [u8; 32],
    pub owner_signature: [u8; 64],
}

/// Decoded protobuf `StreamProposal` mirrored for C hosts.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiDecodedStreamProposal {
    pub vault_proof: PaymentStreamsFfiDecodedVaultProof,
    pub params: PaymentStreamsFfiStreamParams,
    pub session_public_key: [u8; 32],
}

/// Decoded protobuf `StreamProof`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiDecodedStreamProof {
    pub stream_id: u64,
    pub signature: [u8; 64],
}

/// Store query inputs used to build the canonical eligibility payload (integration plan N8).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiCanonicalStoreQuery {
    pub request_id: PaymentStreamsFfiByteSpan,
    pub include_data: u8,
    pub has_pubsub_topic: u8,
    pub pubsub_topic: PaymentStreamsFfiByteSpan,
    pub content_topics: *const PaymentStreamsFfiByteSpan,
    pub content_topics_len: u32,
    pub has_start_time: u8,
    pub start_time: i64,
    pub has_end_time: u8,
    pub end_time: i64,
    pub message_hashes: *const u8,
    pub message_hashes_len: u32,
    pub has_pagination_cursor: u8,
    pub pagination_cursor: [u8; 32],
    pub pagination_forward: u8,
    pub has_pagination_limit: u8,
    pub pagination_limit: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiProposalCheckInputs {
    pub params: PaymentStreamsFfiStreamParams,
    pub policy: PaymentStreamsFfiStreamProviderPolicy,
    pub vault_holding_balance_lo: u64,
    pub vault_holding_balance_hi: u64,
    pub vault_total_allocated_lo: u64,
    pub vault_total_allocated_hi: u64,
    pub now: u64,
}

/// Pinned session terms surfaced on the wire as [`lez_payment_streams_core::AcceptedStreamTerms`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiAcceptedStreamTerms {
    pub params: PaymentStreamsFfiStreamParams,
    pub provider_id: [u8; 32],
    pub policy_at_acceptance: PaymentStreamsFfiStreamProviderPolicy,
}

/// [`lez_payment_streams_core::StreamFoldedAtTime`] mirrored for C callers.
///
/// Numeric fields split LEZ balances into deterministic little-endian `lo` / `hi` halves.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaymentStreamsFfiStreamFoldAtTime {
    pub folded_stream: PaymentStreamsFfiDecodedStreamConfig,
    pub accrued_lo: u64,
    pub accrued_hi: u64,
    pub unaccrued_lo: u64,
    pub unaccrued_hi: u64,
    pub as_of: u64,
}

/// Stable policy rejection codes for FFI consumers.
///
/// Values `0..=8` mirror [`lez_payment_streams_core::PolicyRejectReason`] today (`repr(u32)`).
/// `Unknown` (`9`) is reserved for forward compatibility when core adds `#[non_exhaustive]` variants
/// before this FFI crate’s rejection mapping catches up.
///
/// Hosts map these to Store-style eligibility buckets (see payment streams integration docs / LIP‑155):
/// most predicate outcomes map to `PARAMS_REJECTED`, `StreamNotActive` maps to `STREAM_NOT_ACTIVE`,
/// and proof-layer failures use `PROOF_INVALID`. Until a host defines finer rules, treat `Unknown` like `PARAMS_REJECTED`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PaymentStreamsFfiPolicyRejectReason {
    RateBelowPolicyMin = 0,
    AllocationBelowPolicyMin = 1,
    CreateStreamDeadlineInvalid = 2,
    UnallocatedInsufficient = 3,
    RateBelowAcceptedParams = 4,
    AllocationBelowAcceptedParams = 5,
    ProviderMismatch = 6,
    StreamNotActive = 7,
    ResponseTooLarge = 8,
    /// Core [`PolicyRejectReason`] variant not yet surfaced by this FFI layer.
    Unknown = 9,
}

const _: () = assert!(PAYMENT_STREAMS_FFI_MAX_SERVICE_ID_LEN == 128usize);

#[must_use]
fn map_decode_fault(err: DecodeFault) -> PaymentStreamsFfiStatus {
    match err {
        DecodeFault::Malformed => PaymentStreamsFfiStatus::Malformed,
        DecodeFault::BadVersion => PaymentStreamsFfiStatus::BadVersion,
    }
}

/// Split NSSA [`Balance`] (effectively `u128`) into low/high `u64` halves for portable C FFI
/// (`*_lo` / `*_hi` as `uint64_t`), avoiding non-standard 128-bit C types across targets.
#[must_use]
pub(crate) fn balance_pair(value: Balance) -> (u64, u64) {
    ((value & u128::from(u64::MAX)) as u64, (value >> 64) as u64)
}

pub(crate) fn program_id_from_le_bytes(bytes: &[u8]) -> Result<ProgramId, PaymentStreamsFfiStatus> {
    let b: &[u8; 32] = bytes
        .try_into()
        .map_err(|_| PaymentStreamsFfiStatus::Malformed)?;

    Ok(std::array::from_fn(|word_idx| {
        let offset = word_idx * 4;
        let mut word = [0_u8; 4];
        word.copy_from_slice(&(*b)[offset..offset + 4]);
        u32::from_le_bytes(word)
    }))
}

fn account_id_from_le_bytes(bytes: &[u8]) -> Result<AccountId, PaymentStreamsFfiStatus> {
    let chunk = <[u8; 32]>::try_from(bytes).map_err(|_| PaymentStreamsFfiStatus::Malformed)?;
    Ok(AccountId::new(chunk))
}

/// C ABI `stream_state` via [`StreamState`]'s `impl From<Self> for u8` in
/// `lez_payment_streams_core` (`#[repr(u8)]`, `#[borsh(use_discriminant = true)]`).
#[must_use]
pub(crate) fn stream_state_repr(state: StreamState) -> u8 {
    u8::from(state)
}

/// C ABI `privacy_tier` via [`VaultPrivacyTier`]'s `impl From<Self> for u8` in
/// `lez_payment_streams_core` (`#[repr(u8)]`, `#[borsh(use_discriminant = true)]`).
#[must_use]
fn privacy_tier_repr(tier: VaultPrivacyTier) -> u8 {
    u8::from(tier)
}

/// Borrows caller-provided input bytes without copying.
///
/// # Safety
///
/// When `len > 0`, `ptr` must be valid for `len` contiguous bytes of immutable reads for lifetime
/// `'a`, and nothing may mutate aliasing memory covering that slice for `'a`. When `len == 0`,
/// `ptr` may be dangling or null (empty slice returned).
pub(crate) unsafe fn borrow_input<'a>(
    ptr: *const u8,
    len: usize,
) -> Result<&'a [u8], PaymentStreamsFfiStatus> {
    if ptr.is_null() && len > 0 {
        return Err(PaymentStreamsFfiStatus::NullPointer);
    }

    Ok(if len == 0 {
        &[]
    } else {
        slice::from_raw_parts(ptr, len)
    })
}

/// Placeholder linkage smoke (may be removed once the FFI surface is fully wired).
#[no_mangle]
pub extern "C" fn payment_streams_ffi_ping() -> PaymentStreamsFfiStatus {
    PaymentStreamsFfiStatus::Success
}

/// Decode serialized `VaultConfig` bytes copied from sequencer account payload.
///
/// `vault_cfg_decoded` is Borsh + version-checked core state; writes the flattened `repr(C)` struct via
/// `ffi_out_decoded` / `ffi_out_decoded_mut`.
///
/// # Safety
///
/// - `(data_ptr, data_len)` form a readable range: when `data_len > 0`, `data_ptr` must be valid for
///   `data_len` bytes of immutable access until return; when `data_len == 0`, `data_ptr` may be null
///   or dangling.
/// - When `ffi_out_decoded` is non-null it must reference memory valid for a full overwrite of
///   [`PaymentStreamsFfiDecodedVaultConfig`] until this function returns.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_decode_vault_config_bytes(
    data_ptr: *const u8,
    data_len: usize,
    ffi_out_decoded: *mut PaymentStreamsFfiDecodedVaultConfig,
) -> PaymentStreamsFfiStatus {
    match borrow_input(data_ptr, data_len) {
        Err(err) => err,
        Ok(bytes) => match decode::decode_vault_config(bytes) {
            Err(err) => map_decode_fault(err),
            Ok(vault_cfg_decoded) => match ffi_out_decoded.as_mut() {
                None => PaymentStreamsFfiStatus::NullPointer,
                Some(ffi_out_decoded_mut) => {
                    let totals = balance_pair(vault_cfg_decoded.total_allocated);
                    ffi_out_decoded_mut.version = vault_cfg_decoded.version;
                    ffi_out_decoded_mut.privacy_tier =
                        privacy_tier_repr(vault_cfg_decoded.privacy_tier);
                    ffi_out_decoded_mut._padding = [0; 6];
                    ffi_out_decoded_mut.owner = *vault_cfg_decoded.owner.value();
                    ffi_out_decoded_mut.vault_id = vault_cfg_decoded.vault_id;
                    ffi_out_decoded_mut.next_stream_id = vault_cfg_decoded.next_stream_id;
                    ffi_out_decoded_mut.total_allocated_lo = totals.0;
                    ffi_out_decoded_mut.total_allocated_hi = totals.1;
                    PaymentStreamsFfiStatus::Success
                }
            },
        },
    }
}

/// Decode serialized `VaultHolding` payload.
///
/// `vault_holding_decoded` is Borsh + version-checked core state; fills `ffi_out_decoded*`.
///
/// # Safety
///
/// - `(data_ptr, data_len)` form a readable range: when `data_len > 0`, `data_ptr` must be valid for
///   `data_len` bytes of immutable access until return; when `data_len == 0`, `data_ptr` may be null
///   or dangling.
/// - When `ffi_out_decoded` is non-null it must reference memory valid for a full overwrite of
///   [`PaymentStreamsFfiDecodedVaultHolding`] until this function returns.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_decode_vault_holding_bytes(
    data_ptr: *const u8,
    data_len: usize,
    ffi_out_decoded: *mut PaymentStreamsFfiDecodedVaultHolding,
) -> PaymentStreamsFfiStatus {
    match borrow_input(data_ptr, data_len) {
        Err(err) => err,
        Ok(bytes) => match decode::decode_vault_holding(bytes) {
            Err(err) => map_decode_fault(err),
            Ok(vault_holding_decoded) => match ffi_out_decoded.as_mut() {
                None => PaymentStreamsFfiStatus::NullPointer,
                Some(ffi_out_decoded_mut) => {
                    ffi_out_decoded_mut.version = vault_holding_decoded.version;
                    ffi_out_decoded_mut._padding = [0; 7];
                    PaymentStreamsFfiStatus::Success
                }
            },
        },
    }
}

/// Decode serialized `StreamConfig` payload.
///
/// `stream_cfg_decoded` is Borsh + version-checked core state; fills `ffi_out_decoded*`.
///
/// # Safety
///
/// - `(data_ptr, data_len)` form a readable range: when `data_len > 0`, `data_ptr` must be valid for
///   `data_len` bytes of immutable access until return; when `data_len == 0`, `data_ptr` may be null
///   or dangling.
/// - When `ffi_out_decoded` is non-null it must reference memory valid for a full overwrite of
///   [`PaymentStreamsFfiDecodedStreamConfig`] until this function returns.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_decode_stream_config_bytes(
    data_ptr: *const u8,
    data_len: usize,
    ffi_out_decoded: *mut PaymentStreamsFfiDecodedStreamConfig,
) -> PaymentStreamsFfiStatus {
    match borrow_input(data_ptr, data_len) {
        Err(err) => err,
        Ok(bytes) => match decode::decode_stream_config(bytes) {
            Err(err) => map_decode_fault(err),
            Ok(stream_cfg_decoded) => match ffi_out_decoded.as_mut() {
                None => PaymentStreamsFfiStatus::NullPointer,
                Some(ffi_out_decoded_mut) => {
                    let alloc = balance_pair(stream_cfg_decoded.allocation);
                    let accrued_parts = balance_pair(stream_cfg_decoded.accrued);
                    ffi_out_decoded_mut.version = stream_cfg_decoded.version;
                    ffi_out_decoded_mut.stream_state = stream_state_repr(stream_cfg_decoded.state);
                    ffi_out_decoded_mut._padding = [0; 6];
                    ffi_out_decoded_mut.stream_id = stream_cfg_decoded.stream_id;
                    ffi_out_decoded_mut.provider = *stream_cfg_decoded.provider.value();
                    ffi_out_decoded_mut.rate_tokens_per_second = stream_cfg_decoded.rate;
                    ffi_out_decoded_mut.allocation_lo = alloc.0;
                    ffi_out_decoded_mut.allocation_hi = alloc.1;
                    ffi_out_decoded_mut.accrued_lo = accrued_parts.0;
                    ffi_out_decoded_mut.accrued_hi = accrued_parts.1;
                    ffi_out_decoded_mut.accrued_as_of = stream_cfg_decoded.accrued_as_of;
                    PaymentStreamsFfiStatus::Success
                }
            },
        },
    }
}

/// Decode serialized `ClockAccountData` (`block_id` + timestamp seconds).
///
/// `clock_decoded` is Borsh core payload; fills `ffi_out_decoded*`.
///
/// # Safety
///
/// - `(data_ptr, data_len)` form a readable range: when `data_len > 0`, `data_ptr` must be valid for
///   `data_len` bytes of immutable access until return; when `data_len == 0`, `data_ptr` may be null
///   or dangling.
/// - When `ffi_out_decoded` is non-null it must reference memory valid for a full overwrite of
///   [`PaymentStreamsFfiDecodedClock`] until this function returns.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_decode_clock_account_data_bytes(
    data_ptr: *const u8,
    data_len: usize,
    ffi_out_decoded: *mut PaymentStreamsFfiDecodedClock,
) -> PaymentStreamsFfiStatus {
    match borrow_input(data_ptr, data_len) {
        Err(err) => err,
        Ok(bytes) => match decode::decode_clock_account_data(bytes) {
            Err(err) => map_decode_fault(err),
            Ok(clock_decoded) => match ffi_out_decoded.as_mut() {
                None => PaymentStreamsFfiStatus::NullPointer,
                Some(ffi_out_decoded_mut) => {
                    ffi_out_decoded_mut.block_id = clock_decoded.block_id;
                    ffi_out_decoded_mut.timestamp = clock_decoded.timestamp;
                    PaymentStreamsFfiStatus::Success
                }
            },
        },
    }
}

/// Copy deterministic CLOCK literal account ids enforced by genesis.
///
/// # Safety
///
/// `out_account_id_bytes` must be non-null and valid for writes of exactly 32 bytes until return.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_fixed_clock_account_id(
    selector: ClockAccountChoice,
    out_account_id_bytes: *mut u8,
) -> PaymentStreamsFfiStatus {
    if out_account_id_bytes.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let id = match selector {
        ClockAccountChoice::Clock01 => CLOCK_01_PROGRAM_ACCOUNT_ID,
        ClockAccountChoice::Clock10 => CLOCK_10_PROGRAM_ACCOUNT_ID,
        ClockAccountChoice::Clock50 => CLOCK_50_PROGRAM_ACCOUNT_ID,
    };

    slice::from_raw_parts_mut(out_account_id_bytes, 32).copy_from_slice(id.value());

    PaymentStreamsFfiStatus::Success
}

/// Derive `(vault_config, vault_holding)` account ids for `(owner, vault_id)`.
///
/// # Safety
///
/// - `program_id_bytes` must address 32 contiguous bytes readable until return (NSSA [`ProgramId`]
///   wire encoding, eight little-endian `u32` words).
/// - `owner_account_id_bytes` must address 32 contiguous bytes readable until return.
/// - Both `out_vault_config_account_id_bytes` / `out_vault_holding_account_id_bytes` must be non-null,
///   distinct or non-overlapping writable regions spanning 32 bytes each until return (the function
///   writes independently to both).
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_derive_vault_account_ids(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    out_vault_config_account_id_bytes: *mut u8,
    out_vault_holding_account_id_bytes: *mut u8,
) -> PaymentStreamsFfiStatus {
    if out_vault_config_account_id_bytes.is_null() || out_vault_holding_account_id_bytes.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let program_slice = match borrow_input(program_id_bytes, 32) {
        Err(err) => return err,
        Ok(slice) => slice,
    };

    let owner_slice = match borrow_input(owner_account_id_bytes, 32) {
        Err(err) => return err,
        Ok(slice) => slice,
    };

    let program_id = match program_id_from_le_bytes(program_slice) {
        Err(err) => return err,
        Ok(pid) => pid,
    };

    let owner = match account_id_from_le_bytes(owner_slice) {
        Err(err) => return err,
        Ok(id) => id,
    };

    let (vault_acc, vault_holding_acc) = derive_vault_account_ids(&program_id, owner, vault_id);

    slice::from_raw_parts_mut(out_vault_config_account_id_bytes, 32)
        .copy_from_slice(vault_acc.value());
    slice::from_raw_parts_mut(out_vault_holding_account_id_bytes, 32)
        .copy_from_slice(vault_holding_acc.value());

    PaymentStreamsFfiStatus::Success
}

/// Derive `stream_config` account identifier for `(vault_cfg, stream_id)`.
///
/// # Safety
///
/// - `program_id_bytes` must address 32 contiguous bytes readable until return (NSSA [`ProgramId`]
///   wire encoding, eight little-endian `u32` words).
/// - `vault_config_account_id_bytes` must address 32 contiguous raw account id bytes readable until
///   return.
/// - `out_stream_config_account_id_bytes` must be non-null and writable for 32 bytes until return.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_derive_stream_config_account_id(
    program_id_bytes: *const u8,
    vault_config_account_id_bytes: *const u8,
    stream_id: u64,
    out_stream_config_account_id_bytes: *mut u8,
) -> PaymentStreamsFfiStatus {
    if out_stream_config_account_id_bytes.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let program_slice = match borrow_input(program_id_bytes, 32) {
        Err(err) => return err,
        Ok(slice) => slice,
    };

    let vault_slice = match borrow_input(vault_config_account_id_bytes, 32) {
        Err(err) => return err,
        Ok(slice) => slice,
    };

    let program_id = match program_id_from_le_bytes(program_slice) {
        Err(err) => return err,
        Ok(pid) => pid,
    };

    let vault_cfg = match account_id_from_le_bytes(vault_slice) {
        Err(err) => return err,
        Ok(acc) => acc,
    };

    let stream_acc = derive_stream_config_account_id(&program_id, vault_cfg, stream_id);

    slice::from_raw_parts_mut(out_stream_config_account_id_bytes, 32)
        .copy_from_slice(stream_acc.value());

    PaymentStreamsFfiStatus::Success
}

#[cfg(test)]
mod tests {
    use super::*;
    use lez_payment_streams_core::VaultPrivacyTier;
    use std::str::FromStr;

    /// Base58 fixtures from the scaffold localnet notes in this repo (same deploy program wire as
    /// [`doc_scaffold_program_id`], canonical owner, and PDAs for `vault_id` / `stream_id` below; NSSA
    /// program-id wire encoding matches the documented hex literal).
    const DOC_SCAFFOLD_OWNER_BASE58: &str = "8UUCxCrkZAiP8A6g6rQAVMmk6bVxfurKqYi8aFxfEZqf";
    const DOC_SCAFFOLD_VAULT_CONFIG_PDA_BASE58: &str =
        "EKnp4sr9HL1vxJX1vdBUf82v2xGoDJhPSreqQXRhYAUS";
    const DOC_SCAFFOLD_VAULT_HOLDING_PDA_BASE58: &str =
        "H1Py4DETrQARSuqH66nL47gRsonsrkLaKNJNCG9hRjnT";
    const DOC_SCAFFOLD_STREAM_CONFIG_PDA_BASE58: &str =
        "HnVEXVhzdzywjAb1JEPbMoCHZ6LXguhwBTkjAGubDri4";

    const DOC_SCAFFOLD_STREAM_ID: u64 = 0;
    const DOC_SCAFFOLD_VAULT_ID: u64 = 0;

    fn program_id_bytes(pid: &ProgramId) -> [u8; 32] {
        let mut raw = [0_u8; 32];
        for (idx, chunk) in raw.chunks_exact_mut(4).enumerate() {
            chunk.copy_from_slice(&pid[idx].to_le_bytes());
        }
        raw
    }

    /// Hex deploy id from the scaffold localnet notes, interpreted as NSSA `ProgramId` wire bytes
    /// (eight little-endian `u32` words, see `program_id_from_le_bytes`).
    fn doc_scaffold_program_id() -> ProgramId {
        let hex_literal = concat!(
            "0b9349a24ceccf031fd2e06af23722e086dd2",
            "a8fec388e4d179619045ffb377d",
        );
        let mut raw = [0_u8; 32];
        raw.copy_from_slice(&hex::decode(hex_literal).expect("doc scaffold program id hex parses"));
        program_id_from_le_bytes(raw.as_slice()).expect("program id wire decodes")
    }

    fn doc_scaffold_owner() -> AccountId {
        AccountId::from_str(DOC_SCAFFOLD_OWNER_BASE58).expect("doc scaffold owner base58 parses")
    }

    fn doc_scaffold_vault_config_pda_fixture() -> AccountId {
        AccountId::from_str(DOC_SCAFFOLD_VAULT_CONFIG_PDA_BASE58)
            .expect("doc scaffold vault config PDA base58 parses")
    }

    fn doc_scaffold_vault_holding_pda_fixture() -> AccountId {
        AccountId::from_str(DOC_SCAFFOLD_VAULT_HOLDING_PDA_BASE58)
            .expect("doc scaffold vault holding PDA base58 parses")
    }

    fn doc_scaffold_stream_config_pda_fixture() -> AccountId {
        AccountId::from_str(DOC_SCAFFOLD_STREAM_CONFIG_PDA_BASE58)
            .expect("doc scaffold stream config PDA base58 parses")
    }

    #[test]
    fn doc_scaffold_pdas_match_recorded_base58_literals() {
        let program_id = doc_scaffold_program_id();
        let owner = doc_scaffold_owner();

        let (vault_cfg, vault_holding) =
            derive_vault_account_ids(&program_id, owner, DOC_SCAFFOLD_VAULT_ID);

        assert_eq!(vault_cfg, doc_scaffold_vault_config_pda_fixture());
        assert_eq!(vault_holding, doc_scaffold_vault_holding_pda_fixture());

        let stream_cfg =
            derive_stream_config_account_id(&program_id, vault_cfg, DOC_SCAFFOLD_STREAM_ID);
        assert_eq!(stream_cfg, doc_scaffold_stream_config_pda_fixture());
    }

    #[test]
    fn doc_scaffold_vault_pdas_roundtrip_via_c_abi() {
        let program_id = doc_scaffold_program_id();
        let owner = doc_scaffold_owner();
        let program_wire = program_id_bytes(&program_id);

        let mut got_cfg = [0_u8; 32];
        let mut got_hold = [0_u8; 32];

        assert_eq!(
            unsafe {
                payment_streams_ffi_derive_vault_account_ids(
                    program_wire.as_ptr(),
                    owner.value().as_ptr(),
                    DOC_SCAFFOLD_VAULT_ID,
                    got_cfg.as_mut_ptr(),
                    got_hold.as_mut_ptr(),
                )
            },
            PaymentStreamsFfiStatus::Success,
        );

        assert_eq!(
            AccountId::new(got_cfg),
            doc_scaffold_vault_config_pda_fixture(),
        );
        assert_eq!(
            AccountId::new(got_hold),
            doc_scaffold_vault_holding_pda_fixture(),
        );
    }

    #[test]
    fn doc_scaffold_stream_pda_roundtrip_via_c_abi() {
        let program_id = doc_scaffold_program_id();
        let owner = doc_scaffold_owner();

        let (vault_cfg, _vault_holding) =
            derive_vault_account_ids(&program_id, owner, DOC_SCAFFOLD_VAULT_ID);

        let expect_stream =
            derive_stream_config_account_id(&program_id, vault_cfg, DOC_SCAFFOLD_STREAM_ID);

        let program_wire = program_id_bytes(&program_id);
        let mut got_stream = [0_u8; 32];

        assert_eq!(
            unsafe {
                payment_streams_ffi_derive_stream_config_account_id(
                    program_wire.as_ptr(),
                    vault_cfg.value().as_ptr(),
                    DOC_SCAFFOLD_STREAM_ID,
                    got_stream.as_mut_ptr(),
                )
            },
            PaymentStreamsFfiStatus::Success,
        );
        assert_eq!(AccountId::new(got_stream), expect_stream);
    }

    #[test]
    fn ffi_decode_vault_helper_roundtrip() {
        const ROUNDTRIP_OWNER_BYTE: u8 = 9;
        const ROUNDTRIP_VAULT_ID: u64 = 3;

        let cfg = VaultConfig::new(
            AccountId::new([ROUNDTRIP_OWNER_BYTE; 32]),
            ROUNDTRIP_VAULT_ID,
            None,
            None,
        );
        let serialized = borsh::to_vec(&cfg).unwrap();
        let mut ffi_out_decoded = PaymentStreamsFfiDecodedVaultConfig {
            version: 0,
            privacy_tier: u8::from(VaultPrivacyTier::Public),
            _padding: [0; 6],
            owner: [0; 32],
            vault_id: 0,
            next_stream_id: 0,
            total_allocated_lo: 0,
            total_allocated_hi: 0,
        };

        let status = unsafe {
            payment_streams_ffi_decode_vault_config_bytes(
                serialized.as_ptr(),
                serialized.len(),
                &mut ffi_out_decoded,
            )
        };
        assert_eq!(status, PaymentStreamsFfiStatus::Success);

        assert_eq!(ffi_out_decoded.owner, [ROUNDTRIP_OWNER_BYTE; 32]);
        assert_eq!(ffi_out_decoded.vault_id, cfg.vault_id);
        assert_eq!(ffi_out_decoded.version, cfg.version);
        assert_eq!(ffi_out_decoded.privacy_tier, u8::from(cfg.privacy_tier));
    }

    #[test]
    fn clock_fixture_matches_known_clock10_literal() {
        // Recorded together with clock account fixtures in product docs; must match core constant.
        const CLOCK10_DOCUMENTED_BASE58: &str = "4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWSs";

        let want = AccountId::from_str(CLOCK10_DOCUMENTED_BASE58)
            .expect("documented CLOCK10 literal base58 parses");
        assert_eq!(want, CLOCK_10_PROGRAM_ACCOUNT_ID);

        let mut rendered = [0_u8; 32];
        let status = unsafe {
            payment_streams_ffi_fixed_clock_account_id(
                ClockAccountChoice::Clock10,
                rendered.as_mut_ptr(),
            )
        };
        assert_eq!(status, PaymentStreamsFfiStatus::Success);
        assert_eq!(rendered, *CLOCK_10_PROGRAM_ACCOUNT_ID.value());
    }

    // Predicate vectors mirror `lez_payment_streams_core` policy unit tests (integration plan "Step 3a/3b").
    mod policy_ffi_abi_vectors {
        use super::*;
        use lez_payment_streams_core::{
            fold_stream, AcceptedStreamTerms, Balance, ErrorCode, PolicyRejectReason, StreamConfig,
            StreamId, StreamParams, StreamProviderPolicy, StreamState, Timestamp, TokensPerSecond,
            DEFAULT_VERSION,
        };

        fn marker_account(marker_byte: u8) -> AccountId {
            AccountId::new([marker_byte; 32])
        }

        fn stream_fixture_row(
            accrued: Balance,
            allocation: Balance,
            rate_tokens_per_second: TokensPerSecond,
            accrued_as_of_checkpoint: Timestamp,
            stream_state: StreamState,
            provider_account: AccountId,
        ) -> StreamConfig {
            StreamConfig {
                version: DEFAULT_VERSION,
                stream_id: StreamId::MIN,
                provider: provider_account,
                rate: rate_tokens_per_second,
                allocation,
                accrued,
                state: stream_state,
                accrued_as_of: accrued_as_of_checkpoint,
            }
        }

        fn decoded_stream_fixture_row(
            snapshot: &StreamConfig,
        ) -> PaymentStreamsFfiDecodedStreamConfig {
            let allocation_halves = balance_pair(snapshot.allocation);
            let accrued_halves = balance_pair(snapshot.accrued);
            PaymentStreamsFfiDecodedStreamConfig {
                version: snapshot.version,
                stream_state: stream_state_repr(snapshot.state),
                _padding: [0; 6],
                stream_id: snapshot.stream_id,
                provider: *snapshot.provider.value(),
                rate_tokens_per_second: snapshot.rate,
                allocation_lo: allocation_halves.0,
                allocation_hi: allocation_halves.1,
                accrued_lo: accrued_halves.0,
                accrued_hi: accrued_halves.1,
                accrued_as_of: snapshot.accrued_as_of,
            }
        }

        fn zero_fold_outcome_scratch() -> PaymentStreamsFfiStreamFoldAtTime {
            PaymentStreamsFfiStreamFoldAtTime {
                folded_stream: PaymentStreamsFfiDecodedStreamConfig {
                    version: 0,
                    stream_state: 0,
                    _padding: [0; 6],
                    stream_id: 0,
                    provider: [0; 32],
                    rate_tokens_per_second: 0,
                    allocation_lo: 0,
                    allocation_hi: 0,
                    accrued_lo: 0,
                    accrued_hi: 0,
                    accrued_as_of: 0,
                },
                accrued_lo: 0,
                accrued_hi: 0,
                unaccrued_lo: 0,
                unaccrued_hi: 0,
                as_of: 0,
            }
        }

        fn ffi_provider_policy_fixture(
            policy_snapshot: &StreamProviderPolicy,
        ) -> PaymentStreamsFfiStreamProviderPolicy {
            let min_alloc_halves = balance_pair(policy_snapshot.min_allocation);
            PaymentStreamsFfiStreamProviderPolicy {
                min_rate: policy_snapshot.min_rate,
                min_allocation_lo: min_alloc_halves.0,
                min_allocation_hi: min_alloc_halves.1,
                max_create_stream_deadline_delay: policy_snapshot.max_create_stream_deadline_delay,
                vault_proof_max_response_bytes: policy_snapshot.vault_proof_max_response_bytes,
            }
        }

        fn ffi_stream_params_fixture(
            params_snapshot: &StreamParams,
        ) -> PaymentStreamsFfiStreamParams {
            assert!(
                params_snapshot.service_id.len() <= PAYMENT_STREAMS_FFI_MAX_SERVICE_ID_LEN,
                "vectors obey the capped `service_id` length documented for callers before signing",
            );
            let allocation_halves = balance_pair(params_snapshot.allocation);
            let mut service_id_scratch = [0_u8; PAYMENT_STREAMS_FFI_MAX_SERVICE_ID_LEN];
            service_id_scratch[..params_snapshot.service_id.len()]
                .copy_from_slice(&params_snapshot.service_id);
            PaymentStreamsFfiStreamParams {
                rate: params_snapshot.rate,
                allocation_lo: allocation_halves.0,
                allocation_hi: allocation_halves.1,
                create_stream_deadline: params_snapshot.create_stream_deadline,
                service_id_len: params_snapshot.service_id.len() as u32,
                _padding: 0,
                service_id_bytes: service_id_scratch,
            }
        }

        #[test]
        fn policy_reason_discriminants_match_core_abi() {
            let alignment_pairs = [
                (
                    PolicyRejectReason::RateBelowPolicyMin,
                    PaymentStreamsFfiPolicyRejectReason::RateBelowPolicyMin,
                ),
                (
                    PolicyRejectReason::AllocationBelowPolicyMin,
                    PaymentStreamsFfiPolicyRejectReason::AllocationBelowPolicyMin,
                ),
                (
                    PolicyRejectReason::CreateStreamDeadlineInvalid,
                    PaymentStreamsFfiPolicyRejectReason::CreateStreamDeadlineInvalid,
                ),
                (
                    PolicyRejectReason::UnallocatedInsufficient,
                    PaymentStreamsFfiPolicyRejectReason::UnallocatedInsufficient,
                ),
                (
                    PolicyRejectReason::RateBelowAcceptedParams,
                    PaymentStreamsFfiPolicyRejectReason::RateBelowAcceptedParams,
                ),
                (
                    PolicyRejectReason::AllocationBelowAcceptedParams,
                    PaymentStreamsFfiPolicyRejectReason::AllocationBelowAcceptedParams,
                ),
                (
                    PolicyRejectReason::ProviderMismatch,
                    PaymentStreamsFfiPolicyRejectReason::ProviderMismatch,
                ),
                (
                    PolicyRejectReason::StreamNotActive,
                    PaymentStreamsFfiPolicyRejectReason::StreamNotActive,
                ),
                (
                    PolicyRejectReason::ResponseTooLarge,
                    PaymentStreamsFfiPolicyRejectReason::ResponseTooLarge,
                ),
            ];

            for (reason_from_core_row, ffi_reason_row) in alignment_pairs {
                assert_eq!(reason_from_core_row as u32, ffi_reason_row as u32);
            }

            assert_eq!(PaymentStreamsFfiPolicyRejectReason::Unknown as u32, 9);
        }

        #[test]
        fn ffi_fold_matches_core_vectors() {
            let provider_payee_binding = marker_account(9);
            let stream_snapshot = stream_fixture_row(
                0,
                1_000,
                10,
                100,
                StreamState::Active,
                provider_payee_binding,
            );
            let decoded_snapshot = decoded_stream_fixture_row(&stream_snapshot);
            let mut fold_scratch = zero_fold_outcome_scratch();
            let mut guest_error_slot = 0_u32;
            let guest_error_ptr = &mut guest_error_slot as *mut u32;

            assert_eq!(
                unsafe {
                    payment_streams_ffi_fold_stream(
                        &decoded_snapshot,
                        105,
                        &mut fold_scratch,
                        guest_error_ptr,
                    )
                },
                PaymentStreamsFfiStatus::Success,
            );

            let expected_fold_snapshot =
                fold_stream(&stream_snapshot, 105).expect("core vector stays valid");
            let ffi_accrued = Balance::from(fold_scratch.accrued_lo)
                | Balance::from(fold_scratch.accrued_hi) << 64;
            let ffi_unaccrued = Balance::from(fold_scratch.unaccrued_lo)
                | Balance::from(fold_scratch.unaccrued_hi) << 64;

            assert_eq!(ffi_accrued, expected_fold_snapshot.accrued);
            assert_eq!(ffi_unaccrued, expected_fold_snapshot.unaccrued);
            assert_eq!(fold_scratch.as_of, expected_fold_snapshot.as_of);
            assert_eq!(
                stream_state_repr(expected_fold_snapshot.stream_config.state),
                fold_scratch.folded_stream.stream_state,
            );
            assert_eq!(
                fold_scratch.folded_stream.accrued_as_of,
                expected_fold_snapshot.stream_config.accrued_as_of,
            );
        }

        #[test]
        fn ffi_fold_surfaces_time_regression_through_guest_slot() {
            let provider_payee_binding = marker_account(8);
            let stream_snapshot = stream_fixture_row(
                0,
                1_000,
                10,
                100,
                StreamState::Active,
                provider_payee_binding,
            );
            let decoded_snapshot = decoded_stream_fixture_row(&stream_snapshot);
            let mut fold_scratch = zero_fold_outcome_scratch();
            let mut guest_error_slot = 0_u32;
            let guest_error_ptr = &mut guest_error_slot as *mut u32;

            assert_eq!(
                unsafe {
                    payment_streams_ffi_fold_stream(
                        &decoded_snapshot,
                        /* folds before accrued_as_of */ 99,
                        &mut fold_scratch,
                        guest_error_ptr,
                    )
                },
                PaymentStreamsFfiStatus::StreamFoldFailed,
            );
            assert_eq!(guest_error_slot, ErrorCode::TimeRegression as u32);
        }

        #[test]
        fn ffi_proposal_matches_below_min_rate_vector() {
            let advertised_provider_policy_snapshot =
                StreamProviderPolicy::new(20, 500, 1_000, 65_536);
            let payer_proposal_terms = StreamParams::new(10, 600, 200, vec![]);
            let mut reject_slot = PaymentStreamsFfiPolicyRejectReason::RateBelowPolicyMin;
            let holding_balance_halves = balance_pair(10_000);
            let total_allocated_halves = balance_pair(100);
            let proposal_check_bundle = PaymentStreamsFfiProposalCheckInputs {
                params: ffi_stream_params_fixture(&payer_proposal_terms),
                policy: ffi_provider_policy_fixture(&advertised_provider_policy_snapshot),
                vault_holding_balance_lo: holding_balance_halves.0,
                vault_holding_balance_hi: holding_balance_halves.1,
                vault_total_allocated_lo: total_allocated_halves.0,
                vault_total_allocated_hi: total_allocated_halves.1,
                now: 100,
            };

            assert_eq!(
                unsafe {
                    payment_streams_ffi_proposal_satisfies_policy(
                        &proposal_check_bundle,
                        &mut reject_slot as *mut _,
                    )
                },
                PaymentStreamsFfiStatus::PolicyRejected,
            );

            assert_eq!(
                reject_slot,
                PaymentStreamsFfiPolicyRejectReason::RateBelowPolicyMin
            );
        }

        #[test]
        fn ffi_deadline_predicate_matches_overflow_pinned_curve() {
            let mut deadline_reject_scratch =
                PaymentStreamsFfiPolicyRejectReason::RateBelowPolicyMin;

            assert_eq!(
                unsafe {
                    payment_streams_ffi_create_stream_deadline_satisfies_policy_as_of(
                        Timestamp::MAX - 1,
                        Timestamp::MAX,
                        100,
                        &mut deadline_reject_scratch as *mut _,
                    )
                },
                PaymentStreamsFfiStatus::Success,
            );

            assert_eq!(
                unsafe {
                    payment_streams_ffi_create_stream_deadline_satisfies_policy_as_of(
                        /* not strictly in the future */ 100,
                        10,
                        100,
                        &mut deadline_reject_scratch as *mut _,
                    )
                },
                PaymentStreamsFfiStatus::PolicyRejected,
            );

            assert_eq!(
                deadline_reject_scratch,
                PaymentStreamsFfiPolicyRejectReason::CreateStreamDeadlineInvalid,
            );
        }

        #[test]
        fn ffi_stream_policy_matches_paused_rejection_vector() {
            let provider_payee_binding = marker_account(12);
            let accepted_params_snapshot = StreamParams::new(5, 100, 0, vec![]);
            let accepted_policy_snapshot = StreamProviderPolicy::new(1, 1, 1_000, 65_536);
            let accepted_terms_host_view = AcceptedStreamTerms {
                params: accepted_params_snapshot.clone(),
                provider_id: provider_payee_binding,
                policy_at_acceptance: accepted_policy_snapshot.clone(),
            };

            let accepted_terms_bundle = PaymentStreamsFfiAcceptedStreamTerms {
                params: ffi_stream_params_fixture(&accepted_terms_host_view.params),
                provider_id: *accepted_terms_host_view.provider_id.value(),
                policy_at_acceptance: ffi_provider_policy_fixture(
                    &accepted_terms_host_view.policy_at_acceptance,
                ),
            };

            let folded_paused_stream_snapshot = stream_fixture_row(
                100,
                100,
                10,
                10,
                StreamState::Paused,
                provider_payee_binding,
            );
            let decoded_paused_fold = decoded_stream_fixture_row(&folded_paused_stream_snapshot);
            let mut reject_slot = PaymentStreamsFfiPolicyRejectReason::RateBelowPolicyMin;

            assert_eq!(
                unsafe {
                    payment_streams_ffi_stream_satisfies_policy(
                        &decoded_paused_fold,
                        &accepted_terms_bundle,
                        &mut reject_slot as *mut _,
                    )
                },
                PaymentStreamsFfiStatus::PolicyRejected,
            );

            assert_eq!(
                reject_slot,
                PaymentStreamsFfiPolicyRejectReason::StreamNotActive
            );
        }

        #[test]
        fn ffi_response_predicate_matches_demo_cap_vector() {
            let capped_policy_snapshot = StreamProviderPolicy::new(1, 1, 1, 128);
            let ffi_policy_row = ffi_provider_policy_fixture(&capped_policy_snapshot);
            let mut reject_slot = PaymentStreamsFfiPolicyRejectReason::RateBelowPolicyMin;

            assert_eq!(
                unsafe {
                    payment_streams_ffi_response_within_policy(
                        129,
                        &ffi_policy_row,
                        &mut reject_slot as *mut _,
                    )
                },
                PaymentStreamsFfiStatus::PolicyRejected,
            );

            assert_eq!(
                reject_slot,
                PaymentStreamsFfiPolicyRejectReason::ResponseTooLarge
            );
        }

        #[test]
        fn ffi_new_stream_predicate_matches_below_allocation_vector() {
            let beneficiary_provider_binding = marker_account(4);
            let accepted_params_snapshot = StreamParams::new(50, 200, 0, vec![]);
            let weakened_chain_snapshot = stream_fixture_row(
                0,
                199,
                50,
                0,
                StreamState::Active,
                beneficiary_provider_binding,
            );
            let decoded_chain_snapshot = decoded_stream_fixture_row(&weakened_chain_snapshot);

            let mut reject_slot = PaymentStreamsFfiPolicyRejectReason::RateBelowPolicyMin;
            assert_eq!(
                unsafe {
                    payment_streams_ffi_new_stream_satisfies_proposal(
                        &decoded_chain_snapshot,
                        &ffi_stream_params_fixture(&accepted_params_snapshot),
                        beneficiary_provider_binding.value().as_ptr(),
                        &mut reject_slot as *mut _,
                    )
                },
                PaymentStreamsFfiStatus::PolicyRejected,
            );

            assert_eq!(
                reject_slot,
                PaymentStreamsFfiPolicyRejectReason::AllocationBelowAcceptedParams,
            );
        }
    }
}
