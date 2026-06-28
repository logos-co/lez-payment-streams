#include "payment_streams_ffi_bridge.h"

#include <lez_payment_streams_ffi.h>
#include <string.h>

static uint32_t map_status(PaymentStreamsFfiPaymentStreamsFfiStatus status) {
    return (uint32_t)status;
}

uint32_t ps_ffi_fixed_clock_10_account_id(uint8_t out_account_id_bytes[32]) {
    return map_status(payment_streams_ffi_fixed_clock_account_id(
        PAYMENT_STREAMS_FFI_CLOCK_ACCOUNT_CHOICE_CLOCK01, out_account_id_bytes));
}

uint32_t ps_ffi_authenticated_transfer_program_id(uint8_t out_bytes[32]) {
    return map_status(payment_streams_ffi_authenticated_transfer_program_id_bytes(out_bytes));
}

uint32_t ps_ffi_derive_vault_account_ids(const uint8_t program_id_bytes[32],
                                         const uint8_t owner_account_id_bytes[32],
                                         uint64_t vault_id,
                                         uint8_t out_vault_config_account_id_bytes[32],
                                         uint8_t out_vault_holding_account_id_bytes[32]) {
    return map_status(payment_streams_ffi_derive_vault_account_ids(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        out_vault_config_account_id_bytes,
        out_vault_holding_account_id_bytes));
}

uint32_t ps_ffi_derive_stream_config_account_id(const uint8_t program_id_bytes[32],
                                                const uint8_t vault_config_account_id_bytes[32],
                                                uint64_t stream_id,
                                                uint8_t out_stream_config_account_id_bytes[32]) {
    return map_status(payment_streams_ffi_derive_stream_config_account_id(
        program_id_bytes,
        vault_config_account_id_bytes,
        stream_id,
        out_stream_config_account_id_bytes));
}

uint32_t ps_ffi_fold_stream_at(const PsFfiDecodedStreamConfig* stream,
                               uint64_t as_of,
                               PsFfiStreamFoldAtTime* out_fold,
                               uint32_t* guest_error_out) {
    if (stream == NULL || out_fold == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    PaymentStreamsFfiPaymentStreamsFfiStreamFoldAtTime fold;
    memset(&fold, 0, sizeof(fold));
    const PaymentStreamsFfiPaymentStreamsFfiStatus status = payment_streams_ffi_fold_stream(
        (const PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig*)stream,
        as_of,
        &fold,
        guest_error_out);
    if (status == PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_SUCCESS) {
        memcpy(&out_fold->folded_stream, &fold.folded_stream, sizeof(out_fold->folded_stream));
        out_fold->accrued_lo = fold.accrued_lo;
        out_fold->accrued_hi = fold.accrued_hi;
        out_fold->unaccrued_lo = fold.unaccrued_lo;
        out_fold->unaccrued_hi = fold.unaccrued_hi;
        out_fold->as_of = fold.as_of;
    }
    return map_status(status);
}

uint32_t ps_ffi_serialize_initialize_vault(uint64_t vault_id,
                                           uint8_t privacy_tier,
                                           uint8_t* out_ptr,
                                           size_t out_cap,
                                           size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_initialize_vault_instruction(
        vault_id, privacy_tier, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_plan_initialize_vault(const uint8_t program_id_bytes[32],
                                      const uint8_t owner_account_id_bytes[32],
                                      uint64_t vault_id,
                                      uint8_t* accounts_hex_out,
                                      size_t accounts_hex_out_cap,
                                      size_t* accounts_hex_out_len) {
    return map_status(payment_streams_ffi_plan_initialize_vault_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len));
}

uint32_t ps_ffi_serialize_deposit(uint64_t vault_id,
                                  uint64_t amount_lo,
                                  uint64_t amount_hi,
                                  const uint8_t authenticated_transfer_program_id_bytes[32],
                                  uint8_t* out_ptr,
                                  size_t out_cap,
                                  size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_deposit_instruction(
        vault_id,
        amount_lo,
        amount_hi,
        authenticated_transfer_program_id_bytes,
        out_ptr,
        out_cap,
        out_len));
}

uint32_t ps_ffi_plan_deposit(const uint8_t program_id_bytes[32],
                             const uint8_t owner_account_id_bytes[32],
                             uint64_t vault_id,
                             uint8_t* accounts_hex_out,
                             size_t accounts_hex_out_cap,
                             size_t* accounts_hex_out_len) {
    return map_status(payment_streams_ffi_plan_deposit_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len));
}

uint32_t ps_ffi_serialize_withdraw(uint64_t vault_id,
                                   uint64_t amount_lo,
                                   uint64_t amount_hi,
                                   uint8_t* out_ptr,
                                   size_t out_cap,
                                   size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_withdraw_instruction(
        vault_id, amount_lo, amount_hi, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_plan_withdraw(const uint8_t program_id_bytes[32],
                              const uint8_t owner_account_id_bytes[32],
                              uint64_t vault_id,
                              const uint8_t withdraw_to_account_id_bytes[32],
                              uint8_t* accounts_hex_out,
                              size_t accounts_hex_out_cap,
                              size_t* accounts_hex_out_len) {
    return map_status(payment_streams_ffi_plan_withdraw_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        withdraw_to_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len));
}

uint32_t ps_ffi_serialize_create_stream(uint64_t vault_id,
                                        uint64_t stream_id,
                                        const uint8_t provider_account_id_bytes[32],
                                        uint64_t rate,
                                        uint64_t allocation_lo,
                                        uint64_t allocation_hi,
                                        uint8_t* out_ptr,
                                        size_t out_cap,
                                        size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_create_stream_instruction(
        vault_id,
        stream_id,
        provider_account_id_bytes,
        rate,
        allocation_lo,
        allocation_hi,
        out_ptr,
        out_cap,
        out_len));
}

uint32_t ps_ffi_plan_create_stream(const uint8_t program_id_bytes[32],
                                   const uint8_t owner_account_id_bytes[32],
                                   uint64_t vault_id,
                                   uint64_t stream_id,
                                   const uint8_t clock_account_id_bytes[32],
                                   uint8_t* accounts_hex_out,
                                   size_t accounts_hex_out_cap,
                                   size_t* accounts_hex_out_len) {
    return map_status(payment_streams_ffi_plan_create_stream_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len));
}

uint32_t ps_ffi_serialize_pause_stream(uint64_t vault_id,
                                       uint64_t stream_id,
                                       uint8_t* out_ptr,
                                       size_t out_cap,
                                       size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_pause_stream_instruction(
        vault_id, stream_id, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_plan_pause_stream(const uint8_t program_id_bytes[32],
                                  const uint8_t owner_account_id_bytes[32],
                                  uint64_t vault_id,
                                  uint64_t stream_id,
                                  const uint8_t clock_account_id_bytes[32],
                                  uint8_t* accounts_hex_out,
                                  size_t accounts_hex_out_cap,
                                  size_t* accounts_hex_out_len) {
    return map_status(payment_streams_ffi_plan_pause_stream_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len));
}

uint32_t ps_ffi_serialize_resume_stream(uint64_t vault_id,
                                        uint64_t stream_id,
                                        uint8_t* out_ptr,
                                        size_t out_cap,
                                        size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_resume_stream_instruction(
        vault_id, stream_id, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_plan_resume_stream(const uint8_t program_id_bytes[32],
                                   const uint8_t owner_account_id_bytes[32],
                                   uint64_t vault_id,
                                   uint64_t stream_id,
                                   const uint8_t clock_account_id_bytes[32],
                                   uint8_t* accounts_hex_out,
                                   size_t accounts_hex_out_cap,
                                   size_t* accounts_hex_out_len) {
    return map_status(payment_streams_ffi_plan_resume_stream_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len));
}

uint32_t ps_ffi_serialize_top_up_stream(uint64_t vault_id,
                                        uint64_t stream_id,
                                        uint64_t increase_lo,
                                        uint64_t increase_hi,
                                        uint8_t* out_ptr,
                                        size_t out_cap,
                                        size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_top_up_stream_instruction(
        vault_id, stream_id, increase_lo, increase_hi, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_plan_top_up_stream(const uint8_t program_id_bytes[32],
                                   const uint8_t owner_account_id_bytes[32],
                                   uint64_t vault_id,
                                   uint64_t stream_id,
                                   const uint8_t clock_account_id_bytes[32],
                                   uint8_t* accounts_hex_out,
                                   size_t accounts_hex_out_cap,
                                   size_t* accounts_hex_out_len) {
    return map_status(payment_streams_ffi_plan_top_up_stream_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len));
}

uint32_t ps_ffi_serialize_close_stream(uint64_t vault_id,
                                       uint64_t stream_id,
                                       uint8_t* out_ptr,
                                       size_t out_cap,
                                       size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_close_stream_instruction(
        vault_id, stream_id, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_plan_close_stream(const uint8_t program_id_bytes[32],
                                  const uint8_t owner_account_id_bytes[32],
                                  uint64_t vault_id,
                                  uint64_t stream_id,
                                  const uint8_t authority_account_id_bytes[32],
                                  const uint8_t clock_account_id_bytes[32],
                                  uint8_t* accounts_hex_out,
                                  size_t accounts_hex_out_cap,
                                  size_t* accounts_hex_out_len) {
    return map_status(payment_streams_ffi_plan_close_stream_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        authority_account_id_bytes,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len));
}

uint32_t ps_ffi_serialize_claim(uint64_t vault_id,
                                uint64_t stream_id,
                                uint8_t* out_ptr,
                                size_t out_cap,
                                size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_claim_instruction(
        vault_id, stream_id, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_plan_claim(const uint8_t program_id_bytes[32],
                           const uint8_t owner_account_id_bytes[32],
                           uint64_t vault_id,
                           uint64_t stream_id,
                           const uint8_t provider_account_id_bytes[32],
                           const uint8_t clock_account_id_bytes[32],
                           uint8_t* accounts_hex_out,
                           size_t accounts_hex_out_cap,
                           size_t* accounts_hex_out_len) {
    return map_status(payment_streams_ffi_plan_claim_instruction_accounts(
        program_id_bytes,
        owner_account_id_bytes,
        vault_id,
        stream_id,
        provider_account_id_bytes,
        clock_account_id_bytes,
        accounts_hex_out,
        accounts_hex_out_cap,
        accounts_hex_out_len));
}

uint32_t ps_ffi_generate_session_keypair(uint8_t out_secret_key_32[32], uint8_t out_public_key_32[32]) {
    return map_status(payment_streams_ffi_generate_session_keypair(out_secret_key_32, out_public_key_32));
}

uint32_t ps_ffi_store_eligibility_digest_from_n8_wire(const uint8_t* n8_wire,
                                                      size_t n8_wire_len,
                                                      uint8_t out_digest_32[32]) {
    return map_status(payment_streams_ffi_store_eligibility_canonical_payload_digest_from_n8_wire_bytes(
        n8_wire, n8_wire_len, out_digest_32));
}

uint32_t ps_ffi_vault_owner_auth_digest_from_decoded_proposal(const PsFfiDecodedStreamProposal* proposal,
                                                              uint8_t out_digest_32[32]) {
    if (proposal == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProposal ffi_proposal;
    memcpy(&ffi_proposal, proposal, sizeof(ffi_proposal));
    return map_status(
        payment_streams_ffi_vault_owner_auth_canonical_payload_digest_from_decoded_proposal(
            &ffi_proposal, out_digest_32));
}

uint32_t ps_ffi_serialize_stream_proposal_decoded(const PsFfiDecodedStreamProposal* proposal,
                                                  uint8_t* out_ptr,
                                                  size_t out_cap,
                                                  size_t* out_len) {
    if (proposal == NULL || out_len == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProposal ffi_proposal;
    memcpy(&ffi_proposal, proposal, sizeof(ffi_proposal));
    return map_status(payment_streams_ffi_serialize_stream_proposal_bytes(
        &ffi_proposal, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_serialize_stream_proof_for_n8_wire(uint64_t stream_id,
                                                   const uint8_t secret_key_32[32],
                                                   const uint8_t* n8_wire,
                                                   size_t n8_wire_len,
                                                   uint8_t* out_ptr,
                                                   size_t out_cap,
                                                   size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_stream_proof_for_n8_wire(
        stream_id, secret_key_32, n8_wire, n8_wire_len, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_serialize_eligibility_proof_stream_proposal(const uint8_t* inner_ptr,
                                                            size_t inner_len,
                                                            uint8_t* out_ptr,
                                                            size_t out_cap,
                                                            size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_eligibility_proof_stream_proposal_bytes(
        inner_ptr, inner_len, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_serialize_eligibility_proof_stream_proof(const uint8_t* inner_ptr,
                                                         size_t inner_len,
                                                         uint8_t* out_ptr,
                                                         size_t out_cap,
                                                         size_t* out_len) {
    return map_status(payment_streams_ffi_serialize_eligibility_proof_stream_proof_bytes(
        inner_ptr, inner_len, out_ptr, out_cap, out_len));
}

uint32_t ps_ffi_parse_eligibility_proof_bytes(const uint8_t* data,
                                              size_t data_len,
                                              uint32_t* out_arm,
                                              uint8_t* inner_out,
                                              size_t inner_cap,
                                              size_t* inner_len) {
    if (out_arm == NULL || inner_len == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    return map_status(payment_streams_ffi_parse_eligibility_proof_bytes(
        data, data_len, out_arm, inner_out, inner_cap, inner_len));
}

uint32_t ps_ffi_parse_stream_proposal_bytes(const uint8_t* data,
                                            size_t data_len,
                                            PsFfiDecodedStreamProposal* out_proposal) {
    if (out_proposal == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    return map_status(payment_streams_ffi_parse_stream_proposal_bytes(
        data,
        data_len,
        (struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProposal*)out_proposal));
}

uint32_t ps_ffi_parse_stream_proof_bytes(const uint8_t* data,
                                         size_t data_len,
                                         PsFfiDecodedStreamProof* out_proof) {
    if (out_proof == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    return map_status(payment_streams_ffi_parse_stream_proof_bytes(
        data, data_len, (struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamProof*)out_proof));
}

uint32_t ps_ffi_verify_stream_proposal_vault_proof_bytes(const uint8_t* proposal_bytes,
                                                         size_t proposal_len,
                                                         const uint8_t vault_owner_id[32]) {
    if (vault_owner_id == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    return map_status(payment_streams_ffi_verify_stream_proposal_vault_proof_bytes(
        proposal_bytes, proposal_len, vault_owner_id));
}

uint32_t ps_ffi_verify_stream_proof_for_n8_wire(const uint8_t* proof_bytes,
                                                size_t proof_len,
                                                const uint8_t session_public_key[32],
                                                const uint8_t* n8_wire,
                                                size_t n8_wire_len) {
    if (session_public_key == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    return map_status(payment_streams_ffi_verify_stream_proof_for_n8_wire_bytes(
        proof_bytes, proof_len, session_public_key, n8_wire, n8_wire_len));
}

uint32_t ps_ffi_proposal_satisfies_policy(const PsFfiProposalCheckInputs* inputs, uint32_t* reject_reason_out) {
    if (inputs == NULL || reject_reason_out == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    return map_status(payment_streams_ffi_proposal_satisfies_policy(
        (const struct PaymentStreamsFfiPaymentStreamsFfiProposalCheckInputs*)inputs,
        (PaymentStreamsFfiPaymentStreamsFfiPolicyRejectReason*)reject_reason_out));
}

uint32_t ps_ffi_new_stream_satisfies_proposal(const PsFfiDecodedStreamConfig* decoded_stream,
                                              const PsFfiStreamParams* accepted_params,
                                              const uint8_t provider_id[32],
                                              uint32_t* reject_reason_out) {
    if (decoded_stream == NULL || accepted_params == NULL || provider_id == NULL || reject_reason_out == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    return map_status(payment_streams_ffi_new_stream_satisfies_proposal(
        (const struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig*)decoded_stream,
        (const struct PaymentStreamsFfiPaymentStreamsFfiStreamParams*)accepted_params,
        provider_id,
        (PaymentStreamsFfiPaymentStreamsFfiPolicyRejectReason*)reject_reason_out));
}

uint32_t ps_ffi_stream_satisfies_policy(const PsFfiDecodedStreamConfig* folded_stream,
                                        const PsFfiAcceptedStreamTerms* accepted_terms,
                                        uint32_t* reject_reason_out) {
    if (folded_stream == NULL || accepted_terms == NULL || reject_reason_out == NULL) {
        return map_status(PAYMENT_STREAMS_FFI_PAYMENT_STREAMS_FFI_STATUS_NULL_POINTER);
    }
    return map_status(payment_streams_ffi_stream_satisfies_policy(
        (const struct PaymentStreamsFfiPaymentStreamsFfiDecodedStreamConfig*)folded_stream,
        (const struct PaymentStreamsFfiPaymentStreamsFfiAcceptedStreamTerms*)accepted_terms,
        (PaymentStreamsFfiPaymentStreamsFfiPolicyRejectReason*)reject_reason_out));
}
