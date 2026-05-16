//! NSSA Schnorr verification helpers for LEZ payment-stream proofs (LIP‑155 Step 4).

use nssa::{PrivateKey, PublicKey, Signature};
use nssa_core::account::AccountId;

use super::canonical::{
    store_eligibility_canonical_payload_digest, vault_owner_auth_canonical_payload_digest,
    CanonicalStoreQueryParts,
};
use super::protobuf::{StreamProofWire, StreamProposalWire};
use super::wire_error::OffChainError;

/// Verifies a Schnorr signature over a 32-byte `canonical_payload_digest` (NSSA / BIP-340 “message” input).
pub fn verify_canonical_payload_digest(
    canonical_payload_digest: &[u8; 32],
    signature: &[u8; 64],
    public_key: &[u8; 32],
) -> Result<(), OffChainError> {
    let pk = PublicKey::try_new(*public_key).map_err(|_| OffChainError::InvalidPublicKey)?;
    let sig = Signature { value: *signature };
    if sig.is_valid_for(canonical_payload_digest.as_slice(), &pk) {
        Ok(())
    } else {
        Err(OffChainError::BadSignature)
    }
}

/// Signs `canonical_payload_digest` with an NSSA private key (non-deterministic aux randomness).
pub fn sign_canonical_payload_digest(
    private_key: &PrivateKey,
    canonical_payload_digest: &[u8; 32],
) -> [u8; 64] {
    Signature::new(private_key, canonical_payload_digest.as_slice()).value
}

/// Confirms `owner_public_key` derives to the `vault_owner` anchor stored in `VaultConfig`.
pub fn owner_public_key_matches_vault_owner(
    owner_public_key: &[u8; 32],
    vault_owner: &AccountId,
) -> Result<(), OffChainError> {
    let pk = PublicKey::try_new(*owner_public_key).map_err(|_| OffChainError::InvalidPublicKey)?;
    let derived = AccountId::from(&pk);
    if derived == *vault_owner {
        Ok(())
    } else {
        Err(OffChainError::OwnerKeyMismatch)
    }
}

/// Verifies `VaultProof.owner_signature` over the canonical vault-owner payload digest.
pub fn verify_stream_proposal_vault_signature(proposal: &StreamProposalWire) -> Result<(), OffChainError> {
    let canonical_payload_digest = vault_owner_auth_canonical_payload_digest(
        proposal.vault.vault_id,
        &proposal.vault.provider_id,
        &proposal.vault.owner_public_key,
        &proposal.params,
        &proposal.session_public_key,
    )
    .map_err(OffChainError::from)?;

    verify_canonical_payload_digest(
        &canonical_payload_digest,
        &proposal.vault.owner_signature,
        &proposal.vault.owner_public_key,
    )
}

/// Verifies LEZ vault-owner binding + owner signature for an already-decoded proposal.
pub fn verify_stream_proposal_vault_proof(
    proposal: &StreamProposalWire,
    vault_owner: &AccountId,
) -> Result<(), OffChainError> {
    owner_public_key_matches_vault_owner(&proposal.vault.owner_public_key, vault_owner)?;
    verify_stream_proposal_vault_signature(proposal)
}

/// Verifies `StreamProof.signature` over the Store eligibility canonical payload digest for a session key.
pub fn verify_stream_proof_for_store_query(
    stream_proof: &StreamProofWire,
    session_public_key: &[u8; 32],
    store: &CanonicalStoreQueryParts<'_>,
) -> Result<(), OffChainError> {
    let canonical_payload_digest = store_eligibility_canonical_payload_digest(store);
    verify_canonical_payload_digest(
        &canonical_payload_digest,
        &stream_proof.signature,
        session_public_key,
    )
}

/// Builds a deterministic `VaultProofWire` by signing the vault-owner canonical payload digest (tests + module helper).
pub fn sign_stream_proposal_vault_proof(
    mut proposal: StreamProposalWire,
    owner_private_key: &PrivateKey,
) -> Result<StreamProposalWire, OffChainError> {
    let owner_public_key = *PublicKey::new_from_private_key(owner_private_key).value();
    proposal.vault.owner_public_key = owner_public_key;

    let canonical_payload_digest = vault_owner_auth_canonical_payload_digest(
        proposal.vault.vault_id,
        &proposal.vault.provider_id,
        &proposal.vault.owner_public_key,
        &proposal.params,
        &proposal.session_public_key,
    )
    .map_err(OffChainError::from)?;

    proposal.vault.owner_signature =
        sign_canonical_payload_digest(owner_private_key, &canonical_payload_digest);
    Ok(proposal)
}

/// Builds a `StreamProofWire` for a store query using the session private key (paired with proposal public key).
pub fn sign_stream_proof_for_store_query(
    stream_id: u64,
    session_private_key: &PrivateKey,
    store: &CanonicalStoreQueryParts<'_>,
) -> StreamProofWire {
    let canonical_payload_digest = store_eligibility_canonical_payload_digest(store);
    StreamProofWire {
        stream_id,
        signature: sign_canonical_payload_digest(session_private_key, &canonical_payload_digest),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::off_chain::protobuf::{self, StreamProposalWire, VaultProofWire};
    use crate::stream_provider_policy::StreamParams;

    #[test]
    fn vault_proof_sign_verify_and_owner_binding() {
        let owner_key = PrivateKey::new_os_random();
        let owner_account = AccountId::from(&PublicKey::new_from_private_key(&owner_key));

        let params = StreamParams::new(10, 500, 1200, b"/vac/waku/store-query/3.0.0".to_vec());
        let proposal = StreamProposalWire {
            vault: VaultProofWire {
                vault_id: 7,
                provider_id: [5_u8; 32],
                owner_public_key: [0_u8; 32],
                owner_signature: [0_u8; 64],
            },
            params,
            session_public_key: [4_u8; 32],
        };

        let signed = sign_stream_proposal_vault_proof(proposal, &owner_key).expect("signs");
        verify_stream_proposal_vault_proof(&signed, &owner_account).expect("verifies");

        let mut tampered_rate = signed.clone();
        tampered_rate.params.rate = signed.params.rate.saturating_add(1);
        assert!(matches!(
            verify_stream_proposal_vault_signature(&tampered_rate),
            Err(OffChainError::BadSignature)
        ));

        let mut tampered_sessions = signed.clone();
        tampered_sessions.session_public_key = [3_u8; 32];
        assert!(matches!(
            verify_stream_proposal_vault_signature(&tampered_sessions),
            Err(OffChainError::BadSignature)
        ));

        let other_owner = AccountId::new([9_u8; 32]);
        assert!(matches!(
            verify_stream_proposal_vault_proof(&signed, &other_owner),
            Err(OffChainError::OwnerKeyMismatch)
        ));
    }

    #[test]
    fn stream_proof_sign_verify_detects_tampered_store_payload() {
        let session_key = PrivateKey::new_os_random();
        let session_pk = *PublicKey::new_from_private_key(&session_key).value();

        let empty_topics: Vec<String> = Vec::new();
        let store = CanonicalStoreQueryParts {
            request_id: "abc",
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

        let proof = sign_stream_proof_for_store_query(99, &session_key, &store);
        verify_stream_proof_for_store_query(&proof, &session_pk, &store).expect("verifies");

        let empty_topics_tampered: Vec<String> = Vec::new();
        let tampered_store = CanonicalStoreQueryParts {
            request_id: "abcd",
            include_data: false,
            pubsub_topic: None,
            content_topics: &empty_topics_tampered,
            start_time: None,
            end_time: None,
            message_hashes: &[],
            pagination_cursor: None,
            pagination_forward: false,
            pagination_limit: None,
        };

        assert!(matches!(
            verify_stream_proof_for_store_query(&proof, &session_pk, &tampered_store),
            Err(OffChainError::BadSignature)
        ));
    }

    #[test]
    fn protobuf_round_trip_then_signature_still_checks_out() {
        let owner_key = PrivateKey::new_os_random();
        let owner_account = AccountId::from(&PublicKey::new_from_private_key(&owner_key));

        let params = StreamParams::new(11, 600, 1300, b"/demo/service-id".to_vec());
        let proposal = StreamProposalWire {
            vault: VaultProofWire {
                vault_id: 3,
                provider_id: [2_u8; 32],
                owner_public_key: [0_u8; 32],
                owner_signature: [0_u8; 64],
            },
            params,
            session_public_key: [6_u8; 32],
        };

        let signed = sign_stream_proposal_vault_proof(proposal, &owner_key).expect("signs");
        let bytes = protobuf::serialize_stream_proposal(&signed).expect("serializes");
        let decoded = protobuf::parse_stream_proposal(&bytes).expect("parses");

        verify_stream_proposal_vault_proof(&decoded, &owner_account).expect("still verifies");
    }
}
