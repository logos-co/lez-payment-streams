//! Canonical payloads (Borsh bytes) and SHA-256 canonical payload digests for LEZ NSSA Schnorr (Step 4 / N8).

use borsh::BorshSerialize;
use sha2::{Digest as _, Sha256};

use crate::stream_provider_policy::StreamParams;

use super::constants::{
    STORE_ELIGIBILITY_DOMAIN_PREFIX, VAULT_OWNER_AUTH_DOMAIN_PREFIX,
};

/// Failed to serialize vault-owner authorization payload (Borsh over LIP‑155 fields).
#[derive(Debug)]
pub enum VaultOwnerAuthCanonicalError {
    /// `StreamParams.service_id` is not valid UTF‑8 (required for the Borsh `string` field).
    InvalidServiceIdUtf8,
    Borsh(std::io::Error),
}

/// Borsh body for vault-owner authorization (`VaultProof.owner_signature` covers the resulting canonical payload).
#[derive(BorshSerialize)]
struct VaultOwnerAuthBorshBody {
    vault_id: u64,
    provider_id: [u8; 32],
    owner_public_key: [u8; 32],
    service_id: String,
    rate: u64,
    allocation: u128,
    create_stream_deadline: u64,
    session_public_key: [u8; 32],
}

/// SHA-256(`domain_prefix` ‖ `canonical_payload`), matching the NSSA public-account digest pattern.
fn lez_canonical_payload_digest(domain: &[u8; 32], canonical_payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(canonical_payload);
    hasher.finalize().into()
}

fn push_borsh_string(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_optional_string(out: &mut Vec<u8>, presence: u8, value: Option<&str>) {
    out.push(presence);
    if let Some(selected) = value {
        push_borsh_string(out, selected);
    }
}

fn push_optional_i64(out: &mut Vec<u8>, presence: u8, value: Option<i64>) {
    out.push(presence);
    if let Some(selected) = value {
        out.extend_from_slice(&selected.to_le_bytes());
    }
}

fn push_optional_u64(out: &mut Vec<u8>, presence: u8, value: Option<u64>) {
    out.push(presence);
    if let Some(selected) = value {
        out.extend_from_slice(&selected.to_le_bytes());
    }
}

/// Raw pieces of a Store query that Nim and Rust must serialize identically (integration plan N8).
pub struct CanonicalStoreQueryParts<'a> {
    pub request_id: &'a str,
    pub include_data: bool,
    pub pubsub_topic: Option<&'a str>,
    pub content_topics: &'a [String],
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub message_hashes: &'a [[u8; 32]],
    pub pagination_cursor: Option<[u8; 32]>,
    pub pagination_forward: bool,
    pub pagination_limit: Option<u64>,
}

/// Build the Store eligibility canonical payload (Borsh body; domain prefix is hashed separately).
pub fn store_eligibility_canonical_payload(parts: &CanonicalStoreQueryParts<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    push_borsh_string(&mut out, parts.request_id);
    out.push(u8::from(parts.include_data));

    let has_pubsub = u8::from(parts.pubsub_topic.is_some());
    push_optional_string(&mut out, has_pubsub, parts.pubsub_topic);

    out.extend_from_slice(&(parts.content_topics.len() as u32).to_le_bytes());
    for topic in parts.content_topics {
        push_borsh_string(&mut out, topic.as_str());
    }

    let has_start_time = u8::from(parts.start_time.is_some());
    push_optional_i64(&mut out, has_start_time, parts.start_time);

    let has_end_time = u8::from(parts.end_time.is_some());
    push_optional_i64(&mut out, has_end_time, parts.end_time);

    out.extend_from_slice(&(parts.message_hashes.len() as u32).to_le_bytes());
    for hash in parts.message_hashes {
        out.extend_from_slice(hash);
    }

    let has_cursor = u8::from(parts.pagination_cursor.is_some());
    out.push(has_cursor);
    if let Some(cursor) = parts.pagination_cursor {
        out.extend_from_slice(&cursor);
    }

    out.push(u8::from(parts.pagination_forward));

    let has_limit = u8::from(parts.pagination_limit.is_some());
    push_optional_u64(&mut out, has_limit, parts.pagination_limit);

    out
}

/// 32-byte canonical payload digest signed by `StreamProof.signature` for Store eligibility.
pub fn store_eligibility_canonical_payload_digest(parts: &CanonicalStoreQueryParts<'_>) -> [u8; 32] {
    let canonical_payload = store_eligibility_canonical_payload(parts);
    lez_canonical_payload_digest(&STORE_ELIGIBILITY_DOMAIN_PREFIX, &canonical_payload)
}

/// Build the vault-owner canonical payload bytes covered by `VaultProof.owner_signature`.
pub fn vault_owner_auth_canonical_payload(
    vault_id: u64,
    provider_id: &[u8; 32],
    owner_public_key: &[u8; 32],
    params: &StreamParams,
    session_public_key: &[u8; 32],
) -> Result<Vec<u8>, VaultOwnerAuthCanonicalError> {
    let service_id = String::from_utf8(params.service_id.clone()).map_err(|_| {
        VaultOwnerAuthCanonicalError::InvalidServiceIdUtf8
    })?;

    let body = VaultOwnerAuthBorshBody {
        vault_id,
        provider_id: *provider_id,
        owner_public_key: *owner_public_key,
        service_id,
        rate: params.rate,
        allocation: params.allocation,
        create_stream_deadline: params.create_stream_deadline,
        session_public_key: *session_public_key,
    };

    borsh::to_vec(&body).map_err(VaultOwnerAuthCanonicalError::Borsh)
}

/// 32-byte canonical payload digest signed by `VaultProof.owner_signature`.
pub fn vault_owner_auth_canonical_payload_digest(
    vault_id: u64,
    provider_id: &[u8; 32],
    owner_public_key: &[u8; 32],
    params: &StreamParams,
    session_public_key: &[u8; 32],
) -> Result<[u8; 32], VaultOwnerAuthCanonicalError> {
    let canonical_payload = vault_owner_auth_canonical_payload(
        vault_id,
        provider_id,
        owner_public_key,
        params,
        session_public_key,
    )?;
    Ok(lez_canonical_payload_digest(
        &VAULT_OWNER_AUTH_DOMAIN_PREFIX,
        &canonical_payload,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream_provider_policy::StreamParams;

    #[test]
    fn canonical_store_query_matches_integration_plan_vector_shape() {
        let hashes = [[1_u8; 32], [2_u8; 32]];
        let topics = vec!["/my-app/1/chat/proto".to_string()];
        let parts = CanonicalStoreQueryParts {
            request_id: "req-1",
            include_data: true,
            pubsub_topic: Some("/waku/2/topic"),
            content_topics: &topics,
            start_time: Some(10),
            end_time: None,
            message_hashes: &hashes,
            pagination_cursor: None,
            pagination_forward: true,
            pagination_limit: Some(100),
        };

        let bytes = store_eligibility_canonical_payload(&parts);
        assert!(
            bytes.len() > 32,
            "vector-shaped encoding should not collapse to a trivial slice"
        );

        let first = store_eligibility_canonical_payload_digest(&parts);
        let second = store_eligibility_canonical_payload_digest(&parts);
        assert_eq!(
            first, second,
            "canonical payload digest must be deterministic for the same canonical payload"
        );
    }

    #[test]
    fn vault_owner_payload_rejects_non_utf8_service_id() {
        let params = StreamParams::new(1, 2, 3, vec![0xFF, 0xFE, 0xFD]);
        let err = vault_owner_auth_canonical_payload(9, &[7_u8; 32], &[8_u8; 32], &params, &[3_u8; 32])
            .expect_err("invalid utf-8 service id must fail");
        assert!(matches!(err, VaultOwnerAuthCanonicalError::InvalidServiceIdUtf8));
    }

    #[test]
    fn store_eligibility_digest_matches_n8_reference_fixture() {
        let hashes = [[1_u8; 32], [2_u8; 32]];
        let topics = vec!["/my-app/1/chat/proto".to_string()];
        let parts = CanonicalStoreQueryParts {
            request_id: "req-1",
            include_data: true,
            pubsub_topic: Some("/waku/2/topic"),
            content_topics: &topics,
            start_time: Some(10),
            end_time: None,
            message_hashes: &hashes,
            pagination_cursor: None,
            pagination_forward: true,
            pagination_limit: Some(100),
        };
        let digest = store_eligibility_canonical_payload_digest(&parts);
        // Rust-computed digest for the fixture above (integration plan N8 shape). When Nim
        // `liblogosdelivery` publishes the same inputs, re-check and align this literal.
        assert_eq!(
            digest,
            [
                53, 238, 26, 182, 83, 10, 132, 140, 241, 208, 236, 55, 89, 13, 57, 202, 251, 119, 44,
                172, 99, 161, 112, 250, 114, 37, 177, 149, 230, 133, 233, 166,
            ]
        );
    }
}

