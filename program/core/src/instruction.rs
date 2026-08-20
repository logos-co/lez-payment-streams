//! Program instructions on the wire (NSSA public transaction payloads).

use serde::{Deserialize, Serialize};

use lee_core::account::{AccountId, Balance};
use lee_core::program::ProgramId;

use crate::{StreamId, TokensPerSecond, VaultId, VaultPrivacyTier};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Instruction {
    InitializeVault {
        vault_id: VaultId,
        /// Serialized as a single wire byte; see [`crate::VaultPrivacyTier`].
        privacy_tier: VaultPrivacyTier,
        /// LIP-155 vault token identity. All-zeroes is native.
        token_id: [u8; 32],
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
    CloseStreamByOwner {
        vault_id: VaultId,
        stream_id: StreamId,
    },
    Claim {
        vault_id: VaultId,
        stream_id: StreamId,
    },
    CloseStreamByProvider {
        vault_id: VaultId,
        stream_id: StreamId,
    },
    WithdrawToOwner {
        vault_id: VaultId,
        amount: Balance,
    },
}

impl Instruction {
    pub fn initialize_vault(vault_id: VaultId, privacy_tier: VaultPrivacyTier) -> Self {
        Self::initialize_vault_with_token(vault_id, privacy_tier, crate::NATIVE_TOKEN_ID)
    }

    pub fn initialize_vault_with_token(
        vault_id: VaultId,
        privacy_tier: VaultPrivacyTier,
        token_id: [u8; 32],
    ) -> Self {
        Self::InitializeVault {
            vault_id,
            privacy_tier,
            token_id,
        }
    }

    pub fn initialize_vault_public(vault_id: VaultId) -> Self {
        Self::initialize_vault(vault_id, VaultPrivacyTier::Public)
    }
}
