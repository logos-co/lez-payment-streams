//! LEZ width caps and 32-byte domain prefixes for canonical payload digests (Borsh canonical payload + SHA-256).

/// `VaultProof.vault_id` on the wire is eight little-endian bytes (see LIP‑155 LEZ integration).
pub const LEZ_VAULT_ID_WIRE_LEN: usize = 8;
/// `StreamProof.stream_id` uses the same encoding as `VaultId` on LEZ demo nets.
pub const LEZ_STREAM_ID_WIRE_LEN: usize = 8;
/// NSSA public-account keys and LEZ `AccountId`s use 32 raw bytes.
pub const LEZ_ACCOUNT_RAW_LEN: usize = 32;
/// NSSA Schnorr signatures are 64 raw bytes (BIP-340 encoding).
pub const LEZ_SCHNORR_SIGNATURE_LEN: usize = 64;

/// 32-byte domain separation prefix hashed together with a Borsh body (integration plan N8 / LIP‑155 LEZ).
pub const STORE_ELIGIBILITY_DOMAIN_PREFIX: [u8; 32] =
    *b"/LEZ/v0.1/StoreEligibility/\x00\x00\x00\x00\x00";

/// Domain prefix for `VaultProof.owner_signature` over vault fields, proposal terms, and session key.
pub const VAULT_OWNER_AUTH_DOMAIN_PREFIX: [u8; 32] =
    *b"/LEZ/v0.1/VaultOwnerAuth/\x00\x00\x00\x00\x00\x00\x00";
