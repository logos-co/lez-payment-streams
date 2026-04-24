//! Program instructions on the wire (NSSA public transaction payloads).

use serde::{Deserialize, Serialize};

use nssa_core::account::{AccountId, Balance};
use nssa_core::program::ProgramId;

use crate::{StreamId, TokensPerSecond, VaultId, VaultPrivacyTier};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    InitializeVault {
        vault_id: VaultId,
        /// Serialized as a single wire byte; see [`crate::VaultPrivacyTier`].
        privacy_tier: VaultPrivacyTier,
    },
    Deposit {
        vault_id: VaultId,
        amount: Balance,
        authenticated_transfer_program_id: ProgramId,
    },
    Withdraw {
        vault_id: VaultId,
        amount: Balance,
    },
    CreateStream {
        vault_id: VaultId,
        stream_id: StreamId,
        provider: AccountId,
        rate: TokensPerSecond,
        allocation: Balance,
    },
    PauseStream {
        vault_id: VaultId,
        stream_id: StreamId,
    },
    ResumeStream {
        vault_id: VaultId,
        stream_id: StreamId,
    },
    TopUpStream {
        vault_id: VaultId,
        stream_id: StreamId,
        vault_total_allocated_increase: Balance,
    },
    CloseStream {
        vault_id: VaultId,
        stream_id: StreamId,
    },
    Claim {
        vault_id: VaultId,
        stream_id: StreamId,
    },
}

impl Instruction {
    pub fn initialize_vault(vault_id: VaultId, privacy_tier: VaultPrivacyTier) -> Self {
        Self::InitializeVault {
            vault_id,
            privacy_tier,
        }
    }

    pub fn initialize_vault_public(vault_id: VaultId) -> Self {
        Self::initialize_vault(vault_id, VaultPrivacyTier::Public)
    }
}
