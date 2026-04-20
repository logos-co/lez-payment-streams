//! Program instructions on the wire (NSSA public transaction payloads).

use serde::{Deserialize, Serialize};

use nssa_core::account::{AccountId, Balance};
use nssa_core::program::ProgramId;

use crate::{StreamId, TokensPerSecond, VaultId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    InitializeVault {
        vault_id: VaultId,
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
    SyncStream {
        vault_id: VaultId,
        stream_id: StreamId,
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
