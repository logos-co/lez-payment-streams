//! Ordered account id lists for each public [`crate::Instruction`] variant.
//!
//! Ordering matches the SPEL guest `#[instruction]` account parameter lists
//! (`methods/guest/src/bin/lez_payment_streams.rs`) and the harness helpers in
//! [`crate::test_helpers`] / [`crate::program_tests::common`].

use nssa_core::account::AccountId;
use nssa_core::program::ProgramId;

use crate::pda::{derive_stream_config_account_id, derive_vault_account_ids};
use crate::{StreamId, VaultId};

/// `initialize_vault`, `deposit`: vault config PDA, vault holding PDA, owner (signer).
pub type InitializeVaultInstructionAccounts = [AccountId; 3];
/// Same slot pattern as [`InitializeVaultInstructionAccounts`] (PDAs are identical for a vault).
pub type DepositInstructionAccounts = [AccountId; 3];
/// `withdraw`: vault config, vault holding, owner (signer), withdraw recipient.
pub type WithdrawInstructionAccounts = [AccountId; 4];
/// Owner-authorized stream instructions share this five-account tail:
/// vault config, vault holding, stream config PDA, owner (signer), system clock account.
pub type StreamOwnerInstructionAccounts = [AccountId; 5];
/// `close_stream` / `claim`: adds an explicit signer after the bound owner slot.
pub type StreamAuthorityInstructionAccounts = [AccountId; 6];
/// `claim` reuses the [`StreamAuthorityInstructionAccounts`] slot pattern (`close_stream` layout).
pub type ClaimStreamInstructionAccounts = StreamAuthorityInstructionAccounts;

#[must_use]
pub fn initialize_vault_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
) -> InitializeVaultInstructionAccounts {
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_account_ids(program_id, owner_account_id, vault_id);
    [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ]
}

#[must_use]
pub fn deposit_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
) -> DepositInstructionAccounts {
    initialize_vault_instruction_accounts(program_id, owner_account_id, vault_id)
}

#[must_use]
pub fn withdraw_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
    withdraw_to_account_id: AccountId,
) -> WithdrawInstructionAccounts {
    let [vault_config_account_id, vault_holding_account_id, owner_id] =
        initialize_vault_instruction_accounts(program_id, owner_account_id, vault_id);
    [
        vault_config_account_id,
        vault_holding_account_id,
        owner_id,
        withdraw_to_account_id,
    ]
}

#[must_use]
pub fn create_stream_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
    stream_id: StreamId,
    clock_account_id: AccountId,
) -> StreamOwnerInstructionAccounts {
    stream_owner_instruction_accounts(
        program_id,
        owner_account_id,
        vault_id,
        stream_id,
        clock_account_id,
    )
}

#[must_use]
pub fn pause_stream_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
    stream_id: StreamId,
    clock_account_id: AccountId,
) -> StreamOwnerInstructionAccounts {
    stream_owner_instruction_accounts(
        program_id,
        owner_account_id,
        vault_id,
        stream_id,
        clock_account_id,
    )
}

#[must_use]
pub fn resume_stream_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
    stream_id: StreamId,
    clock_account_id: AccountId,
) -> StreamOwnerInstructionAccounts {
    stream_owner_instruction_accounts(
        program_id,
        owner_account_id,
        vault_id,
        stream_id,
        clock_account_id,
    )
}

#[must_use]
pub fn top_up_stream_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
    stream_id: StreamId,
    clock_account_id: AccountId,
) -> StreamOwnerInstructionAccounts {
    stream_owner_instruction_accounts(
        program_id,
        owner_account_id,
        vault_id,
        stream_id,
        clock_account_id,
    )
}

#[must_use]
fn stream_owner_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
    stream_id: StreamId,
    clock_account_id: AccountId,
) -> StreamOwnerInstructionAccounts {
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_account_ids(program_id, owner_account_id, vault_id);
    let stream_config_account_id =
        derive_stream_config_account_id(program_id, vault_config_account_id, stream_id);
    [
        vault_config_account_id,
        vault_holding_account_id,
        stream_config_account_id,
        owner_account_id,
        clock_account_id,
    ]
}

#[must_use]
pub fn close_stream_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
    stream_id: StreamId,
    authority_account_id: AccountId,
    clock_account_id: AccountId,
) -> StreamAuthorityInstructionAccounts {
    stream_authority_instruction_accounts(
        program_id,
        owner_account_id,
        vault_id,
        stream_id,
        authority_account_id,
        clock_account_id,
    )
}

#[must_use]
pub fn claim_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
    stream_id: StreamId,
    provider_account_id: AccountId,
    clock_account_id: AccountId,
) -> StreamAuthorityInstructionAccounts {
    stream_authority_instruction_accounts(
        program_id,
        owner_account_id,
        vault_id,
        stream_id,
        provider_account_id,
        clock_account_id,
    )
}

#[must_use]
fn stream_authority_instruction_accounts(
    program_id: &ProgramId,
    owner_account_id: AccountId,
    vault_id: VaultId,
    stream_id: StreamId,
    authority_account_id: AccountId,
    clock_account_id: AccountId,
) -> StreamAuthorityInstructionAccounts {
    let [a, b, c, owner_slot, clock] = stream_owner_instruction_accounts(
        program_id,
        owner_account_id,
        vault_id,
        stream_id,
        clock_account_id,
    );
    [a, b, c, owner_slot, authority_account_id, clock]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness_seeds::SEED_PROVIDER;
    use crate::program_tests::common::{
        first_stream_ix_accounts, state_deposited_with_clock, CloseStreamIxAccounts,
        DEFAULT_CLOCK_INITIAL_TS, DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
    };
    use crate::test_helpers::{
        create_keypair, derive_stream_pda, state_with_initialized_vault,
        state_with_initialized_vault_with_recipient,
    };
    use crate::CLOCK_01_PROGRAM_ACCOUNT_ID;

    #[test]
    #[ignore = "guest targets LEZ 491 (LEE PDAs and authenticated_transfer enum); NSSA in-process harness expects NSSA v0.1.2 encoding"]
    fn initialize_and_deposit_match_harness_fixture() {
        let fixture = state_with_initialized_vault(DEFAULT_OWNER_GENESIS_BALANCE);
        let expected = [
            fixture.vault_config_account_id,
            fixture.vault_holding_account_id,
            fixture.owner_account_id,
        ];
        assert_eq!(
            initialize_vault_instruction_accounts(
                &fixture.program_id,
                fixture.owner_account_id,
                fixture.vault_id,
            ),
            expected
        );
        assert_eq!(
            deposit_instruction_accounts(
                &fixture.program_id,
                fixture.owner_account_id,
                fixture.vault_id,
            ),
            expected
        );
    }

    #[test]
    #[ignore = "guest targets LEZ 491 (LEE PDAs and authenticated_transfer enum); NSSA in-process harness expects NSSA v0.1.2 encoding"]
    fn withdraw_matches_harness_four_account_flow() {
        let fixture = state_with_initialized_vault_with_recipient(DEFAULT_OWNER_GENESIS_BALANCE);
        let expected = [
            fixture.vault.vault_config_account_id,
            fixture.vault.vault_holding_account_id,
            fixture.vault.owner_account_id,
            fixture.recipient_account_id,
        ];
        assert_eq!(
            withdraw_instruction_accounts(
                &fixture.vault.program_id,
                fixture.vault.owner_account_id,
                fixture.vault.vault_id,
                fixture.recipient_account_id,
            ),
            expected
        );
    }

    #[test]
    #[ignore = "guest targets LEZ 491 (LEE PDAs and authenticated_transfer enum); NSSA in-process harness expects NSSA v0.1.2 encoding"]
    fn stream_owner_instructions_match_first_stream_ix_accounts() {
        let dep = state_deposited_with_clock(
            DEFAULT_OWNER_GENESIS_BALANCE,
            DEFAULT_STREAM_TEST_DEPOSIT,
            CLOCK_01_PROGRAM_ACCOUNT_ID,
            DEFAULT_CLOCK_INITIAL_TS,
        );
        let (stream_id, _stream_pda, expected) = first_stream_ix_accounts(&dep);
        let planner = create_stream_instruction_accounts(
            &dep.vault.program_id,
            dep.vault.owner_account_id,
            dep.vault.vault_id,
            stream_id,
            dep.clock_id,
        );
        assert_eq!(planner, expected);
        assert_eq!(
            pause_stream_instruction_accounts(
                &dep.vault.program_id,
                dep.vault.owner_account_id,
                dep.vault.vault_id,
                stream_id,
                dep.clock_id,
            ),
            expected
        );
    }

    #[test]
    #[ignore = "guest targets LEZ 491 (LEE PDAs and authenticated_transfer enum); NSSA in-process harness expects NSSA v0.1.2 encoding"]
    fn close_and_claim_match_six_account_layout() {
        let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
        let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
        let t0 = DEFAULT_CLOCK_INITIAL_TS;
        let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
        let (_, provider_account_id) = create_keypair(SEED_PROVIDER);

        let dep = state_deposited_with_clock(owner_balance_start, deposit_amount, clock_id, t0);
        let stream_id = StreamId::MIN;
        let stream_pda = derive_stream_pda(
            dep.vault.program_id,
            dep.vault.vault_config_account_id,
            stream_id,
        );
        let close_accounts: CloseStreamIxAccounts = [
            dep.vault.vault_config_account_id,
            dep.vault.vault_holding_account_id,
            stream_pda,
            dep.vault.owner_account_id,
            provider_account_id,
            clock_id,
        ];

        let planner = close_stream_instruction_accounts(
            &dep.vault.program_id,
            dep.vault.owner_account_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            clock_id,
        );
        assert_eq!(planner, close_accounts);

        let claim_planner = claim_instruction_accounts(
            &dep.vault.program_id,
            dep.vault.owner_account_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            clock_id,
        );
        assert_eq!(claim_planner, close_accounts);
    }
}
