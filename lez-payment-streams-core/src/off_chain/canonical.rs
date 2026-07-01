//! Canonical payloads (Borsh bytes) and SHA-256 canonical payload digests for LEZ NSSA Schnorr (Step 4 / N8).

use borsh::BorshSerialize;
use sha2::{Digest as _, Sha256};

use crate::stream_provider_policy::StreamParams;

use super::constants::{STORE_ELIGIBILITY_DOMAIN_PREFIX, VAULT_OWNER_AUTH_DOMAIN_PREFIX};

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

/// Byte length of the pinned N8 cross-language reference wire (32-byte prefix + Borsh body).
pub const N8_REFERENCE_STORE_ELIGIBILITY_WIRE_LEN: usize = 138;

pub const N8_REFERENCE_CONTENT_TOPIC: &str = "/lez-payment-streams/1/e2e-eligibility/proto";
pub const N8_REFERENCE_PUBSUB_TOPIC: &str = "/waku/2/rs/0/1";

/// Pinned Store query fields for Step 15 / Step 17 Nim–Rust parity (N8).
pub fn n8_reference_store_query_parts<'a>(
    content_topics: &'a [String],
) -> CanonicalStoreQueryParts<'a> {
    CanonicalStoreQueryParts {
        request_id: "req-1",
        include_data: true,
        pubsub_topic: Some(N8_REFERENCE_PUBSUB_TOPIC),
        content_topics,
        start_time: Some(10),
        end_time: None,
        message_hashes: &[],
        pagination_cursor: None,
        pagination_forward: true,
        pagination_limit: Some(100),
    }
}

/// Full N8 reference wire: domain prefix || Borsh(`CanonicalStoreRequest`) for [`n8_reference_store_query_parts`].
pub fn n8_reference_store_eligibility_wire() -> Vec<u8> {
    let topics = vec![N8_REFERENCE_CONTENT_TOPIC.to_string()];
    let parts = n8_reference_store_query_parts(&topics);
    let body = store_eligibility_canonical_payload(&parts);
    let mut wire = STORE_ELIGIBILITY_DOMAIN_PREFIX.to_vec();
    wire.extend_from_slice(&body);
    wire
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
pub fn store_eligibility_canonical_payload_digest(
    parts: &CanonicalStoreQueryParts<'_>,
) -> [u8; 32] {
    let canonical_payload = store_eligibility_canonical_payload(parts);
    lez_canonical_payload_digest(&STORE_ELIGIBILITY_DOMAIN_PREFIX, &canonical_payload)
}

/// Digest from the full N8 wire bytes (`STORE_ELIGIBILITY_DOMAIN_PREFIX` || Borsh body).
pub fn store_eligibility_canonical_payload_digest_from_n8_wire(
    wire: &[u8],
) -> Result<[u8; 32], super::wire_error::WireError> {
    let prefix_len = STORE_ELIGIBILITY_DOMAIN_PREFIX.len();
    // Reject wires shorter than the prefix or with a mismatched prefix. The
    // `get(..prefix_len)` / `get(prefix_len..)` calls below are infallible
    // after this length check, but use the checked form to keep the slicing
    // lint satisfied.
    let head = wire.get(..prefix_len);
    let body = wire.get(prefix_len..);
    match (head, body) {
        (Some(h), Some(b)) if h == STORE_ELIGIBILITY_DOMAIN_PREFIX.as_slice() => {
            Ok(lez_canonical_payload_digest(&STORE_ELIGIBILITY_DOMAIN_PREFIX, b))
        }
        _ => Err(super::wire_error::WireError::InvalidWireFrame),
    }
}

/// Build the vault-owner canonical payload bytes covered by `VaultProof.owner_signature`.
pub fn vault_owner_auth_canonical_payload(
    vault_id: u64,
    provider_id: &[u8; 32],
    owner_public_key: &[u8; 32],
    params: &StreamParams,
    session_public_key: &[u8; 32],
) -> Result<Vec<u8>, VaultOwnerAuthCanonicalError> {
    let service_id = String::from_utf8(params.service_id.clone())
        .map_err(|_| VaultOwnerAuthCanonicalError::InvalidServiceIdUtf8)?;

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
        let topics = vec![N8_REFERENCE_CONTENT_TOPIC.to_string()];
        let parts = n8_reference_store_query_parts(&topics);

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
        let err =
            vault_owner_auth_canonical_payload(9, &[7_u8; 32], &[8_u8; 32], &params, &[3_u8; 32])
                .expect_err("invalid utf-8 service id must fail");
        assert!(matches!(
            err,
            VaultOwnerAuthCanonicalError::InvalidServiceIdUtf8
        ));
    }

    #[test]
    fn store_eligibility_digest_matches_n8_reference_fixture() {
        let topics = vec![N8_REFERENCE_CONTENT_TOPIC.to_string()];
        let parts = n8_reference_store_query_parts(&topics);
        let digest = store_eligibility_canonical_payload_digest(&parts);
        assert_eq!(
            digest,
            [
                65, 191, 241, 41, 255, 102, 85, 126, 128, 47, 231, 66, 240, 218, 71, 20, 131,
                131, 208, 86, 116, 168, 252, 230, 30, 83, 237, 26, 164, 157, 140, 56,
            ]
        );
        let wire = n8_reference_store_eligibility_wire();
        assert_eq!(wire.len(), N8_REFERENCE_STORE_ELIGIBILITY_WIRE_LEN);
    }
}
