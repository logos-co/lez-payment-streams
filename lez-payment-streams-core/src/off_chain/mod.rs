//! LIP‑155 off-chain wire helpers (protobuf framing + canonical payloads / digests + Schnorr).

mod canonical;
mod constants;
mod proofs;
mod protobuf;
mod wire_error;

pub use canonical::{
    n8_reference_store_eligibility_wire, n8_reference_store_query_parts,
    store_eligibility_canonical_payload, store_eligibility_canonical_payload_digest,
    store_eligibility_canonical_payload_digest_from_n8_wire,
    vault_owner_auth_canonical_payload, vault_owner_auth_canonical_payload_digest,
    CanonicalStoreQueryParts, N8_REFERENCE_CONTENT_TOPIC, N8_REFERENCE_PUBSUB_TOPIC,
    N8_REFERENCE_STORE_ELIGIBILITY_WIRE_LEN, VaultOwnerAuthCanonicalError,
};
pub use constants::{
    LEZ_ACCOUNT_RAW_LEN, LEZ_SCHNORR_SIGNATURE_LEN, LEZ_STREAM_ID_WIRE_LEN, LEZ_VAULT_ID_WIRE_LEN,
    STORE_ELIGIBILITY_DOMAIN_PREFIX, VAULT_OWNER_AUTH_DOMAIN_PREFIX,
};
pub use proofs::{
    generate_session_keypair, owner_public_key_matches_vault_owner, sign_canonical_payload_digest,
    sign_stream_proof_for_store_query, sign_stream_proposal_vault_proof,
    verify_canonical_payload_digest, verify_stream_proof_for_store_query,
    verify_stream_proposal_vault_proof, verify_stream_proposal_vault_signature,
};
pub use protobuf::{
    parse_eligibility_proof, parse_stream_params, parse_stream_proof, parse_stream_proposal,
    parse_vault_proof, serialize_eligibility_proof, serialize_stream_params,
    serialize_stream_proof, serialize_stream_proposal, EligibilityProofWire, StreamProofWire,
    StreamProposalWire, VaultProofWire,
};
pub use wire_error::{OffChainError, WireError};
