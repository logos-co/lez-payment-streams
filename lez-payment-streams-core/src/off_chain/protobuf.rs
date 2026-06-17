//! Minimal protobuf encode/decode for LIP‑155 `StreamProposal`, `VaultProof`, `StreamProof`, and nested
//! `StreamParams` (field numbers only; unknown fields are skipped while decoding).
//!
//! Implementation is hand-rolled (varint + length-delimited only) to avoid a full `prost` stack and to
//! apply LEZ `bytes` width checks in one place; field numbers must stay aligned with
//! `rfc-index/docs/ift-ts/raw/payment-streams.md` (see module `field_numbers`).

use nssa_core::account::Balance;

use crate::stream_provider_policy::{StreamParams, MAX_SERVICE_ID_LEN};
use crate::{Timestamp, TokensPerSecond};

use super::constants::{
    LEZ_ACCOUNT_RAW_LEN, LEZ_SCHNORR_SIGNATURE_LEN, LEZ_STREAM_ID_WIRE_LEN, LEZ_VAULT_ID_WIRE_LEN,
};
use super::wire_error::WireError;

const WIRE_VARINT: u32 = 0;
const WIRE_LEN_DELIM: u32 = 2;

/// Bits of integer payload per varint byte (high bit is continuation). Protobuf base-128.
const PROTOBUF_VARINT_PAYLOAD_BITS: u32 = 7;
const PROTOBUF_VARINT_PAYLOAD_MASK: u8 = (1u8 << PROTOBUF_VARINT_PAYLOAD_BITS) - 1;
const PROTOBUF_VARINT_CONTINUATION: u8 = 1u8 << PROTOBUF_VARINT_PAYLOAD_BITS;

/// Tag key is `field_number << wire_type_bits | wire_type` (3-bit wire type).
const PROTOBUF_TAG_WIRE_TYPE_BITS: u32 = 3;
const PROTOBUF_TAG_WIRE_TYPE_MASK: u64 = (1u64 << PROTOBUF_TAG_WIRE_TYPE_BITS) - 1;

/// Protobuf field numbers from LIP‑155 LEZ integration (`rfc-index/docs/ift-ts/raw/payment-streams.md`).
mod field_numbers {
    pub const VAULT_PROOF_VAULT_ID: u32 = 1;
    pub const VAULT_PROOF_PROVIDER_ID: u32 = 2;
    pub const VAULT_PROOF_OWNER_PUBLIC_KEY: u32 = 3;
    pub const VAULT_PROOF_OWNER_SIGNATURE: u32 = 4;

    pub const STREAM_PARAMS_SERVICE_ID: u32 = 1;
    pub const STREAM_PARAMS_RATE: u32 = 2;
    pub const STREAM_PARAMS_ALLOCATION: u32 = 3;
    pub const STREAM_PARAMS_DEADLINE: u32 = 4;

    pub const STREAM_PROOF_STREAM_ID: u32 = 1;
    pub const STREAM_PROOF_SIGNATURE: u32 = 2;

    pub const STREAM_PROPOSAL_VAULT: u32 = 1;
    pub const STREAM_PROPOSAL_PARAMS: u32 = 2;
    pub const STREAM_PROPOSAL_SESSION_PUBLIC_KEY: u32 = 3;

    pub const ELIGIBILITY_STREAM_PROPOSAL: u32 = 2;
    pub const ELIGIBILITY_STREAM_PROOF: u32 = 3;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultProofWire {
    pub vault_id: u64,
    pub provider_id: [u8; 32],
    pub owner_public_key: [u8; 32],
    pub owner_signature: [u8; 64],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamProofWire {
    pub stream_id: u64,
    pub signature: [u8; 64],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamProposalWire {
    pub vault: VaultProofWire,
    pub params: StreamParams,
    pub session_public_key: [u8; 32],
}

fn read_byte(cursor: &mut usize, bytes: &[u8]) -> Result<u8, WireError> {
    let idx = *cursor;
    let next = idx.checked_add(1).ok_or(WireError::InvalidWireFrame)?;
    if next > bytes.len() {
        return Err(WireError::UnexpectedEof);
    }
    let out = bytes[idx];
    *cursor = next;
    Ok(out)
}

fn read_exact<'a>(cursor: &mut usize, bytes: &'a [u8], len: usize) -> Result<&'a [u8], WireError> {
    let start = *cursor;
    let end = start.checked_add(len).ok_or(WireError::InvalidWireFrame)?;
    if end > bytes.len() {
        return Err(WireError::UnexpectedEof);
    }
    *cursor = end;
    Ok(&bytes[start..end])
}

fn read_varint(cursor: &mut usize, bytes: &[u8]) -> Result<u64, WireError> {
    let last_u64_bit: u32 = u64::BITS - 1;
    let mut shift = 0_u32;
    let mut out = 0_u64;
    loop {
        let byte = read_byte(cursor, bytes)?;
        let value = u64::from(byte & PROTOBUF_VARINT_PAYLOAD_MASK);
        if shift >= last_u64_bit && value > 1 {
            return Err(WireError::InvalidWireFrame);
        }
        out |= value << shift;
        if byte & PROTOBUF_VARINT_CONTINUATION == 0 {
            return Ok(out);
        }
        shift += PROTOBUF_VARINT_PAYLOAD_BITS;
        if shift > last_u64_bit {
            return Err(WireError::InvalidWireFrame);
        }
    }
}

fn read_tag(cursor: &mut usize, bytes: &[u8]) -> Result<Option<(u32, u32)>, WireError> {
    if *cursor >= bytes.len() {
        return Ok(None);
    }
    let key = read_varint(cursor, bytes)?;
    let field = (key >> PROTOBUF_TAG_WIRE_TYPE_BITS) as u32;
    if field == 0 {
        return Err(WireError::InvalidWireFrame);
    }
    let wire = (key & PROTOBUF_TAG_WIRE_TYPE_MASK) as u32;
    Ok(Some((field, wire)))
}

fn skip_field(cursor: &mut usize, bytes: &[u8], wire: u32) -> Result<(), WireError> {
    match wire {
        WIRE_VARINT => {
            let _ = read_varint(cursor, bytes)?;
            Ok(())
        }
        WIRE_LEN_DELIM => {
            let len_u64 = read_varint(cursor, bytes)?;
            let len = usize::try_from(len_u64).map_err(|_| WireError::InvalidWireFrame)?;
            let _ = read_exact(cursor, bytes, len)?;
            Ok(())
        }
        _ => Err(WireError::InvalidWireFrame),
    }
}

fn read_len_delim<'a>(cursor: &mut usize, bytes: &'a [u8]) -> Result<&'a [u8], WireError> {
    let len_u64 = read_varint(cursor, bytes)?;
    let len = usize::try_from(len_u64).map_err(|_| WireError::InvalidWireFrame)?;
    read_exact(cursor, bytes, len)
}

fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    let terminal_max = u64::from(PROTOBUF_VARINT_CONTINUATION) - 1;
    loop {
        if value <= terminal_max {
            out.push(value as u8);
            break;
        }
        out.push(
            ((value & u64::from(PROTOBUF_VARINT_PAYLOAD_MASK))
                | u64::from(PROTOBUF_VARINT_CONTINUATION)) as u8,
        );
        value >>= PROTOBUF_VARINT_PAYLOAD_BITS;
    }
}

fn write_tag(out: &mut Vec<u8>, field: u32, wire: u32) {
    write_varint(
        out,
        u64::from((field << PROTOBUF_TAG_WIRE_TYPE_BITS) | wire),
    );
}

fn write_len_delim_bytes(out: &mut Vec<u8>, field: u32, payload: &[u8]) {
    write_tag(out, field, WIRE_LEN_DELIM);
    write_varint(out, payload.len() as u64);
    out.extend_from_slice(payload);
}

fn write_varint_field(out: &mut Vec<u8>, field: u32, value: u64) {
    write_tag(out, field, WIRE_VARINT);
    write_varint(out, value);
}

fn verify_fixed_bytes(label: &[u8], expected: usize) -> Result<(), WireError> {
    if label.len() != expected {
        return Err(WireError::InvalidWireFrame);
    }
    Ok(())
}

fn vault_id_from_bytes(label: &[u8]) -> Result<u64, WireError> {
    verify_fixed_bytes(label, LEZ_VAULT_ID_WIRE_LEN)?;
    Ok(u64::from_le_bytes(
        label.try_into().map_err(|_| WireError::InvalidWireFrame)?,
    ))
}

pub fn parse_stream_params(bytes: &[u8]) -> Result<StreamParams, WireError> {
    let mut service_id: Option<Vec<u8>> = None;
    let mut rate: Option<TokensPerSecond> = None;
    let mut allocation: Option<u64> = None;
    let mut deadline: Option<Timestamp> = None;

    let mut cursor = 0_usize;
    while let Some((field, wire)) = read_tag(&mut cursor, bytes)? {
        match (field, wire) {
            (field_numbers::STREAM_PARAMS_SERVICE_ID, WIRE_LEN_DELIM) => {
                let raw = read_len_delim(&mut cursor, bytes)?;
                if raw.len() > MAX_SERVICE_ID_LEN {
                    return Err(WireError::ServiceIdTooLong);
                }
                service_id = Some(raw.to_vec());
            }
            (field_numbers::STREAM_PARAMS_RATE, WIRE_VARINT) => {
                rate = Some(read_varint(&mut cursor, bytes)?)
            }
            (field_numbers::STREAM_PARAMS_ALLOCATION, WIRE_VARINT) => {
                allocation = Some(read_varint(&mut cursor, bytes)?)
            }
            (field_numbers::STREAM_PARAMS_DEADLINE, WIRE_VARINT) => {
                deadline = Some(read_varint(&mut cursor, bytes)?)
            }
            (_, _) => skip_field(&mut cursor, bytes, wire)?,
        }
    }

    if cursor != bytes.len() {
        return Err(WireError::InvalidWireFrame);
    }

    let service_id = service_id.ok_or(WireError::InvalidWireFrame)?;
    let rate = rate.ok_or(WireError::InvalidWireFrame)?;
    let allocation = allocation.ok_or(WireError::InvalidWireFrame)?;
    let deadline = deadline.ok_or(WireError::InvalidWireFrame)?;

    Ok(StreamParams::new(
        rate,
        Balance::from(allocation),
        deadline,
        service_id,
    ))
}

pub fn serialize_stream_params(params: &StreamParams) -> Result<Vec<u8>, WireError> {
    let mut out = Vec::new();
    write_len_delim_bytes(
        &mut out,
        field_numbers::STREAM_PARAMS_SERVICE_ID,
        &params.service_id,
    );
    write_varint_field(&mut out, field_numbers::STREAM_PARAMS_RATE, params.rate);
    let allocation_u64 =
        u64::try_from(params.allocation).map_err(|_| WireError::AllocationExceedsProtobufUint64)?;
    write_varint_field(
        &mut out,
        field_numbers::STREAM_PARAMS_ALLOCATION,
        allocation_u64,
    );
    write_varint_field(
        &mut out,
        field_numbers::STREAM_PARAMS_DEADLINE,
        params.create_stream_deadline,
    );
    Ok(out)
}

pub fn parse_vault_proof(bytes: &[u8]) -> Result<VaultProofWire, WireError> {
    let mut vault_label: Option<&[u8]> = None;
    let mut provider: Option<&[u8]> = None;
    let mut owner_pk: Option<&[u8]> = None;
    let mut owner_sig: Option<&[u8]> = None;

    let mut cursor = 0_usize;
    while let Some((field, wire)) = read_tag(&mut cursor, bytes)? {
        match (field, wire) {
            (field_numbers::VAULT_PROOF_VAULT_ID, WIRE_LEN_DELIM) => {
                vault_label = Some(read_len_delim(&mut cursor, bytes)?)
            }
            (field_numbers::VAULT_PROOF_PROVIDER_ID, WIRE_LEN_DELIM) => {
                provider = Some(read_len_delim(&mut cursor, bytes)?)
            }
            (field_numbers::VAULT_PROOF_OWNER_PUBLIC_KEY, WIRE_LEN_DELIM) => {
                owner_pk = Some(read_len_delim(&mut cursor, bytes)?)
            }
            (field_numbers::VAULT_PROOF_OWNER_SIGNATURE, WIRE_LEN_DELIM) => {
                owner_sig = Some(read_len_delim(&mut cursor, bytes)?)
            }
            (_, _) => skip_field(&mut cursor, bytes, wire)?,
        }
    }

    if cursor != bytes.len() {
        return Err(WireError::InvalidWireFrame);
    }

    let vault_label = vault_label.ok_or(WireError::InvalidWireFrame)?;
    let provider_slice = provider.ok_or(WireError::InvalidWireFrame)?;
    let owner_pk_slice = owner_pk.ok_or(WireError::InvalidWireFrame)?;
    let owner_sig_slice = owner_sig.ok_or(WireError::InvalidWireFrame)?;

    verify_fixed_bytes(provider_slice, LEZ_ACCOUNT_RAW_LEN)?;
    verify_fixed_bytes(owner_pk_slice, LEZ_ACCOUNT_RAW_LEN)?;
    verify_fixed_bytes(owner_sig_slice, LEZ_SCHNORR_SIGNATURE_LEN)?;

    Ok(VaultProofWire {
        vault_id: vault_id_from_bytes(vault_label)?,
        provider_id: provider_slice
            .try_into()
            .map_err(|_| WireError::InvalidWireFrame)?,
        owner_public_key: owner_pk_slice
            .try_into()
            .map_err(|_| WireError::InvalidWireFrame)?,
        owner_signature: owner_sig_slice
            .try_into()
            .map_err(|_| WireError::InvalidWireFrame)?,
    })
}

pub fn serialize_vault_proof(vault: &VaultProofWire) -> Vec<u8> {
    let mut out = Vec::new();
    write_len_delim_bytes(
        &mut out,
        field_numbers::VAULT_PROOF_VAULT_ID,
        &vault.vault_id.to_le_bytes(),
    );
    write_len_delim_bytes(
        &mut out,
        field_numbers::VAULT_PROOF_PROVIDER_ID,
        &vault.provider_id,
    );
    write_len_delim_bytes(
        &mut out,
        field_numbers::VAULT_PROOF_OWNER_PUBLIC_KEY,
        &vault.owner_public_key,
    );
    write_len_delim_bytes(
        &mut out,
        field_numbers::VAULT_PROOF_OWNER_SIGNATURE,
        &vault.owner_signature,
    );
    out
}

pub fn parse_stream_proof(bytes: &[u8]) -> Result<StreamProofWire, WireError> {
    let mut stream_label: Option<&[u8]> = None;
    let mut signature: Option<&[u8]> = None;

    let mut cursor = 0_usize;
    while let Some((field, wire)) = read_tag(&mut cursor, bytes)? {
        match (field, wire) {
            (field_numbers::STREAM_PROOF_STREAM_ID, WIRE_LEN_DELIM) => {
                stream_label = Some(read_len_delim(&mut cursor, bytes)?)
            }
            (field_numbers::STREAM_PROOF_SIGNATURE, WIRE_LEN_DELIM) => {
                signature = Some(read_len_delim(&mut cursor, bytes)?)
            }
            (_, _) => skip_field(&mut cursor, bytes, wire)?,
        }
    }

    if cursor != bytes.len() {
        return Err(WireError::InvalidWireFrame);
    }

    let stream_label = stream_label.ok_or(WireError::InvalidWireFrame)?;
    let signature = signature.ok_or(WireError::InvalidWireFrame)?;

    verify_fixed_bytes(stream_label, LEZ_STREAM_ID_WIRE_LEN)?;
    verify_fixed_bytes(signature, LEZ_SCHNORR_SIGNATURE_LEN)?;

    Ok(StreamProofWire {
        stream_id: u64::from_le_bytes(
            stream_label
                .try_into()
                .map_err(|_| WireError::InvalidWireFrame)?,
        ),
        signature: signature
            .try_into()
            .map_err(|_| WireError::InvalidWireFrame)?,
    })
}

pub fn serialize_stream_proof(proof: &StreamProofWire) -> Vec<u8> {
    let mut out = Vec::new();
    write_len_delim_bytes(
        &mut out,
        field_numbers::STREAM_PROOF_STREAM_ID,
        &proof.stream_id.to_le_bytes(),
    );
    write_len_delim_bytes(
        &mut out,
        field_numbers::STREAM_PROOF_SIGNATURE,
        &proof.signature,
    );
    out
}

pub fn parse_stream_proposal(bytes: &[u8]) -> Result<StreamProposalWire, WireError> {
    let mut vault_blob: Option<&[u8]> = None;
    let mut params_blob: Option<&[u8]> = None;
    let mut session_pk: Option<&[u8]> = None;

    let mut cursor = 0_usize;
    while let Some((field, wire)) = read_tag(&mut cursor, bytes)? {
        match (field, wire) {
            (field_numbers::STREAM_PROPOSAL_VAULT, WIRE_LEN_DELIM) => {
                vault_blob = Some(read_len_delim(&mut cursor, bytes)?)
            }
            (field_numbers::STREAM_PROPOSAL_PARAMS, WIRE_LEN_DELIM) => {
                params_blob = Some(read_len_delim(&mut cursor, bytes)?)
            }
            (field_numbers::STREAM_PROPOSAL_SESSION_PUBLIC_KEY, WIRE_LEN_DELIM) => {
                session_pk = Some(read_len_delim(&mut cursor, bytes)?)
            }
            (_, _) => skip_field(&mut cursor, bytes, wire)?,
        }
    }

    if cursor != bytes.len() {
        return Err(WireError::InvalidWireFrame);
    }

    let vault_blob = vault_blob.ok_or(WireError::InvalidWireFrame)?;
    let params_blob = params_blob.ok_or(WireError::InvalidWireFrame)?;
    let session_pk = session_pk.ok_or(WireError::InvalidWireFrame)?;

    verify_fixed_bytes(session_pk, LEZ_ACCOUNT_RAW_LEN)?;

    Ok(StreamProposalWire {
        vault: parse_vault_proof(vault_blob)?,
        params: parse_stream_params(params_blob)?,
        session_public_key: session_pk
            .try_into()
            .map_err(|_| WireError::InvalidWireFrame)?,
    })
}

pub fn serialize_stream_proposal(proposal: &StreamProposalWire) -> Result<Vec<u8>, WireError> {
    let mut out = Vec::new();
    let vault = serialize_vault_proof(&proposal.vault);
    let params = serialize_stream_params(&proposal.params)?;
    write_len_delim_bytes(&mut out, field_numbers::STREAM_PROPOSAL_VAULT, &vault);
    write_len_delim_bytes(&mut out, field_numbers::STREAM_PROPOSAL_PARAMS, &params);
    write_len_delim_bytes(
        &mut out,
        field_numbers::STREAM_PROPOSAL_SESSION_PUBLIC_KEY,
        &proposal.session_public_key,
    );
    Ok(out)
}

/// Incentivization `EligibilityProof` (LIP-155): exactly one of `stream_proposal` or `stream_proof`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EligibilityProofWire {
    StreamProposal(Vec<u8>),
    StreamProof(Vec<u8>),
}

pub fn parse_eligibility_proof(bytes: &[u8]) -> Result<EligibilityProofWire, WireError> {
    let mut proposal: Option<Vec<u8>> = None;
    let mut proof: Option<Vec<u8>> = None;
    let mut cursor = 0_usize;
    while let Some((field, wire)) = read_tag(&mut cursor, bytes)? {
        match (field, wire) {
            (field_numbers::ELIGIBILITY_STREAM_PROPOSAL, WIRE_LEN_DELIM) => {
                proposal = Some(read_len_delim(&mut cursor, bytes)?.to_vec());
            }
            (field_numbers::ELIGIBILITY_STREAM_PROOF, WIRE_LEN_DELIM) => {
                proof = Some(read_len_delim(&mut cursor, bytes)?.to_vec());
            }
            (_, _) => skip_field(&mut cursor, bytes, wire)?,
        }
    }
    if cursor != bytes.len() {
        return Err(WireError::InvalidWireFrame);
    }
    match (proposal, proof) {
        (Some(inner), None) => Ok(EligibilityProofWire::StreamProposal(inner)),
        (None, Some(inner)) => Ok(EligibilityProofWire::StreamProof(inner)),
        _ => Err(WireError::InvalidWireFrame),
    }
}

pub fn serialize_eligibility_proof(arm: &EligibilityProofWire) -> Vec<u8> {
    let mut out = Vec::new();
    match arm {
        EligibilityProofWire::StreamProposal(inner) => {
            write_len_delim_bytes(&mut out, field_numbers::ELIGIBILITY_STREAM_PROPOSAL, inner);
        }
        EligibilityProofWire::StreamProof(inner) => {
            write_len_delim_bytes(&mut out, field_numbers::ELIGIBILITY_STREAM_PROOF, inner);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream_provider_policy::StreamParams;
    use crate::Balance;

    #[test]
    fn stream_params_allocation_above_uint64_fails_serialize() {
        let params = StreamParams::new(1, Balance::from(u128::from(u64::MAX) + 1), 1, vec![]);
        let err = serialize_stream_params(&params).expect_err("allocation must not fit uint64");
        assert_eq!(err, WireError::AllocationExceedsProtobufUint64);
    }

    #[test]
    fn stream_round_trips_through_minimal_protobuf_stack() {
        let original = StreamProposalWire {
            vault: VaultProofWire {
                vault_id: 42,
                provider_id: [9_u8; 32],
                owner_public_key: [8_u8; 32],
                owner_signature: [7_u8; 64],
            },
            params: StreamParams::new(15, 200, 999, b"/demo/service".to_vec()),
            session_public_key: [3_u8; 32],
        };

        let bytes = serialize_stream_proposal(&original).expect("serializes");
        let decoded = parse_stream_proposal(&bytes).expect("round trip parses");
        assert_eq!(decoded, original);
    }
}
