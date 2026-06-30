//! NSSA / LEZ public-transaction instruction words for [`crate::Instruction`].
//!
//! NSSA stores guest instruction payloads as [`lee_core::program::InstructionData`] (`Vec<u32>`)
//! using Risc0’s serde codec (same as [`lee::program::Program::serialize_instruction`] and the
//! guest’s [`lee_core::program::read_lee_inputs`] path). Wallet `send_public_transaction` JSON
//! typically hex-encodes the little-endian byte expansion of those `u32` words (see
//! [`instruction_bytes_le_from_words`]).

use lee::error::LeeError;
use lee::program::Program;
use lee_core::program::InstructionData;
use risc0_zkvm::serde::{Deserializer, Error as Risc0SerdeError};
use serde::Deserialize;

use crate::Instruction;

/// Serialize an [`Instruction`] the same way NSSA builds [`PublicTransaction`](lee::public_transaction::PublicTransaction) payloads.
#[must_use]
pub fn instruction_words_for_public_transaction(
    instruction: &Instruction,
) -> Result<InstructionData, LeeError> {
    Program::serialize_instruction(instruction)
}

/// Deserialize instruction words produced by [`instruction_words_for_public_transaction`] (or the host `Program::serialize_instruction` path).
#[must_use]
pub fn instruction_try_from_instruction_words(
    words: &[u32],
) -> Result<Instruction, Risc0SerdeError> {
    Instruction::deserialize(&mut Deserializer::new(words))
}

/// Flatten NSSA [`InstructionData`] to raw bytes (`u32::to_le_bytes` per word).
#[must_use]
pub fn instruction_bytes_le_from_words(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

/// Convenience: instruction words then LE byte expansion (for `send_public_transaction` `instruction` hex).
#[must_use]
pub fn instruction_bytes_for_public_transaction(
    instruction: &Instruction,
) -> Result<Vec<u8>, LeeError> {
    let words = instruction_words_for_public_transaction(instruction)?;
    Ok(instruction_bytes_le_from_words(&words))
}

/// Parse a LE `u32` word slice from a byte buffer (must be a multiple of four).
#[must_use]
pub fn instruction_words_from_bytes_le(bytes: &[u8]) -> Option<Vec<u32>> {
    if bytes.len() % 4 != 0 {
        return None;
    }
    let mut words = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        words.push(u32::from_le_bytes(chunk.try_into().ok()?));
    }
    Some(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VaultPrivacyTier;
    use lee_core::account::AccountId;
    use programs::authenticated_transfer;

    #[test]
    fn all_variants_round_trip_via_instruction_words() {
        let transfer_pid = authenticated_transfer().id();
        let provider = AccountId::new([11_u8; 32]);

        let samples = vec![
            Instruction::initialize_vault(7, VaultPrivacyTier::Public),
            Instruction::initialize_vault(8, VaultPrivacyTier::PseudonymousFunder),
            Instruction::Deposit {
                vault_id: 1,
                amount: 42,
                authenticated_transfer_program_id: transfer_pid,
            },
            Instruction::Withdraw {
                vault_id: 2,
                amount: 99,
            },
            Instruction::CreateStream {
                vault_id: 3,
                stream_id: 4,
                provider,
                rate: 10,
                allocation: 200,
            },
            Instruction::PauseStream {
                vault_id: 5,
                stream_id: 6,
            },
            Instruction::ResumeStream {
                vault_id: 7,
                stream_id: 8,
            },
            Instruction::TopUpStream {
                vault_id: 9,
                stream_id: 10,
                vault_total_allocated_increase: 123,
            },
            Instruction::CloseStream {
                vault_id: 11,
                stream_id: 12,
            },
            Instruction::Claim {
                vault_id: 13,
                stream_id: 14,
            },
        ];

        for instruction in samples {
            let words =
                instruction_words_for_public_transaction(&instruction).unwrap_or_else(|err| {
                    panic!("serialize failed: {err:?} instruction={instruction:?}")
                });
            let decoded = instruction_try_from_instruction_words(&words).unwrap_or_else(|err| {
                panic!("deserialize failed: {err:?} instruction={instruction:?}")
            });
            assert_eq!(instruction, decoded);

            let bytes = instruction_bytes_le_from_words(&words);
            let reparsed = instruction_words_from_bytes_le(&bytes).expect("bytes should parse");
            assert_eq!(reparsed, words);
        }
    }
}
