//! C ABI wrappers for payment stream folding and [`StreamProviderPolicy`] predicates (LIP-155).
//!
//! Keeps arithmetic in [`lez_payment_streams_core`]; this module only reshapes structs and forwards.

use lez_payment_streams_core::{
    create_stream_deadline_satisfies_policy_as_of, fold_stream, new_stream_satisfies_proposal,
    proposal_satisfies_policy, response_within_policy, stream_satisfies_policy,
    AcceptedStreamTerms, ErrorCode, PolicyRejectReason, ProposalCheckInputs, StreamConfig,
    StreamParams, StreamProviderPolicy, StreamState,
};
use nssa_core::account::{AccountId, Balance};

use crate::stream_state_repr;
use crate::{
    PaymentStreamsFfiAcceptedStreamTerms, PaymentStreamsFfiDecodedStreamConfig,
    PaymentStreamsFfiPolicyRejectReason, PaymentStreamsFfiProposalCheckInputs,
    PaymentStreamsFfiStatus, PaymentStreamsFfiStreamFoldAtTime, PaymentStreamsFfiStreamParams,
    PaymentStreamsFfiStreamProviderPolicy, PAYMENT_STREAMS_FFI_MAX_SERVICE_ID_LEN,
};

/// Recombine a NSSA [`Balance`] (`u128`) from its little-endian `lo` / `hi` `u64` halves.
#[must_use]
fn balance_from_lo_hi(lo: u64, hi: u64) -> Balance {
    Balance::from(lo) | (Balance::from(hi) << 64)
}

#[must_use]
fn guest_error_repr(code: ErrorCode) -> u32 {
    code as u32
}

#[must_use]
fn map_policy_rejection(reason: PolicyRejectReason) -> PaymentStreamsFfiPolicyRejectReason {
    match reason {
        PolicyRejectReason::RateBelowPolicyMin => {
            PaymentStreamsFfiPolicyRejectReason::RateBelowPolicyMin
        }
        PolicyRejectReason::AllocationBelowPolicyMin => {
            PaymentStreamsFfiPolicyRejectReason::AllocationBelowPolicyMin
        }
        PolicyRejectReason::CreateStreamDeadlineInvalid => {
            PaymentStreamsFfiPolicyRejectReason::CreateStreamDeadlineInvalid
        }
        PolicyRejectReason::UnallocatedInsufficient => {
            PaymentStreamsFfiPolicyRejectReason::UnallocatedInsufficient
        }
        PolicyRejectReason::RateBelowAcceptedParams => {
            PaymentStreamsFfiPolicyRejectReason::RateBelowAcceptedParams
        }
        PolicyRejectReason::AllocationBelowAcceptedParams => {
            PaymentStreamsFfiPolicyRejectReason::AllocationBelowAcceptedParams
        }
        PolicyRejectReason::ProviderMismatch => {
            PaymentStreamsFfiPolicyRejectReason::ProviderMismatch
        }
        PolicyRejectReason::StreamNotActive => PaymentStreamsFfiPolicyRejectReason::StreamNotActive,
        PolicyRejectReason::ResponseTooLarge => {
            PaymentStreamsFfiPolicyRejectReason::ResponseTooLarge
        }
        // Forward-compatible path for hypothetical future `#[non_exhaustive]` variants shipped ahead of FFI updates.
        _ => PaymentStreamsFfiPolicyRejectReason::Unknown,
    }
}

#[must_use]
fn stream_provider_policy_from_ffi(
    ffi_policy: &PaymentStreamsFfiStreamProviderPolicy,
) -> StreamProviderPolicy {
    StreamProviderPolicy::new(
        ffi_policy.min_rate,
        balance_from_lo_hi(ffi_policy.min_allocation_lo, ffi_policy.min_allocation_hi),
        ffi_policy.max_create_stream_deadline_delay,
        ffi_policy.vault_proof_max_response_bytes,
    )
}

#[must_use]
fn stream_params_from_ffi(
    params: &PaymentStreamsFfiStreamParams,
) -> Result<StreamParams, PaymentStreamsFfiStatus> {
    let sid_len_usize = params.service_id_len as usize;
    if sid_len_usize > PAYMENT_STREAMS_FFI_MAX_SERVICE_ID_LEN {
        return Err(PaymentStreamsFfiStatus::Malformed);
    }

    Ok(StreamParams::new(
        params.rate,
        balance_from_lo_hi(params.allocation_lo, params.allocation_hi),
        params.create_stream_deadline,
        params.service_id_bytes[..sid_len_usize].to_vec(),
    ))
}

/// Map [`PaymentStreamsFfiDecodedStreamConfig`] (`payment_streams_ffi_decode_stream_config_bytes` layout) into [`StreamConfig`].
#[must_use]
fn stream_config_from_ffi(
    decoded: &PaymentStreamsFfiDecodedStreamConfig,
) -> Result<StreamConfig, PaymentStreamsFfiStatus> {
    let state = match decoded.stream_state {
        0 => StreamState::Active,
        1 => StreamState::Paused,
        2 => StreamState::Closed,
        _ => return Err(PaymentStreamsFfiStatus::Malformed),
    };

    Ok(StreamConfig {
        version: decoded.version,
        stream_id: decoded.stream_id,
        provider: AccountId::new(decoded.provider),
        rate: decoded.rate_tokens_per_second,
        allocation: balance_from_lo_hi(decoded.allocation_lo, decoded.allocation_hi),
        accrued: balance_from_lo_hi(decoded.accrued_lo, decoded.accrued_hi),
        state,
        accrued_as_of: decoded.accrued_as_of,
    })
}

fn fill_decoded_stream_config(
    stream: &StreamConfig,
    ffi_out_decoded_stream: &mut PaymentStreamsFfiDecodedStreamConfig,
) {
    let allocation_parts = crate::balance_pair(stream.allocation);
    let accrued_parts = crate::balance_pair(stream.accrued);
    ffi_out_decoded_stream.version = stream.version;
    ffi_out_decoded_stream.stream_state = stream_state_repr(stream.state);
    ffi_out_decoded_stream._padding = [0; 6];
    ffi_out_decoded_stream.stream_id = stream.stream_id;
    ffi_out_decoded_stream.provider = *stream.provider.value();
    ffi_out_decoded_stream.rate_tokens_per_second = stream.rate;
    ffi_out_decoded_stream.allocation_lo = allocation_parts.0;
    ffi_out_decoded_stream.allocation_hi = allocation_parts.1;
    ffi_out_decoded_stream.accrued_lo = accrued_parts.0;
    ffi_out_decoded_stream.accrued_hi = accrued_parts.1;
    ffi_out_decoded_stream.accrued_as_of = stream.accrued_as_of;
}

#[must_use]
fn accepted_terms_from_ffi(
    ffi_accepted_terms: &PaymentStreamsFfiAcceptedStreamTerms,
) -> Result<AcceptedStreamTerms, PaymentStreamsFfiStatus> {
    Ok(AcceptedStreamTerms {
        params: stream_params_from_ffi(&ffi_accepted_terms.params)?,
        provider_id: AccountId::new(ffi_accepted_terms.provider_id),
        policy_at_acceptance: stream_provider_policy_from_ffi(
            &ffi_accepted_terms.policy_at_acceptance,
        ),
    })
}

unsafe fn proposal_provider_binding_from_ffi_bytes(
    proposal_provider_id_bytes: *const u8,
) -> Result<AccountId, PaymentStreamsFfiStatus> {
    let proposal_provider_raw = crate::borrow_input(proposal_provider_id_bytes, 32)?;
    let provider_words = <[u8; 32]>::try_from(proposal_provider_raw)
        .map_err(|_| PaymentStreamsFfiStatus::Malformed)?;
    Ok(AccountId::new(provider_words))
}

/// Fold lazy accrual from decoded `StreamConfig` data carried as [`PaymentStreamsFfiDecodedStreamConfig`]
/// (the struct produced by [`payment_streams_ffi_decode_stream_config_bytes`]).
///
/// On [`PaymentStreamsFfiStatus::StreamFoldFailed`], writes optional precise context to
/// `guest_error_out` when non-null using stable [`lez_payment_streams_core::ErrorCode`] `repr(u32)` values (`6001+`).
///
/// # Safety
///
/// - `ffi_decoded_stream`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiDecodedStreamConfig`].
/// - `ffi_out_fold`: non-null, aligned pointer valid for writable access spanning one [`PaymentStreamsFfiStreamFoldAtTime`].
/// - `guest_error_out`: either null or a non-null, aligned pointer writable for exactly one `uint32_t`.
/// - Required null pointers return [`PaymentStreamsFfiStatus::NullPointer`] instead of touching outputs.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_fold_stream(
    ffi_decoded_stream: *const PaymentStreamsFfiDecodedStreamConfig,
    as_of: u64,
    ffi_out_fold: *mut PaymentStreamsFfiStreamFoldAtTime,
    guest_error_out: *mut u32,
) -> PaymentStreamsFfiStatus {
    if ffi_decoded_stream.is_null() || ffi_out_fold.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    match stream_config_from_ffi(&*ffi_decoded_stream) {
        Err(err_status) => err_status,
        Ok(stream_config_snapshot) => match fold_stream(&stream_config_snapshot, as_of) {
            Ok(stream_fold_snapshot) => {
                let accrued_parts = crate::balance_pair(stream_fold_snapshot.accrued);
                let unaccrued_parts = crate::balance_pair(stream_fold_snapshot.unaccrued);
                let ffi_out_fold_mut = &mut *ffi_out_fold;
                fill_decoded_stream_config(
                    &stream_fold_snapshot.stream_config,
                    &mut ffi_out_fold_mut.folded_stream,
                );
                ffi_out_fold_mut.accrued_lo = accrued_parts.0;
                ffi_out_fold_mut.accrued_hi = accrued_parts.1;
                ffi_out_fold_mut.unaccrued_lo = unaccrued_parts.0;
                ffi_out_fold_mut.unaccrued_hi = unaccrued_parts.1;
                ffi_out_fold_mut.as_of = stream_fold_snapshot.as_of;
                PaymentStreamsFfiStatus::Success
            }
            Err(guest_err) => {
                // Fine-grained context for C callers travels through optional `guest_error_out`; [`PaymentStreamsFfiStatus`] stays coarse.
                if !guest_error_out.is_null() {
                    *guest_error_out = guest_error_repr(guest_err);
                }
                PaymentStreamsFfiStatus::StreamFoldFailed
            }
        },
    }
}

/// Proposal-phase policy gate (runs on payer + provider before signing).
///
/// On [`PaymentStreamsFfiStatus::PolicyRejected`], `ffi_out_policy_reject` carries a
/// [`crate::PaymentStreamsFfiPolicyRejectReason`] code (`0..=8` mirrors core; `Unknown` covers
/// future [`lez_payment_streams_core::PolicyRejectReason`] variants not yet mapped explicitly).
///
/// # Safety
///
/// - `ffi_inputs`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiProposalCheckInputs`].
/// - `ffi_out_policy_reject`: non-null, aligned pointer writable for exactly one [`PaymentStreamsFfiPolicyRejectReason`].
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_proposal_satisfies_policy(
    ffi_inputs: *const PaymentStreamsFfiProposalCheckInputs,
    ffi_out_policy_reject: *mut PaymentStreamsFfiPolicyRejectReason,
) -> PaymentStreamsFfiStatus {
    if ffi_inputs.is_null() || ffi_out_policy_reject.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let ffi_snapshot = &*ffi_inputs;

    let parsed_params = match stream_params_from_ffi(&ffi_snapshot.params) {
        Ok(params) => params,
        Err(err) => return err,
    };
    let parsed_policy = stream_provider_policy_from_ffi(&ffi_snapshot.policy);

    let check_inputs_snapshot = ProposalCheckInputs::new(
        &parsed_params,
        &parsed_policy,
        balance_from_lo_hi(
            ffi_snapshot.vault_holding_balance_lo,
            ffi_snapshot.vault_holding_balance_hi,
        ),
        balance_from_lo_hi(
            ffi_snapshot.vault_total_allocated_lo,
            ffi_snapshot.vault_total_allocated_hi,
        ),
        ffi_snapshot.now,
    );

    match proposal_satisfies_policy(&check_inputs_snapshot) {
        Ok(()) => PaymentStreamsFfiStatus::Success,
        Err(reason) => {
            *ffi_out_policy_reject = map_policy_rejection(reason);
            PaymentStreamsFfiStatus::PolicyRejected
        }
    }
}

/// Deadline-only predicate extracted from proposal checks (`create_stream_deadline` clock band).
///
/// # Safety
///
/// - `ffi_out_policy_reject`: non-null, aligned pointer writable for exactly one [`PaymentStreamsFfiPolicyRejectReason`].
/// - A null `ffi_out_policy_reject` returns [`PaymentStreamsFfiStatus::NullPointer`] instead of touching the slot.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_create_stream_deadline_satisfies_policy_as_of(
    params_create_stream_deadline: u64,
    policy_max_create_stream_deadline_delay: u64,
    check_time: u64,
    ffi_out_policy_reject: *mut PaymentStreamsFfiPolicyRejectReason,
) -> PaymentStreamsFfiStatus {
    if ffi_out_policy_reject.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    match create_stream_deadline_satisfies_policy_as_of(
        params_create_stream_deadline,
        policy_max_create_stream_deadline_delay,
        check_time,
    ) {
        Ok(()) => PaymentStreamsFfiStatus::Success,
        Err(reason) => {
            *ffi_out_policy_reject = map_policy_rejection(reason);
            PaymentStreamsFfiStatus::PolicyRejected
        }
    }
}

/// First service proof binds folded on-chain state to accepted negotiation terms.
///
/// # Safety
///
/// - `ffi_decoded_stream`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiDecodedStreamConfig`].
/// - `ffi_accepted_params`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiStreamParams`].
/// - `proposal_provider_id_bytes`: non-null, aligned pointer valid for immutable reads spanning 32 bytes.
/// - `ffi_out_policy_reject`: non-null, aligned pointer writable for exactly one [`PaymentStreamsFfiPolicyRejectReason`].
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_new_stream_satisfies_proposal(
    ffi_decoded_stream: *const PaymentStreamsFfiDecodedStreamConfig,
    ffi_accepted_params: *const PaymentStreamsFfiStreamParams,
    proposal_provider_id_bytes: *const u8,
    ffi_out_policy_reject: *mut PaymentStreamsFfiPolicyRejectReason,
) -> PaymentStreamsFfiStatus {
    if ffi_decoded_stream.is_null()
        || ffi_accepted_params.is_null()
        || proposal_provider_id_bytes.is_null()
        || ffi_out_policy_reject.is_null()
    {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let proposal_payee_binding =
        match proposal_provider_binding_from_ffi_bytes(proposal_provider_id_bytes) {
            Ok(id) => id,
            Err(err) => return err,
        };

    match (
        stream_config_from_ffi(&*ffi_decoded_stream),
        stream_params_from_ffi(&*ffi_accepted_params),
    ) {
        (Err(err), _) | (_, Err(err)) => err,
        (Ok(on_chain_folded_stream), Ok(accepted_params)) => match new_stream_satisfies_proposal(
            &on_chain_folded_stream,
            &accepted_params,
            proposal_payee_binding,
        ) {
            Ok(()) => PaymentStreamsFfiStatus::Success,
            Err(reason) => {
                *ffi_out_policy_reject = map_policy_rejection(reason);
                PaymentStreamsFfiStatus::PolicyRejected
            }
        },
    }
}

/// Ongoing proofs must respect the pinned policy snapshot + active stream state.
///
/// # Safety
///
/// - `ffi_decoded_stream`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiDecodedStreamConfig`].
/// - `ffi_accepted_terms_snapshot`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiAcceptedStreamTerms`].
/// - `ffi_out_policy_reject`: non-null, aligned pointer writable for exactly one [`PaymentStreamsFfiPolicyRejectReason`].
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_stream_satisfies_policy(
    ffi_decoded_stream: *const PaymentStreamsFfiDecodedStreamConfig,
    ffi_accepted_terms_snapshot: *const PaymentStreamsFfiAcceptedStreamTerms,
    ffi_out_policy_reject: *mut PaymentStreamsFfiPolicyRejectReason,
) -> PaymentStreamsFfiStatus {
    if ffi_decoded_stream.is_null()
        || ffi_accepted_terms_snapshot.is_null()
        || ffi_out_policy_reject.is_null()
    {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let accepted_terms = match accepted_terms_from_ffi(&*ffi_accepted_terms_snapshot) {
        Ok(snapshot) => snapshot,
        Err(err) => return err,
    };

    match stream_config_from_ffi(&*ffi_decoded_stream) {
        Err(err_status) => err_status,
        Ok(on_chain_folded_stream) => {
            match stream_satisfies_policy(&on_chain_folded_stream, &accepted_terms) {
                Ok(()) => PaymentStreamsFfiStatus::Success,
                Err(reason) => {
                    *ffi_out_policy_reject = map_policy_rejection(reason);
                    PaymentStreamsFfiStatus::PolicyRejected
                }
            }
        }
    }
}

/// Outbound vault proof payload sizing guard enforced by MVP providers (`response_within_policy`).
///
/// Argument order mirrors core: serialized response byte length first, then policy snapshot.
///
/// # Safety
///
/// - `ffi_policy_snapshot`: non-null, aligned pointer valid for immutable reads spanning one [`PaymentStreamsFfiStreamProviderPolicy`].
/// - `ffi_out_policy_reject`: non-null, aligned pointer writable for exactly one [`PaymentStreamsFfiPolicyRejectReason`].
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_response_within_policy(
    response_payload_byte_len: u64,
    ffi_policy_snapshot: *const PaymentStreamsFfiStreamProviderPolicy,
    ffi_out_policy_reject: *mut PaymentStreamsFfiPolicyRejectReason,
) -> PaymentStreamsFfiStatus {
    if ffi_policy_snapshot.is_null() || ffi_out_policy_reject.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let policy_snapshot = stream_provider_policy_from_ffi(&*ffi_policy_snapshot);

    match response_within_policy(response_payload_byte_len, &policy_snapshot) {
        Ok(()) => PaymentStreamsFfiStatus::Success,
        Err(reason) => {
            *ffi_out_policy_reject = map_policy_rejection(reason);
            PaymentStreamsFfiStatus::PolicyRejected
        }
    }
}
