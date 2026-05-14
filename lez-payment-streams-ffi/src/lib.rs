//! C ABI for LEZ payment streams (LIP-155).

mod decode;

pub use lez_payment_streams_core::{
    derive_stream_config_account_id, derive_vault_account_ids, VaultConfig,
    CLOCK_01_PROGRAM_ACCOUNT_ID, CLOCK_10_PROGRAM_ACCOUNT_ID, CLOCK_50_PROGRAM_ACCOUNT_ID,
};
pub use nssa_core::account::AccountId;
pub use nssa_core::program::ProgramId;

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
    /// Malformed/unusable inputs (truncated payloads, unexpected wire shape, invalid fixed sizes).
    Malformed = 2,
    BadVersion = 3,
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

#[must_use]
fn map_decode_fault(err: DecodeFault) -> PaymentStreamsFfiStatus {
    match err {
        DecodeFault::Malformed => PaymentStreamsFfiStatus::Malformed,
        DecodeFault::BadVersion => PaymentStreamsFfiStatus::BadVersion,
    }
}

/// Split NSSA [`Balance`] (effectively `u128`) into low/high limbs for portable C FFI
/// (`*_lo`/`*_hi` as `uint64_t`), avoiding non-standard 128-bit C types across targets.
#[must_use]
fn balance_pair(value: Balance) -> (u64, u64) {
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
fn stream_state_repr(state: StreamState) -> u8 {
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
unsafe fn borrow_input<'a>(
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

/// Placeholder linkage smoke exported in Step 1 (can be removed in later steps).
#[no_mangle]
pub extern "C" fn payment_streams_ffi_ping() -> PaymentStreamsFfiStatus {
    PaymentStreamsFfiStatus::Success
}

/// Decode serialized `VaultConfig` bytes copied from sequencer account payload.
///
/// `vault_cfg_decoded` is Borsh + version-checked core state; writes the flattened FFI view via
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

    /// Base58 fixtures recorded in `docs/step1-findings-scaffold-rpc.md`: same deploy program wire as
    /// [`doc_scaffold_program_id`], doc owner, and PDAs for `vault_id` / `stream_id` below (canonical
    /// NSSA program-id wire encoding for the hex in that doc).
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

    /// Hex from `docs/step1-findings-scaffold-rpc.md` deploy output, interpreted as NSSA
    /// `ProgramId` wire bytes (eight little-endian `u32` words, see `program_id_from_le_bytes`).
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
}
