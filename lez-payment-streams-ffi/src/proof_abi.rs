//! C ABI for LIP‑155 off-chain proofs (protobuf framing + canonical payload / digest + NSSA Schnorr).

use core::slice;

use lez_payment_streams_core::{
    generate_session_keypair, parse_eligibility_proof, parse_stream_proof, parse_stream_proposal,
    serialize_eligibility_proof,
    serialize_stream_proof, serialize_stream_proposal,
    sign_canonical_payload_digest, store_eligibility_canonical_payload_digest_from_n8_wire,
    vault_owner_auth_canonical_payload_digest, verify_canonical_payload_digest,
    verify_stream_proof_for_store_query, verify_stream_proposal_vault_proof, CanonicalStoreQueryParts,
    EligibilityProofWire, OffChainError, StreamProofWire, StreamProposalWire, VaultProofWire,
    WireError,
};

use lee::PrivateKey;
use lee_core::account::AccountId;

use crate::policy_abi::stream_params_from_ffi;
use crate::{
    balance_pair, borrow_input, PaymentStreamsFfiByteSpan, PaymentStreamsFfiCanonicalStoreQuery,
    PaymentStreamsFfiDecodedStreamProof, PaymentStreamsFfiDecodedStreamProposal,
    PaymentStreamsFfiStatus, PAYMENT_STREAMS_FFI_MAX_SERVICE_ID_LEN,
};

/// Maps [`OffChainError`] from `lez-payment-streams-core` (Step 4) to FFI status codes.
///
/// Rationale:
/// - [`PaymentStreamsFfiStatus::Malformed`]: bad protobuf, bad UTF-8 in wire-typed strings, bad key
///   bytes, truncated frames (caller or transport supplied unusable bytes).
/// - [`PaymentStreamsFfiStatus::ProofInvalid`]: structural parsing succeeded but cryptography or vault
///   owner binding failed (wrong owner account vs `owner_public_key`, or Schnorr did not verify).
///
/// Both misuse classes surface as LIP‑155 proof/eligibility failure at a higher layer; this split
/// keeps “replace the bytes” distinct from “bytes parse but proof is wrong”.
///
/// `ProofInvalid` does not distinguish `OwnerKeyMismatch` from `BadSignature`; C callers need finer
/// detail should use the appropriate verify helper sequence or the Rust `OffChainError` API.
fn map_off_chain_err(err: OffChainError) -> PaymentStreamsFfiStatus {
    match err {
        OffChainError::Wire(_) | OffChainError::InvalidPublicKey => {
            PaymentStreamsFfiStatus::Malformed
        }
        OffChainError::OwnerKeyMismatch | OffChainError::BadSignature => {
            PaymentStreamsFfiStatus::ProofInvalid
        }
    }
}

fn map_wire_err(err: WireError) -> PaymentStreamsFfiStatus {
    map_off_chain_err(OffChainError::Wire(err))
}

unsafe fn read_span<'a>(
    span: PaymentStreamsFfiByteSpan,
) -> Result<&'a [u8], PaymentStreamsFfiStatus> {
    borrow_input(span.ptr, span.len)
}

fn proposal_wire_to_ffi(
    out: &mut PaymentStreamsFfiDecodedStreamProposal,
    wire: &StreamProposalWire,
) {
    let allocation_halves = balance_pair(wire.params.allocation);
    let mut service_id_scratch = [0_u8; PAYMENT_STREAMS_FFI_MAX_SERVICE_ID_LEN];
    let sid_len = wire.params.service_id.len();
    service_id_scratch[..sid_len].copy_from_slice(&wire.params.service_id);

    out.vault_proof.vault_id = wire.vault.vault_id;
    out.vault_proof.provider_id = wire.vault.provider_id;
    out.vault_proof.owner_public_key = wire.vault.owner_public_key;
    out.vault_proof.owner_signature = wire.vault.owner_signature;

    out.params.rate = wire.params.rate;
    out.params.allocation_lo = allocation_halves.0;
    out.params.allocation_hi = allocation_halves.1;
    out.params.create_stream_deadline = wire.params.create_stream_deadline;
    out.params.service_id_len = sid_len as u32;
    out.params._padding = 0;
    out.params.service_id_bytes = service_id_scratch;

    out.session_public_key = wire.session_public_key;
}

fn proposal_ffi_to_wire(
    ffi_proposal: &PaymentStreamsFfiDecodedStreamProposal,
) -> Result<StreamProposalWire, PaymentStreamsFfiStatus> {
    let params = stream_params_from_ffi(&ffi_proposal.params)?;
    Ok(StreamProposalWire {
        vault: VaultProofWire {
            vault_id: ffi_proposal.vault_proof.vault_id,
            provider_id: ffi_proposal.vault_proof.provider_id,
            owner_public_key: ffi_proposal.vault_proof.owner_public_key,
            owner_signature: ffi_proposal.vault_proof.owner_signature,
        },
        params,
        session_public_key: ffi_proposal.session_public_key,
    })
}

/// Owned Store query built from [`PaymentStreamsFfiCanonicalStoreQuery`].
///
/// Byte spans from the C side are copied into owned `String` / `Vec` immediately so this struct does
/// not borrow caller memory (simpler lifetimes; typical query sizes are small). `canonical_parts()`
/// then borrows those buffers for core [`CanonicalStoreQueryParts`].
struct ParsedStoreQuery {
    request_id: String,
    pubsub_topic: Option<String>,
    content_topics: Vec<String>,
    message_hashes: Vec<[u8; 32]>,
    include_data: bool,
    start_time: Option<i64>,
    end_time: Option<i64>,
    pagination_cursor: Option<[u8; 32]>,
    pagination_forward: bool,
    pagination_limit: Option<u64>,
}

impl ParsedStoreQuery {
    /// Reads UTF-8 spans and fixed fields from the FFI descriptor into owned buffers.
    unsafe fn parse(
        query: &PaymentStreamsFfiCanonicalStoreQuery,
    ) -> Result<Self, PaymentStreamsFfiStatus> {
        let rid_bytes = read_span(query.request_id)?;
        let request_id = String::from_utf8(rid_bytes.to_vec())
            .map_err(|_| PaymentStreamsFfiStatus::Malformed)?;

        let pubsub_topic = if query.has_pubsub_topic != 0 {
            let bytes = read_span(query.pubsub_topic)?;
            Some(
                String::from_utf8(bytes.to_vec())
                    .map_err(|_| PaymentStreamsFfiStatus::Malformed)?,
            )
        } else {
            None
        };

        let mut content_topics = Vec::new();
        let topics_len = query.content_topics_len as usize;
        if topics_len > 0 {
            if query.content_topics.is_null() {
                return Err(PaymentStreamsFfiStatus::NullPointer);
            }
            let spans = slice::from_raw_parts(query.content_topics, topics_len);
            for span in spans {
                let bytes = read_span(*span)?;
                content_topics.push(
                    String::from_utf8(bytes.to_vec())
                        .map_err(|_| PaymentStreamsFfiStatus::Malformed)?,
                );
            }
        }

        let mut message_hashes = Vec::new();
        if query.message_hashes_len > 0 {
            let total_bytes_u64 = query
                .message_hashes_len
                .checked_mul(32)
                .ok_or(PaymentStreamsFfiStatus::Malformed)?;
            let total_bytes =
                usize::try_from(total_bytes_u64).map_err(|_| PaymentStreamsFfiStatus::Malformed)?;
            let bytes = borrow_input(query.message_hashes, total_bytes)?;
            for chunk in bytes.chunks_exact(32) {
                message_hashes.push(
                    chunk
                        .try_into()
                        .map_err(|_| PaymentStreamsFfiStatus::Malformed)?,
                );
            }
        }

        Ok(Self {
            request_id,
            pubsub_topic,
            content_topics,
            message_hashes,
            include_data: query.include_data != 0,
            start_time: if query.has_start_time != 0 {
                Some(query.start_time)
            } else {
                None
            },
            end_time: if query.has_end_time != 0 {
                Some(query.end_time)
            } else {
                None
            },
            pagination_cursor: if query.has_pagination_cursor != 0 {
                Some(query.pagination_cursor)
            } else {
                None
            },
            pagination_forward: query.pagination_forward != 0,
            pagination_limit: if query.has_pagination_limit != 0 {
                Some(query.pagination_limit)
            } else {
                None
            },
        })
    }

    fn canonical_parts(&self) -> CanonicalStoreQueryParts<'_> {
        CanonicalStoreQueryParts {
            request_id: self.request_id.as_str(),
            include_data: self.include_data,
            pubsub_topic: self.pubsub_topic.as_deref(),
            content_topics: &self.content_topics,
            start_time: self.start_time,
            end_time: self.end_time,
            message_hashes: &self.message_hashes,
            pagination_cursor: self.pagination_cursor,
            pagination_forward: self.pagination_forward,
            pagination_limit: self.pagination_limit,
        }
    }

    fn store_eligibility_canonical_payload_digest(&self) -> [u8; 32] {
        lez_payment_streams_core::store_eligibility_canonical_payload_digest(
            &self.canonical_parts(),
        )
    }
}

/// Deserialize a protobuf `StreamProposal` into the flattened FFI view (LEZ width limits enforced).
///
/// # Safety
///
/// `(data_ptr, data_len)` must be a readable range; `ffi_out_proposal` must be non-null.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_parse_stream_proposal_bytes(
    data_ptr: *const u8,
    data_len: usize,
    ffi_out_proposal: *mut PaymentStreamsFfiDecodedStreamProposal,
) -> PaymentStreamsFfiStatus {
    let Some(out) = ffi_out_proposal.as_mut() else {
        return PaymentStreamsFfiStatus::NullPointer;
    };

    let bytes = match borrow_input(data_ptr, data_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    match parse_stream_proposal(bytes) {
        Err(err) => map_wire_err(err),
        Ok(wire) => {
            proposal_wire_to_ffi(out, &wire);
            PaymentStreamsFfiStatus::Success
        }
    }
}

/// Serialize a `StreamProposal` protobuf frame from the flattened FFI view.
///
/// When `out_ptr` is null this returns [`PaymentStreamsFfiStatus::Success`] after writing the required
/// buffer size to `out_len` (sizing pass).
///
/// # Safety
///
/// `ffi_proposal` must be non-null. When `out_ptr` is non-null it must address `out_cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_stream_proposal_bytes(
    ffi_proposal: *const PaymentStreamsFfiDecodedStreamProposal,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    if out_len.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }
    *out_len = 0;

    let ffi_proposal_ref = match ffi_proposal.as_ref() {
        None => return PaymentStreamsFfiStatus::NullPointer,
        Some(value) => value,
    };

    let wire = match proposal_ffi_to_wire(ffi_proposal_ref) {
        Err(status) => return status,
        Ok(value) => value,
    };

    let encoded = match serialize_stream_proposal(&wire) {
        Err(err) => return map_wire_err(err),
        Ok(value) => value,
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

/// Deserialize a protobuf `StreamProof`.
///
/// # Safety
///
/// `(data_ptr, data_len)` must be readable; `ffi_out_proof` must be non-null.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_parse_stream_proof_bytes(
    data_ptr: *const u8,
    data_len: usize,
    ffi_out_proof: *mut PaymentStreamsFfiDecodedStreamProof,
) -> PaymentStreamsFfiStatus {
    let Some(out) = ffi_out_proof.as_mut() else {
        return PaymentStreamsFfiStatus::NullPointer;
    };

    let bytes = match borrow_input(data_ptr, data_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    match parse_stream_proof(bytes) {
        Err(err) => map_wire_err(err),
        Ok(wire) => {
            out.stream_id = wire.stream_id;
            out.signature = wire.signature;
            PaymentStreamsFfiStatus::Success
        }
    }
}

/// Serialize a protobuf `StreamProof`.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_stream_proof_bytes(
    ffi_proof: *const PaymentStreamsFfiDecodedStreamProof,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    if out_len.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }
    *out_len = 0;

    let ffi_proof_ref = match ffi_proof.as_ref() {
        None => return PaymentStreamsFfiStatus::NullPointer,
        Some(value) => value,
    };

    let wire = StreamProofWire {
        stream_id: ffi_proof_ref.stream_id,
        signature: ffi_proof_ref.signature,
    };
    let encoded = serialize_stream_proof(&wire);
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

/// Write the 32-byte vault-owner canonical payload digest for a decoded proposal (`VaultProof.owner_signature` signs it).
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_vault_owner_auth_canonical_payload_digest_from_decoded_proposal(
    ffi_proposal: *const PaymentStreamsFfiDecodedStreamProposal,
    out_canonical_payload_digest: *mut u8,
) -> PaymentStreamsFfiStatus {
    if out_canonical_payload_digest.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let ffi_proposal_ref = match ffi_proposal.as_ref() {
        None => return PaymentStreamsFfiStatus::NullPointer,
        Some(value) => value,
    };

    let wire = match proposal_ffi_to_wire(ffi_proposal_ref) {
        Err(status) => return status,
        Ok(value) => value,
    };

    let digest = match vault_owner_auth_canonical_payload_digest(
        wire.vault.vault_id,
        &wire.vault.provider_id,
        &wire.vault.owner_public_key,
        &wire.params,
        &wire.session_public_key,
    ) {
        Ok(value) => value,
        Err(err) => return map_off_chain_err(err.into()),
    };

    slice::from_raw_parts_mut(out_canonical_payload_digest, 32).copy_from_slice(&digest);
    PaymentStreamsFfiStatus::Success
}

/// Verify `VaultProof.owner_signature` + derived owner binding against `VaultConfig.owner`.
///
/// # Safety
///
/// Inputs must follow [`borrow_input`] rules; `vault_owner_id` must address 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_verify_stream_proposal_vault_proof_bytes(
    proposal_ptr: *const u8,
    proposal_len: usize,
    vault_owner_id: *const u8,
) -> PaymentStreamsFfiStatus {
    let proposal_bytes = match borrow_input(proposal_ptr, proposal_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let owner_bytes = match borrow_input(vault_owner_id, 32) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let owner = match <[u8; 32]>::try_from(owner_bytes) {
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
        Ok(value) => AccountId::new(value),
    };

    let wire = match parse_stream_proposal(proposal_bytes) {
        Err(err) => return map_wire_err(err),
        Ok(value) => value,
    };

    match verify_stream_proposal_vault_proof(&wire, &owner) {
        Ok(()) => PaymentStreamsFfiStatus::Success,
        Err(err) => map_off_chain_err(err),
    }
}

/// Write the 32-byte Store eligibility `canonical_payload_digest` described in integration plan N8.
///
/// # Safety
///
/// `query` must be non-null and all nested spans must satisfy [`borrow_input`] rules.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_store_eligibility_canonical_payload_digest(
    query: *const PaymentStreamsFfiCanonicalStoreQuery,
    out_canonical_payload_digest: *mut u8,
) -> PaymentStreamsFfiStatus {
    if query.is_null() || out_canonical_payload_digest.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let query_ref = &*query;
    let parsed = match ParsedStoreQuery::parse(query_ref) {
        Err(status) => return status,
        Ok(value) => value,
    };

    let digest = parsed.store_eligibility_canonical_payload_digest();
    slice::from_raw_parts_mut(out_canonical_payload_digest, 32).copy_from_slice(&digest);
    PaymentStreamsFfiStatus::Success
}

/// Verify `StreamProof.signature` over the canonical Store query described by `query`.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_verify_stream_proof_for_store_query(
    proof_ptr: *const u8,
    proof_len: usize,
    session_public_key: *const u8,
    query: *const PaymentStreamsFfiCanonicalStoreQuery,
) -> PaymentStreamsFfiStatus {
    if query.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let proof_bytes = match borrow_input(proof_ptr, proof_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let session_bytes = match borrow_input(session_public_key, 32) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let session_arr: [u8; 32] = match session_bytes.try_into() {
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
        Ok(value) => value,
    };

    let proof_wire = match parse_stream_proof(proof_bytes) {
        Err(err) => return map_wire_err(err),
        Ok(value) => value,
    };

    let query_ref = &*query;
    let parsed = match ParsedStoreQuery::parse(query_ref) {
        Err(status) => return status,
        Ok(value) => value,
    };

    match verify_stream_proof_for_store_query(&proof_wire, &session_arr, &parsed.canonical_parts())
    {
        Ok(()) => PaymentStreamsFfiStatus::Success,
        Err(err) => map_off_chain_err(err),
    }
}

/// Sign a 32-byte `canonical_payload_digest` with a 32-byte NSSA private key (Schnorr signature writes 64 bytes).
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_sign_canonical_payload_digest(
    secret_key: *const u8,
    canonical_payload_digest: *const u8,
    out_signature: *mut u8,
) -> PaymentStreamsFfiStatus {
    if out_signature.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }

    let secret = match borrow_input(secret_key, 32) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let digest = match borrow_input(canonical_payload_digest, 32) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let secret_arr: [u8; 32] = match secret.try_into() {
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
        Ok(value) => value,
    };

    let digest_arr: [u8; 32] = match digest.try_into() {
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
        Ok(value) => value,
    };

    let key = match PrivateKey::try_new(secret_arr) {
        Ok(value) => value,
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
    };

    let signature = sign_canonical_payload_digest(&key, &digest_arr);
    slice::from_raw_parts_mut(out_signature, 64).copy_from_slice(&signature);
    PaymentStreamsFfiStatus::Success
}

/// Verify a 32-byte `canonical_payload_digest` against a 64-byte Schnorr signature and 32-byte public key.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_verify_canonical_payload_digest(
    public_key: *const u8,
    canonical_payload_digest: *const u8,
    signature: *const u8,
) -> PaymentStreamsFfiStatus {
    let pubkey = match borrow_input(public_key, 32) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let digest = match borrow_input(canonical_payload_digest, 32) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let sig_bytes = match borrow_input(signature, 64) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let pubkey_arr: [u8; 32] = match pubkey.try_into() {
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
        Ok(value) => value,
    };

    let digest_arr: [u8; 32] = match digest.try_into() {
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
        Ok(value) => value,
    };

    let sig_arr: [u8; 64] = match sig_bytes.try_into() {
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
        Ok(value) => value,
    };

    match verify_canonical_payload_digest(&digest_arr, &sig_arr, &pubkey_arr) {
        Ok(()) => PaymentStreamsFfiStatus::Success,
        Err(err) => map_off_chain_err(err),
    }
}

/// Generate a 32-byte NSSA session secret and matching 32-byte public key (x-only).
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_generate_session_keypair(
    out_secret_key_32: *mut u8,
    out_public_key_32: *mut u8,
) -> PaymentStreamsFfiStatus {
    if out_secret_key_32.is_null() || out_public_key_32.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }
    let (secret, public) = generate_session_keypair();
    slice::from_raw_parts_mut(out_secret_key_32, 32).copy_from_slice(&secret);
    slice::from_raw_parts_mut(out_public_key_32, 32).copy_from_slice(&public);
    PaymentStreamsFfiStatus::Success
}

/// Write SHA-256 digest for full N8 wire bytes (`domain prefix` || Borsh body).
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_store_eligibility_canonical_payload_digest_from_n8_wire_bytes(
    n8_wire_ptr: *const u8,
    n8_wire_len: usize,
    out_digest_32: *mut u8,
) -> PaymentStreamsFfiStatus {
    if out_digest_32.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }
    let wire = match borrow_input(n8_wire_ptr, n8_wire_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };
    let digest = match store_eligibility_canonical_payload_digest_from_n8_wire(wire) {
        Ok(value) => value,
        Err(err) => return map_wire_err(err),
    };
    slice::from_raw_parts_mut(out_digest_32, 32).copy_from_slice(&digest);
    PaymentStreamsFfiStatus::Success
}

/// Build inner `StreamProof` protobuf bytes for a session key and N8 canonical wire payload.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_stream_proof_for_n8_wire(
    stream_id: u64,
    secret_key_32: *const u8,
    n8_wire_ptr: *const u8,
    n8_wire_len: usize,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    if out_len.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }
    *out_len = 0;

    let secret = match borrow_input(secret_key_32, 32) {
        Err(status) => return status,
        Ok(slice) => slice,
    };
    let secret_arr: [u8; 32] = match secret.try_into() {
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
        Ok(value) => value,
    };
    let key = match PrivateKey::try_new(secret_arr) {
        Ok(value) => value,
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
    };

    let wire = match borrow_input(n8_wire_ptr, n8_wire_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };
    let digest = match store_eligibility_canonical_payload_digest_from_n8_wire(wire) {
        Ok(value) => value,
        Err(err) => return map_wire_err(err),
    };

    let proof = StreamProofWire {
        stream_id,
        signature: sign_canonical_payload_digest(&key, &digest),
    };
    let encoded = serialize_stream_proof(&proof);
    unsafe {
        *out_len = encoded.len();
    }
    if out_ptr.is_null() {
        return PaymentStreamsFfiStatus::Success;
    }
    if out_cap < encoded.len() {
        return PaymentStreamsFfiStatus::Malformed;
    }
    slice::from_raw_parts_mut(out_ptr, encoded.len()).copy_from_slice(&encoded);
    PaymentStreamsFfiStatus::Success
}

/// Parse outer protobuf `EligibilityProof` (exactly one arm).
///
/// `out_arm`: `0` = `stream_proposal`, `1` = `stream_proof`.
///
/// # Safety
///
/// `(data_ptr, data_len)` readable; `out_arm` and `inner_out_len` non-null when sizing or copying inner bytes.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_parse_eligibility_proof_bytes(
    data_ptr: *const u8,
    data_len: usize,
    out_arm: *mut u32,
    inner_out_ptr: *mut u8,
    inner_out_cap: usize,
    inner_out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    if out_arm.is_null() || inner_out_len.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }
    *inner_out_len = 0;

    let bytes = match borrow_input(data_ptr, data_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let parsed = match parse_eligibility_proof(bytes) {
        Err(err) => return map_wire_err(err),
        Ok(value) => value,
    };

    let (arm_code, inner): (u32, &[u8]) = match &parsed {
        EligibilityProofWire::StreamProposal(bytes) => (0, bytes.as_slice()),
        EligibilityProofWire::StreamProof(bytes) => (1, bytes.as_slice()),
    };

    *out_arm = arm_code;
    *inner_out_len = inner.len();

    if inner_out_ptr.is_null() {
        return PaymentStreamsFfiStatus::Success;
    }
    if inner_out_cap < inner.len() {
        return PaymentStreamsFfiStatus::Malformed;
    }
    slice::from_raw_parts_mut(inner_out_ptr, inner.len()).copy_from_slice(inner);
    PaymentStreamsFfiStatus::Success
}

/// Verify `StreamProof.signature` over the N8 canonical Store request wire.
///
/// # Safety
///
/// All pointers follow [`borrow_input`] rules; session key is 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_verify_stream_proof_for_n8_wire_bytes(
    proof_ptr: *const u8,
    proof_len: usize,
    session_public_key: *const u8,
    n8_wire_ptr: *const u8,
    n8_wire_len: usize,
) -> PaymentStreamsFfiStatus {
    let proof_bytes = match borrow_input(proof_ptr, proof_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };
    let session_bytes = match borrow_input(session_public_key, 32) {
        Err(status) => return status,
        Ok(slice) => slice,
    };
    let n8_wire = match borrow_input(n8_wire_ptr, n8_wire_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };

    let session_arr: [u8; 32] = match session_bytes.try_into() {
        Err(_) => return PaymentStreamsFfiStatus::Malformed,
        Ok(value) => value,
    };

    let proof_wire = match parse_stream_proof(proof_bytes) {
        Err(err) => return map_wire_err(err),
        Ok(value) => value,
    };

    let digest = match store_eligibility_canonical_payload_digest_from_n8_wire(n8_wire) {
        Ok(value) => value,
        Err(err) => return map_wire_err(err),
    };

    match verify_canonical_payload_digest(&digest, &proof_wire.signature, &session_arr) {
        Ok(()) => PaymentStreamsFfiStatus::Success,
        Err(err) => map_off_chain_err(err),
    }
}

fn write_eligibility_wrapper(
    arm: &EligibilityProofWire,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    if out_len.is_null() {
        return PaymentStreamsFfiStatus::NullPointer;
    }
    unsafe {
        *out_len = 0;
    }
    let encoded = serialize_eligibility_proof(arm);
    unsafe {
        *out_len = encoded.len();
    }
    if out_ptr.is_null() {
        return PaymentStreamsFfiStatus::Success;
    }
    if out_cap < encoded.len() {
        return PaymentStreamsFfiStatus::Malformed;
    }
    unsafe {
        slice::from_raw_parts_mut(out_ptr, encoded.len()).copy_from_slice(&encoded);
    }
    PaymentStreamsFfiStatus::Success
}

/// Serialize `EligibilityProof { stream_proposal: inner }`.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_eligibility_proof_stream_proposal_bytes(
    inner_proposal_ptr: *const u8,
    inner_proposal_len: usize,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let inner = match borrow_input(inner_proposal_ptr, inner_proposal_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };
    write_eligibility_wrapper(
        &EligibilityProofWire::StreamProposal(inner.to_vec()),
        out_ptr,
        out_cap,
        out_len,
    )
}

/// Serialize `EligibilityProof { stream_proof: inner }`.
#[no_mangle]
pub unsafe extern "C" fn payment_streams_ffi_serialize_eligibility_proof_stream_proof_bytes(
    inner_proof_ptr: *const u8,
    inner_proof_len: usize,
    out_ptr: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> PaymentStreamsFfiStatus {
    let inner = match borrow_input(inner_proof_ptr, inner_proof_len) {
        Err(status) => return status,
        Ok(slice) => slice,
    };
    write_eligibility_wrapper(
        &EligibilityProofWire::StreamProof(inner.to_vec()),
        out_ptr,
        out_cap,
        out_len,
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "FFI proof_abi tests use known-good inputs"
)]
mod tests {
    use super::*;
    use lez_payment_streams_core::{
        generate_session_keypair, parse_eligibility_proof, sign_stream_proof_for_store_query,
        sign_stream_proposal_vault_proof, StreamParams, VaultProofWire,
    };
    use lee::PublicKey;

    #[test]
    fn ffi_round_trips_proposal_and_verifies_vault_proof() {
        let owner_key = PrivateKey::new_os_random();
        let owner_account = AccountId::from(&PublicKey::new_from_private_key(&owner_key));

        let proposal_core = StreamProposalWire {
            vault: VaultProofWire {
                vault_id: 2,
                provider_id: [5_u8; 32],
                owner_public_key: [0_u8; 32],
                owner_signature: [0_u8; 64],
            },
            params: StreamParams::new(12, 800, 1500, b"/demo/service".to_vec()),
            session_public_key: [9_u8; 32],
        };

        let signed = sign_stream_proposal_vault_proof(proposal_core, &owner_key).expect("signs");
        let bytes = serialize_stream_proposal(&signed).expect("serializes");
        let mut ffi_decoded: PaymentStreamsFfiDecodedStreamProposal =
            unsafe { core::mem::zeroed() };

        assert_eq!(
            unsafe {
                payment_streams_ffi_parse_stream_proposal_bytes(
                    bytes.as_ptr(),
                    bytes.len(),
                    &mut ffi_decoded,
                )
            },
            PaymentStreamsFfiStatus::Success
        );

        assert_eq!(
            unsafe {
                payment_streams_ffi_verify_stream_proposal_vault_proof_bytes(
                    bytes.as_ptr(),
                    bytes.len(),
                    owner_account.value().as_ptr(),
                )
            },
            PaymentStreamsFfiStatus::Success
        );

        let mut out_len = 0_usize;
        assert_eq!(
            unsafe {
                payment_streams_ffi_serialize_stream_proposal_bytes(
                    &ffi_decoded,
                    core::ptr::null_mut(),
                    0,
                    &mut out_len,
                )
            },
            PaymentStreamsFfiStatus::Success
        );
        assert_eq!(out_len, bytes.len());

        let mut scratch = vec![0_u8; bytes.len()];
        assert_eq!(
            unsafe {
                payment_streams_ffi_serialize_stream_proposal_bytes(
                    &ffi_decoded,
                    scratch.as_mut_ptr(),
                    scratch.len(),
                    &mut out_len,
                )
            },
            PaymentStreamsFfiStatus::Success
        );
        assert_eq!(scratch, bytes);
    }

    #[test]
    fn ffi_store_eligibility_round_trip() {
        let session_key = PrivateKey::new_os_random();
        let session_pk = *PublicKey::new_from_private_key(&session_key).value();

        let store = PaymentStreamsFfiCanonicalStoreQuery {
            request_id: PaymentStreamsFfiByteSpan {
                ptr: b"rid".as_ptr(),
                len: 3,
            },
            include_data: 0,
            has_pubsub_topic: 0,
            pubsub_topic: PaymentStreamsFfiByteSpan {
                ptr: core::ptr::null(),
                len: 0,
            },
            content_topics: core::ptr::null(),
            content_topics_len: 0,
            has_start_time: 0,
            start_time: 0,
            has_end_time: 0,
            end_time: 0,
            message_hashes: core::ptr::null(),
            message_hashes_len: 0,
            has_pagination_cursor: 0,
            pagination_cursor: [0_u8; 32],
            pagination_forward: 0,
            has_pagination_limit: 0,
            pagination_limit: 0,
        };

        let mut canonical_payload_digest = [0_u8; 32];
        assert_eq!(
            unsafe {
                payment_streams_ffi_store_eligibility_canonical_payload_digest(
                    &store,
                    canonical_payload_digest.as_mut_ptr(),
                )
            },
            PaymentStreamsFfiStatus::Success
        );

        let empty_topics: Vec<String> = Vec::new();
        let parts = CanonicalStoreQueryParts {
            request_id: "rid",
            include_data: false,
            pubsub_topic: None,
            content_topics: &empty_topics,
            start_time: None,
            end_time: None,
            message_hashes: &[],
            pagination_cursor: None,
            pagination_forward: false,
            pagination_limit: None,
        };

        assert_eq!(
            canonical_payload_digest,
            lez_payment_streams_core::store_eligibility_canonical_payload_digest(&parts)
        );

        let checked_proof = sign_stream_proof_for_store_query(44, &session_key, &parts);
        let proof_bytes = serialize_stream_proof(&checked_proof);

        assert_eq!(
            unsafe {
                payment_streams_ffi_verify_stream_proof_for_store_query(
                    proof_bytes.as_ptr(),
                    proof_bytes.len(),
                    session_pk.as_ptr(),
                    &store,
                )
            },
            PaymentStreamsFfiStatus::Success
        );
    }

    #[test]
    fn ffi_generate_session_keypair_sign_verify_round_trip() {
        let mut secret = [0_u8; 32];
        let mut public = [0_u8; 32];
        assert_eq!(
            unsafe {
                payment_streams_ffi_generate_session_keypair(
                    secret.as_mut_ptr(),
                    public.as_mut_ptr(),
                )
            },
            PaymentStreamsFfiStatus::Success
        );
        let digest = [7_u8; 32];
        let mut sig = [0_u8; 64];
        assert_eq!(
            unsafe {
                payment_streams_ffi_sign_canonical_payload_digest(
                    secret.as_ptr(),
                    digest.as_ptr(),
                    sig.as_mut_ptr(),
                )
            },
            PaymentStreamsFfiStatus::Success
        );
        assert_eq!(
            unsafe {
                payment_streams_ffi_verify_canonical_payload_digest(
                    public.as_ptr(),
                    digest.as_ptr(),
                    sig.as_ptr(),
                )
            },
            PaymentStreamsFfiStatus::Success
        );
    }

    #[test]
    fn ffi_eligibility_proof_wrapper_round_trip() {
        let inner = b"inner-proposal-bytes".to_vec();
        let mut out_len = 0_usize;
        let mut buf = vec![0_u8; 256];
        assert_eq!(
            unsafe {
                payment_streams_ffi_serialize_eligibility_proof_stream_proposal_bytes(
                    inner.as_ptr(),
                    inner.len(),
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut out_len,
                )
            },
            PaymentStreamsFfiStatus::Success
        );
        buf.truncate(out_len);
        let parsed = parse_eligibility_proof(&buf).expect("parses");
        match parsed {
            EligibilityProofWire::StreamProposal(bytes) => assert_eq!(bytes, inner),
            _ => panic!("expected proposal arm"),
        }
    }

    #[test]
    fn ffi_parse_eligibility_proof_bytes_round_trip() {
        let inner = b"inner-proof-bytes".to_vec();
        let wrapped = serialize_eligibility_proof(&EligibilityProofWire::StreamProof(inner.clone()));
        let mut arm = 0_u32;
        let mut inner_len = 0_usize;
        let mut out = vec![0_u8; 64];
        assert_eq!(
            unsafe {
                payment_streams_ffi_parse_eligibility_proof_bytes(
                    wrapped.as_ptr(),
                    wrapped.len(),
                    &mut arm,
                    out.as_mut_ptr(),
                    out.len(),
                    &mut inner_len,
                )
            },
            PaymentStreamsFfiStatus::Success
        );
        assert_eq!(arm, 1);
        assert_eq!(&out[..inner_len], inner.as_slice());
    }
}
