//! C ABI for payment-stream public instructions (LEZ / NSSA `InstructionData` bytes + account lists).
//!
//! ## Naming (core vs FFI)
//!
//! - Rust crate [`lez_payment_streams_core`](lez_payment_streams_core) exposes
//!   `*_instruction_accounts` helpers (nouns: the ordered account-id list for an instruction).
//! - This module uses `payment_streams_ffi_plan_*_instruction_accounts` (verb: compute that list
//!   for C hosts). Semantics match the SPEL guest account order.
//!
//! - `payment_streams_ffi_serialize_*_instruction` builds the instruction byte payload for wallet
//!   JSON `send_public_transaction` (`instruction` field), using the same encoding as
//!   `lee::program::Program::serialize_instruction` followed by little-endian `u32` expansion (see
//!   `lez_payment_streams_core::instruction_bytes_for_public_transaction`).
//!
//! ## Two-phase sizing
//!
//! All `serialize_*` and `plan_*` entry points use the same pattern as other payment-streams FFI:
//!
//! 1. Call with `out_ptr` / `accounts_hex_out` null and `out_cap` / `accounts_hex_out_cap` zero
//!    (values are ignored for the sizing pass). `out_len` / `accounts_hex_out_len` must be non-null.
//!    On [`PaymentStreamsFfiStatus::Success`], the length fields hold the required byte length.
//! 2. Allocate at least that many bytes, call again with a non-null output pointer and matching cap.
//!
//! If the output pointer is non-null and `out_cap` is too small, the call returns
//! [`PaymentStreamsFfiStatus::Malformed`].
//!
//! ## Account hex layout (`plan_*_instruction_accounts`)
//!
//! Each account occupies [`PAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN`] consecutive bytes: **lowercase**
//! ASCII hex (`[0-9a-f]`) for the raw 32-byte [`AccountId`], **no** `0x` prefix, **no** separators.
//! Example (64 characters): `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`.
//! This matches the `accounts` array style used by `logos-rln-module` when building
//! `send_public_transaction` JSON for `logos_execution_zone`.
//!
//! ## Error detail
//!
//! Instruction serialization failures ([`lee::error::LeeError`]) are reported as
//! [`PaymentStreamsFfiStatus::Malformed`]; the C ABI does not preserve nested error strings.
//! For instructions built from validated scalars on this path, serialization failures are
//! unexpected. [`VaultPrivacyTier`] values that are not `0` or `1` are rejected before serialize.
//!
//! ## Implementation note
//!
//! Planner wrappers repeat `read_program_id` / `read_account_id` explicitly so `cbindgen` output
//! stays one obvious symbol per instruction (macros would obscure the generated header).
//!
//! ## FFI contracts (`# Safety`)
//!
//! Unless stated otherwise on a symbol, every `unsafe extern "C"` entry point requires:
//!
//! - `out_len` / `accounts_hex_out_len`: non-null, writable `usize`.
//! - Any `*_bytes` / `program_id_bytes` / `*_account_id_bytes` input: when the parameter is
//!   required, the pointer must address **32** immutable bytes for the duration of the call (NSSA
//!   wire layout: eight little-endian `u32` words), matching [`crate::program_id_from_le_bytes`] /
//!   [`crate::account_id_from_le_bytes`].
//! - Output buffers: when non-null, the pointer must address `out_cap` / `accounts_hex_out_cap`
//!   contiguous writable bytes; only the prefix of length returned in `*out_len` may be written.

use core::slice;

use lez_payment_streams_core::{
    claim_instruction_accounts, close_stream_instruction_accounts,
    create_stream_instruction_accounts, deposit_instruction_accounts,
    initialize_vault_instruction_accounts, instruction_bytes_for_public_transaction,
    pause_stream_instruction_accounts, resume_stream_instruction_accounts,
    top_up_stream_instruction_accounts, withdraw_instruction_accounts, Instruction,
    VaultPrivacyTier,
};
use lee::program::Program;
use lee_core::account::{AccountId, Balance};
use lee_core::program::ProgramId;

use crate::policy_abi::balance_from_lo_hi;
use crate::{
    account_id_from_le_bytes, borrow_input, program_id_from_le_bytes, PaymentStreamsFfiStatus,
};

/// Byte length of one account entry in `plan_*_instruction_accounts` output.
///
/// Each account is encoded as **64** lowercase ASCII hex nibbles for the **32** raw id bytes
/// (`[0-9a-f]`, no `0x`, no separators), suitable for `send_public_transaction` JSON `accounts`
/// entries alongside `logos-rln-module`.
pub const PAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN: usize = 64;

fn write_account_id_hex_lower(
    id: &AccountId,
    out: &mut [u8; PAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN],
) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = id.value();
    for (idx, byte) in bytes.iter().enumerate() {
        out[idx * 2] = HEX[(byte >> 4) as usize];
        out[idx * 2 + 1] = HEX[(byte & 0xf) as usize];
    }
}

unsafe fn serialize_instruction_bytes(
    instruction: &Instruction,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    if out_len.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }
    *out_len = 0;

    let encoded = match instruction_bytes_for_public_transaction(instruction) {
        Ok(bytes) => bytes,
        // `LeeError` (e.g. `InstructionSerializationError`) is not threaded through the C ABI;
        // treat as `Malformed`. Should not trigger for `Instruction` values constructed here.
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
    };

    *out_len = encoded.len();
    if out_ptr.is_null() {
        return PaymentStreamsFfiStatus::Success;
    }
    if out_cap < encoded.len() {
        return PaymentStreamsFfiStatus::Malformed;
    }

    slice::from_raw_parts_mut(out_ptr, encoded.len()).copy_from_slice(&encoded);
    PaymentStreamsFfiStatus::Success
}

unsafe fn write_instruction_accounts_hex(
    accounts: &[AccountId],
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    if out_len.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }
    *out_len = 0;

    let required = match accounts
        .len()
        .checked_mul(PAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN)
    {
        Some(value) => value,
        None => return PaymentStreamsFfiStatus::Malformed,
    };

    if out_ptr.is_null() {
        *out_len = required;
        return PaymentStreamsFfiStatus::Success;
    }

    if out_cap < required {
        return PaymentStreamsFfiStatus::Malformed;
    }

    let out = slice::from_raw_parts_mut(out_ptr, required);
    for (idx, account) in accounts.iter().enumerate() {
        let mut scratch = [0_u8; PAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN];
        write_account_id_hex_lower(account, &mut scratch);
        let start = idx * PAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN;
        out[start..start + PAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN].copy_from_slice(&scratch);
    }

    *out_len = required;
    PaymentStreamsFfiStatus::Success
}

unsafe fn read_program_id(
    program_id_bytes: *const u8,
) -> Result<ProgramId, PaymentStreamsFfiStatus> {
    let slice = borrow_input(program_id_bytes, 32)?;
    program_id_from_le_bytes(slice)
}

unsafe fn read_account_id(account_bytes: *const u8) -> Result<AccountId, PaymentStreamsFfiStatus> {
    let slice = borrow_input(account_bytes, 32)?;
    account_id_from_le_bytes(slice)
}

type StreamOwnerPlannerFn = fn(&ProgramId, AccountId, u64, u64, AccountId) -> [AccountId; 5];

unsafe fn plan_stream_owner_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    stream_id: u64,
    clock_account_id_bytes: *const u8,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
    planner: StreamOwnerPlannerFn,
) -> PaymentStreamsFfiStatus {
    let program_id = match read_program_id(program_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let owner = match read_account_id(owner_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let clock = match read_account_id(clock_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };

    let accounts = planner(&program_id, owner, vault_id, stream_id, clock);
    write_instruction_accounts_hex(
        &accounts,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
    )
}

unsafe fn plan_stream_authority_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    stream_id: u64,
    authority_account_id_bytes: *const u8,
    clock_account_id_bytes: *const u8,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
    planner: fn(&ProgramId, AccountId, u64, u64, AccountId, AccountId) -> [AccountId; 6],
) -> PaymentStreamsFfiStatus {
    let program_id = match read_program_id(program_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let owner = match read_account_id(owner_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let authority = match read_account_id(authority_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let clock = match read_account_id(clock_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };

    let accounts = planner(&program_id, owner, vault_id, stream_id, authority, clock);
    write_instruction_accounts_hex(
        &accounts,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
    )
}

/// Writes the system authenticated-transfer [`ProgramId`] as **32** little-endian bytes (NSSA wire
/// layout: eight `u32` words), suitable for [`payment_streams_ffi_serialize_deposit_instruction`]'s
/// `authenticated_transfer_program_id_bytes` argument.
///
/// Equivalent to reading `lee::program::Program::authenticated_transfer_program().id()` on the Rust
/// side and flattening with `u32::to_le_bytes` in program-id order.
///
/// # Safety
///
/// `out_bytes` must be non-null and address **32** writable bytes.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_authenticated_transfer_program_id_bytes(
    out_bytes: *mut u8,
) -> PaymentStreamsFfiStatus {
    if out_bytes.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }
    let pid = Program::authenticated_transfer_program().id();
    let out = slice::from_raw_parts_mut(out_bytes, 32);
    for (idx, word) in pid.iter().enumerate() {
        out[idx * 4..idx * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    PaymentStreamsFfiStatus::Success
}

/// Serializes an `initialize_vault` instruction for wallet JSON `instruction` (LE instruction bytes).
///
/// `privacy_tier`: `0` = [`VaultPrivacyTier::Public`], `1` = [`VaultPrivacyTier::PseudonymousFunder`].
/// Any other value yields [`PaymentStreamsFfiStatus::Malformed`].
///
/// # Safety
///
/// See module-level FFI contracts (`out_ptr`, `out_cap`, `out_len`, two-phase sizing).
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_initialize_vault_instruction(
    vault_id: u64,
    privacy_tier: u8,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let privacy = match VaultPrivacyTier::try_from(privacy_tier) {
        Ok(value) => value,
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
    };
    let instruction = Instruction::initialize_vault(vault_id, privacy);
    serialize_instruction_bytes(&instruction, out_ptr, out_cap, out_len)
}

/// Plans ordered account ids for `initialize_vault` (hex layout per module docs).
///
/// # Safety
///
/// See module-level FFI contracts (`program_id_bytes`, `owner_account_id_bytes`, account hex output).
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_plan_initialize_vault_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let program_id = match read_program_id(program_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let owner = match read_account_id(owner_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };

    let accounts = initialize_vault_instruction_accounts(&program_id, owner, vault_id);
    write_instruction_accounts_hex(
        &accounts,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
    )
}

/// Serializes a `deposit` instruction. `amount_lo` / `amount_hi` form LEZ `Balance` (`u128`).
///
/// `authenticated_transfer_program_id_bytes`: NSSA wire `ProgramId` (**32** bytes). Hosts may fill this
/// with [`payment_streams_ffi_authenticated_transfer_program_id_bytes`] for standard deposits.
///
/// # Safety
///
/// See module-level FFI contracts (`authenticated_transfer_program_id_bytes`, `out_ptr`, `out_len`, two-phase sizing).
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_deposit_instruction(
    vault_id: u64,
    amount_lo: u64,
    amount_hi: u64,
    authenticated_transfer_program_id_bytes: *const u8,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let authenticated_transfer_program_id =
        match read_program_id(authenticated_transfer_program_id_bytes) {
            Ok(value) => value,
            Err(err) => return err,
        };
    let amount: Balance = balance_from_lo_hi(amount_lo, amount_hi);
    let instruction = Instruction::Deposit {
        vault_id,
        amount,
        authenticated_transfer_program_id,
    };
    serialize_instruction_bytes(&instruction, out_ptr, out_cap, out_len)
}

/// Plans ordered account ids for `deposit`.
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_plan_deposit_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let program_id = match read_program_id(program_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let owner = match read_account_id(owner_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };

    let accounts = deposit_instruction_accounts(&program_id, owner, vault_id);
    write_instruction_accounts_hex(
        &accounts,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
    )
}

/// Serializes a `withdraw` instruction (`amount_lo` / `amount_hi` → `Balance`).
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_withdraw_instruction(
    vault_id: u64,
    amount_lo: u64,
    amount_hi: u64,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let amount: Balance = balance_from_lo_hi(amount_lo, amount_hi);
    let instruction = Instruction::Withdraw { vault_id, amount };
    serialize_instruction_bytes(&instruction, out_ptr, out_cap, out_len)
}

/// Plans ordered account ids for `withdraw` (includes `withdraw_to_account_id_bytes`).
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_plan_withdraw_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    withdraw_to_account_id_bytes: *const u8,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let program_id = match read_program_id(program_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let owner = match read_account_id(owner_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let withdraw_to = match read_account_id(withdraw_to_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };

    let accounts = withdraw_instruction_accounts(&program_id, owner, vault_id, withdraw_to);
    write_instruction_accounts_hex(
        &accounts,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
    )
}

/// Serializes `create_stream` (`allocation_lo` / `allocation_hi` → `Balance`).
///
/// # Safety
///
/// See module-level FFI contracts (`provider_account_id_bytes`, output buffers).
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_create_stream_instruction(
    vault_id: u64,
    stream_id: u64,
    provider_account_id_bytes: *const u8,
    rate: u64,
    allocation_lo: u64,
    allocation_hi: u64,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let provider = match read_account_id(provider_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let allocation: Balance = balance_from_lo_hi(allocation_lo, allocation_hi);
    let instruction = Instruction::CreateStream {
        vault_id,
        stream_id,
        provider,
        rate,
        allocation,
    };
    serialize_instruction_bytes(&instruction, out_ptr, out_cap, out_len)
}

/// Plans ordered account ids for `create_stream` (vault owner stream layout + clock).
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_plan_create_stream_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    stream_id: u64,
    clock_account_id_bytes: *const u8,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let program_id = match read_program_id(program_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let owner = match read_account_id(owner_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };
    let clock = match read_account_id(clock_account_id_bytes) {
        Ok(value) => value,
        Err(err) => return err,
    };

    let accounts =
        create_stream_instruction_accounts(&program_id, owner, vault_id, stream_id, clock);
    write_instruction_accounts_hex(
        &accounts,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
    )
}

/// Serializes `pause_stream`.
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_pause_stream_instruction(
    vault_id: u64,
    stream_id: u64,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let instruction = Instruction::PauseStream {
        vault_id,
        stream_id,
    };
    serialize_instruction_bytes(&instruction, out_ptr, out_cap, out_len)
}

/// Plans ordered account ids for `pause_stream`.
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_plan_pause_stream_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    stream_id: u64,
    clock_account_id_bytes: *const u8,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    plan_stream_owner_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
        pause_stream_instruction_accounts,
    )
}

/// Serializes `resume_stream`.
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_resume_stream_instruction(
    vault_id: u64,
    stream_id: u64,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let instruction = Instruction::ResumeStream {
        vault_id,
        stream_id,
    };
    serialize_instruction_bytes(&instruction, out_ptr, out_cap, out_len)
}

/// Plans ordered account ids for `resume_stream`.
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_plan_resume_stream_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    stream_id: u64,
    clock_account_id_bytes: *const u8,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    plan_stream_owner_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
        resume_stream_instruction_accounts,
    )
}

/// Serializes `top_up_stream` (`vault_total_allocated_increase_*` → `Balance`).
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_top_up_stream_instruction(
    vault_id: u64,
    stream_id: u64,
    vault_total_allocated_increase_lo: u64,
    vault_total_allocated_increase_hi: u64,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let vault_total_allocated_increase: Balance = balance_from_lo_hi(
        vault_total_allocated_increase_lo,
        vault_total_allocated_increase_hi,
    );
    let instruction = Instruction::TopUpStream {
        vault_id,
        stream_id,
        vault_total_allocated_increase,
    };
    serialize_instruction_bytes(&instruction, out_ptr, out_cap, out_len)
}

/// Plans ordered account ids for `top_up_stream`.
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_plan_top_up_stream_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    stream_id: u64,
    clock_account_id_bytes: *const u8,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    plan_stream_owner_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
        top_up_stream_instruction_accounts,
    )
}

/// Serializes `close_stream`.
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_close_stream_instruction(
    vault_id: u64,
    stream_id: u64,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let instruction = Instruction::CloseStream {
        vault_id,
        stream_id,
    };
    serialize_instruction_bytes(&instruction, out_ptr, out_cap, out_len)
}

/// Plans ordered account ids for `close_stream` (`authority_account_id_bytes` signs).
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_plan_close_stream_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    stream_id: u64,
    authority_account_id_bytes: *const u8,
    clock_account_id_bytes: *const u8,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    plan_stream_authority_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        authority_account_id_bytes,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
        close_stream_instruction_accounts,
    )
}

/// Serializes `claim`.
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_claim_instruction(
    vault_id: u64,
    stream_id: u64,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let instruction = Instruction::Claim {
        vault_id,
        stream_id,
    };
    serialize_instruction_bytes(&instruction, out_ptr, out_cap, out_len)
}

/// Plans ordered account ids for `claim` (`provider_account_id_bytes` signs).
///
/// # Safety
///
/// See module-level FFI contracts.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_plan_claim_instruction_accounts(
    program_id_bytes: *const u8,
    owner_account_id_bytes: *const u8,
    vault_id: u64,
    stream_id: u64,
    provider_account_id_bytes: *const u8,
    clock_account_id_bytes: *const u8,
    accounts_hex_out: *mut u8,
    accounts_hex_out_cap: usize,
    accounts_hex_out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    plan_stream_authority_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        provider_account_id_bytes,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len,
        claim_instruction_accounts,
    )
}

#[cfg(test)]
mod tests {
    use lez_payment_streams_core::{
        initialize_vault_instruction_accounts, instruction_try_from_instruction_words,
        instruction_words_from_bytes_le, Instruction, VaultPrivacyTier,
    };
    use lee::program::Program;
    use lee_core::account::AccountId;

    #[test]
    fn authenticated_transfer_program_id_bytes_helper_matches_host_id() {
        let mut buf = [0_u8; 32];
        let status = unsafe {
            super::payment_streams_ffi_authenticated_transfer_program_id_bytes(buf.as_mut_ptr())
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::Success);
        let expected_pid = Program::authenticated_transfer_program().id();
        let mut expected = [0_u8; 32];
        for (idx, word) in expected_pid.iter().enumerate() {
            expected[idx * 4..idx * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        assert_eq!(buf, expected);
    }

    #[test]
    fn invalid_privacy_tier_returns_malformed() {
        let mut out_len = 0_usize;
        let status = unsafe {
            super::payment_streams_ffi_serialize_initialize_vault_instruction(
                1,
                0xFF,
                std::ptr::null_mut(),
                0,
                &mut out_len,
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::Malformed);
    }

    #[test]
    fn null_out_len_returns_null_pointer() {
        let status = unsafe {
            super::payment_streams_ffi_serialize_initialize_vault_instruction(
                1,
                0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::NullPointer);
    }

    #[test]
    fn plan_accounts_null_len_returns_null_pointer() {
        let program_id = [0_u8; 32];
        let owner = [1_u8; 32];
        let status = unsafe {
            super::payment_streams_ffi_plan_initialize_vault_instruction_accounts(
                program_id.as_ptr(),
                owner.as_ptr(),
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::NullPointer);
    }

    #[test]
    fn create_stream_serialize_round_trip() {
        let provider = AccountId::new([3_u8; 32]);
        let mut out_len = 0_usize;
        let status = unsafe {
            super::payment_streams_ffi_serialize_create_stream_instruction(
                10,
                11,
                provider.value().as_ptr(),
                5,
                100,
                0,
                std::ptr::null_mut(),
                0,
                &mut out_len,
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::Success);
        assert!(out_len > 0);

        let mut buf = vec![0_u8; out_len];
        let status = unsafe {
            super::payment_streams_ffi_serialize_create_stream_instruction(
                10,
                11,
                provider.value().as_ptr(),
                5,
                100,
                0,
                buf.as_mut_ptr(),
                buf.len(),
                &mut out_len,
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::Success);

        let words = instruction_words_from_bytes_le(&buf).expect("words");
        let decoded = instruction_try_from_instruction_words(&words).expect("decode");
        assert_eq!(
            decoded,
            Instruction::CreateStream {
                vault_id: 10,
                stream_id: 11,
                provider,
                rate: 5,
                allocation: 100,
            }
        );
    }

    #[test]
    fn plan_initialize_hex_matches_core_order() {
        let program_words = [0x01020304_u32, 0, 0, 0, 0, 0, 0, 0];
        let mut program_bytes = [0_u8; 32];
        for (idx, word) in program_words.iter().enumerate() {
            program_bytes[idx * 4..idx * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        let program_id = crate::program_id_from_le_bytes(&program_bytes).expect("program id");
        let owner = AccountId::new([9_u8; 32]);
        let vault_id = 42_u64;

        let expected = initialize_vault_instruction_accounts(&program_id, owner, vault_id);
        let mut required = 0_usize;
        let status = unsafe {
            super::payment_streams_ffi_plan_initialize_vault_instruction_accounts(
                program_bytes.as_ptr(),
                owner.value().as_ptr(),
                vault_id,
                std::ptr::null_mut(),
                0,
                &mut required,
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::Success);
        assert_eq!(
            required,
            expected.len() * super::PAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN
        );

        let mut hex_buf = vec![0_u8; required];
        let mut written = 0_usize;
        let status = unsafe {
            super::payment_streams_ffi_plan_initialize_vault_instruction_accounts(
                program_bytes.as_ptr(),
                owner.value().as_ptr(),
                vault_id,
                hex_buf.as_mut_ptr(),
                hex_buf.len(),
                &mut written,
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::Success);
        assert_eq!(written, required);

        for (idx, account) in expected.iter().enumerate() {
            let chunk = &hex_buf[idx * super::PAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN
                ..(idx + 1) * super::PAYMENT_STREAMS_FFI_ACCOUNT_ID_HEX_LEN];
            let decoded = hex::decode(chunk).expect("hex decode");
            assert_eq!(decoded.as_slice(), account.value().as_slice());
        }
    }

    #[test]
    fn ffi_sizing_matches_round_trip_for_initialize_vault() {
        let mut out_len = 0_usize;
        let status = unsafe {
            super::payment_streams_ffi_serialize_initialize_vault_instruction(
                9,
                0,
                std::ptr::null_mut(),
                0,
                &mut out_len,
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::Success);
        assert!(out_len > 0);

        let mut buf = vec![0_u8; out_len];
        let status = unsafe {
            super::payment_streams_ffi_serialize_initialize_vault_instruction(
                9,
                0,
                buf.as_mut_ptr(),
                buf.len(),
                &mut out_len,
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::Success);
        assert_eq!(out_len, buf.len());

        let words = instruction_words_from_bytes_le(&buf).expect("words");
        let ix = instruction_try_from_instruction_words(&words).expect("decode");
        assert_eq!(
            ix,
            Instruction::initialize_vault(9, VaultPrivacyTier::Public)
        );
    }

    #[test]
    fn deposit_serialize_round_trip_includes_authenticated_transfer_program_id() {
        let transfer_pid = Program::authenticated_transfer_program().id();
        let mut pid_bytes = [0_u8; 32];
        for (idx, word) in transfer_pid.iter().enumerate() {
            pid_bytes[idx * 4..idx * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }

        let mut out_len = 0_usize;
        let status = unsafe {
            super::payment_streams_ffi_serialize_deposit_instruction(
                2,
                8,
                0,
                pid_bytes.as_ptr(),
                std::ptr::null_mut(),
                0,
                &mut out_len,
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::Success);

        let mut buf = vec![0_u8; out_len];
        let status = unsafe {
            super::payment_streams_ffi_serialize_deposit_instruction(
                2,
                8,
                0,
                pid_bytes.as_ptr(),
                buf.as_mut_ptr(),
                buf.len(),
                &mut out_len,
            )
        };
        assert_eq!(status, super::PaymentStreamsFfiStatus::Success);

        let words = instruction_words_from_bytes_le(&buf).expect("words");
        let decoded = instruction_try_from_instruction_words(&words).expect("decode");
        assert_eq!(
            decoded,
            Instruction::Deposit {
                vault_id: 2,
                amount: 8,
                authenticated_transfer_program_id: transfer_pid,
            }
        );
    }
}
