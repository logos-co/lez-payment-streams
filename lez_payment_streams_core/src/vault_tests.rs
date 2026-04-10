use nssa_core::{
    account::{AccountId, Balance, Nonce},
    program::BlockId,
};
use nssa::{program::Program};

use crate::{
    DEFAULT_VERSION, VaultConfig, VaultHolding, VaultId,
    test_helpers::{
        assert_vault_state_unchanged, assert_vault_state_unchanged_with_recipient,
        build_signed_public_tx, create_keypair, create_state_with_guest_program,
        derive_vault_pdas, state_with_initialized_vault,
        state_with_initialized_vault_with_recipient,
    },
};
use crate::Instruction;


// ---- Key derivation ---- //

#[test]
fn test_keypair_is_deterministic_for_seed() {
    let (_, first) = create_keypair(7);
    let (_, second) = create_keypair(7);
    assert_eq!(first, second);
}


// ---- Serialization ---- //

#[test]
fn test_vault_config_roundtrip_serialization() {
    let vault_config = VaultConfig::new(AccountId::new([43; 32]), 34u64);
    let serialized = vault_config.to_bytes();
    let deserialized = VaultConfig::from_bytes(&serialized);
    assert_eq!(Some(vault_config), deserialized);
}

// ---- Vault initialization ---- //

#[test]
fn test_vault_config_from_bytes_wrong_len_returns_none() {
    let vault_config = VaultConfig::new(AccountId::new([43; 32]), 34u64);
    let bytes = vault_config.to_bytes();
    let short = &bytes[..bytes.len() - 1];
    assert!(VaultConfig::from_bytes(short).is_none());
    let mut long = bytes.clone();
    long.push(0);
    assert!(VaultConfig::from_bytes(&long).is_none());
}

#[test]
fn test_vault_holding_roundtrip_serialization() {
    let vault_holding = VaultHolding::new();
    let serialized = vault_holding.to_bytes();
    let deserialized = VaultHolding::from_bytes(&serialized);
    assert_eq!(Some(vault_holding), deserialized);
}

#[test]
fn test_vault_holding_from_bytes_wrong_len_returns_none() {
    let vault_holding = VaultHolding::new();
    let bytes = vault_holding.to_bytes();
    let short = &bytes[..bytes.len() - 1];
    assert!(VaultHolding::from_bytes(short).is_none());
    let mut long = bytes.clone();
    long.push(0);
    assert!(VaultHolding::from_bytes(&long).is_none());
}


// ---- Vault functionality ---- //

#[test]
fn test_initialize_vault_then_reinitialize_fails() {
    let (owner_private_key, owner_account_id) = create_keypair(1);
    let initial_accounts_data = vec![(owner_account_id, 1_000 as Balance)];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect("guest ELF present (build methods/guest) and state genesis ok");
    let program_id = guest_program.id();

    let vault_id: VaultId = 1;
    let block_init = 1 as BlockId;
    let block_reinit = 2 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_reinit = Nonce(1);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids = [vault_config_account_id, vault_holding_account_id, owner_account_id];

    // one nonce per _signer_ account (not every account!)
    let instruction_init = Instruction::InitializeVault { vault_id };
    let tx_init = build_signed_public_tx(
        program_id,
        instruction_init,
        &account_ids,
        &[nonce_init],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_init, block_init);
    assert!(result.is_ok(), "initialize_vault tx failed: {:?}", result);
    let vault_config_account = state.get_account_by_id(vault_config_account_id);
    assert_eq!(vault_config_account.data.len(), VaultConfig::SIZE);
    let vault_config = VaultConfig::from_bytes(&vault_config_account.data).expect("valid vault config bytes");
    assert_eq!(vault_config.version, DEFAULT_VERSION);
    assert_eq!(vault_config.owner, owner_account_id);
    assert_eq!(vault_config.vault_id, vault_id);
    assert_eq!(vault_config.next_stream_id, 0);
    assert_eq!(vault_config.total_allocated, 0);
    let vault_holding_account = state.get_account_by_id(vault_holding_account_id);
    assert_eq!(vault_holding_account.data.len(), VaultHolding::SIZE);
    let vault_holding = VaultHolding::from_bytes(&vault_holding_account.data).expect("valid vault holding bytes");
    assert_eq!(vault_holding.version, DEFAULT_VERSION);

    // negative test: re-initialization must fail (SPEL reports init-on-existing during account
    // validation; the host sees an opaque transaction error, not a structured SpelError here).
    let instruction_reinit = Instruction::InitializeVault { vault_id };
    let tx_reinit = build_signed_public_tx(
        program_id,
        instruction_reinit,
        &account_ids,
        &[nonce_reinit],
        &[&owner_private_key],
    );
    let result = state.transition_from_public_transaction(&tx_reinit, block_reinit);
    assert!(result.is_err(), "repeated initialize_vault tx succeeded: {:?}", result);

}


// ---- Deposit and withdraw ---- //

#[test]
fn test_deposit() {
    let owner_balance_before = 1_000 as Balance;
    let deposit_amount = 300 as Balance;
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(owner_balance_before);
    let account_ids = [vault_config_account_id, vault_holding_account_id, owner_account_id];

    let vault_config_before = state.get_account_by_id(vault_config_account_id);
    let vault_config_state_before =
        VaultConfig::from_bytes(&vault_config_before.data).expect("valid vault config bytes");
    let instruction_deposit = Instruction::Deposit {
        vault_id,
        amount: deposit_amount,
        authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
    };
    let tx_deposit = build_signed_public_tx(
        program_id,
        instruction_deposit,
        &account_ids,
        &[nonce_deposit],
        &[&owner_private_key],
    );

    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;

    let result_deposit = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(result_deposit.is_ok(), "deposit tx failed: {:?}", result_deposit);

    let owner_balance_after = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_after = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_after = state.get_account_by_id(vault_config_account_id);
    let vault_config_state_after =
        VaultConfig::from_bytes(&vault_config_after.data).expect("valid vault config bytes");

    assert_eq!(owner_balance_after, owner_balance_before - deposit_amount);
    assert_eq!(vault_holding_balance_after, vault_holding_balance_before + deposit_amount);
    assert_eq!(
        vault_config_state_after.total_allocated,
        vault_config_state_before.total_allocated
    );
    assert_eq!(
        vault_config_state_after.next_stream_id,
        vault_config_state_before.next_stream_id
    );
    assert_eq!(vault_config_state_after.version, vault_config_state_before.version);
    assert_eq!(vault_config_state_after.owner, vault_config_state_before.owner);
    assert_eq!(vault_config_state_after.vault_id, vault_config_state_before.vault_id);
}

#[test]
fn test_deposit_zero_amount_fails() {
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let deposit_amount = 0 as Balance;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(1_000);
    let account_ids = [vault_config_account_id, vault_holding_account_id, owner_account_id];

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_data_before = state.get_account_by_id(vault_config_account_id).data.clone();

    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids,
        &[nonce_deposit],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(result.is_err(), "deposit with zero amount succeeded: {:?}", result);

    assert_vault_state_unchanged(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_deposit_wrong_vault_id_fails() {
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let wrong_vault_id: VaultId = 999;
    let deposit_amount = 100 as Balance;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        _vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(1_000);
    let account_ids = [vault_config_account_id, vault_holding_account_id, owner_account_id];

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_data_before = state.get_account_by_id(vault_config_account_id).data.clone();

    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id: wrong_vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids,
        &[nonce_deposit],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(result.is_err(), "deposit with mismatched vault_id succeeded: {:?}", result);

    assert_vault_state_unchanged(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_deposit_wrong_authenticated_transfer_program_id_fails() {
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let deposit_amount = 100 as Balance;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(1_000);
    let account_ids = [vault_config_account_id, vault_holding_account_id, owner_account_id];

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_data_before = state.get_account_by_id(vault_config_account_id).data.clone();

    // Chained transfer must target `Program::authenticated_transfer_program().id()`; using the
    // payment-streams guest `program_id` here is deliberately wrong and fails chained execution.
    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: program_id,
        },
        &account_ids,
        &[nonce_deposit],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(
        result.is_err(),
        "deposit with wrong authenticated_transfer_program_id succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_deposit_insufficient_funds_fails() {
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let genesis_owner_balance = 100 as Balance;
    let deposit_amount = 200 as Balance;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault(genesis_owner_balance);
    let account_ids = [vault_config_account_id, vault_holding_account_id, owner_account_id];

    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_data_before = state.get_account_by_id(vault_config_account_id).data.clone();

    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids,
        &[nonce_deposit],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(
        result.is_err(),
        "deposit larger than owner balance succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        genesis_owner_balance,
        vault_holding_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_deposit_owner_mismatch_fails() {
    let block_init = 1 as BlockId;
    let block_deposit = 2 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_deposit = Nonce(1);
    let deposit_amount = 100 as Balance;

    let (owner_private_key, owner_account_id) = create_keypair(1);
    let (other_private_key, other_account_id) = create_keypair(2);
    let initial_accounts_data = vec![
        (owner_account_id, 1_000 as Balance),
        (other_account_id, 1_000 as Balance),
    ];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect("guest ELF present (build methods/guest) and state genesis ok");
    let program_id = guest_program.id();

    let vault_id: VaultId = 1;
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids_init = [vault_config_account_id, vault_holding_account_id, owner_account_id];
    let tx_init = build_signed_public_tx(
        program_id,
        Instruction::InitializeVault { vault_id },
        &account_ids_init,
        &[nonce_init],
        &[&owner_private_key],
    );
    let result_init = state.transition_from_public_transaction(&tx_init, block_init);
    assert!(result_init.is_ok(), "initialize_vault tx failed: {:?}", result_init);

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let other_balance_before = state.get_account_by_id(other_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_data_before = state.get_account_by_id(vault_config_account_id).data.clone();

    let account_ids_deposit = [
        vault_config_account_id,
        vault_holding_account_id,
        other_account_id,
    ];
    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids_deposit,
        &[nonce_deposit],
        &[&other_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(
        result.is_err(),
        "deposit with non-owner funding account succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        vault_config_data_before,
    );
    assert_eq!(
        state.get_account_by_id(other_account_id).balance,
        other_balance_before
    );
}

#[test]
fn test_withdraw() {
    let owner_balance_start = 1_000 as Balance;
    let deposit_amount = 400 as Balance;
    let withdraw_amount = 100 as Balance;
    let block_deposit = 2 as BlockId;
    let block_withdraw = 3 as BlockId;
    let nonce_deposit = Nonce(1);
    let nonce_withdraw = Nonce(2);

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        recipient_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault_with_recipient(owner_balance_start);

    let account_ids_deposit =
        [vault_config_account_id, vault_holding_account_id, owner_account_id];
    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids_deposit,
        &[nonce_deposit],
        &[&owner_private_key],
    );
    let result_deposit = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(result_deposit.is_ok(), "deposit tx failed: {:?}", result_deposit);

    let vault_config_before = state.get_account_by_id(vault_config_account_id);
    let vault_config_state_before =
        VaultConfig::from_bytes(&vault_config_before.data).expect("valid vault config bytes");
    let owner_after_deposit = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_before_withdraw =
        state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_before_withdraw = state.get_account_by_id(recipient_account_id).balance;

    let account_ids_withdraw = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        recipient_account_id,
    ];
    let tx_withdraw = build_signed_public_tx(
        program_id,
        Instruction::Withdraw {
            vault_id,
            amount: withdraw_amount,
        },
        &account_ids_withdraw,
        &[nonce_withdraw],
        &[&owner_private_key],
    );
    let result_withdraw = state.transition_from_public_transaction(&tx_withdraw, block_withdraw);
    assert!(result_withdraw.is_ok(), "withdraw tx failed: {:?}", result_withdraw);

    let vault_config_after = state.get_account_by_id(vault_config_account_id);
    let vault_config_state_after =
        VaultConfig::from_bytes(&vault_config_after.data).expect("valid vault config bytes");

    assert_eq!(
        state.get_account_by_id(owner_account_id).balance,
        owner_after_deposit
    );
    assert_eq!(
        state.get_account_by_id(vault_holding_account_id).balance,
        vault_holding_before_withdraw - withdraw_amount
    );
    assert_eq!(
        state.get_account_by_id(recipient_account_id).balance,
        recipient_before_withdraw + withdraw_amount
    );
    assert_eq!(
        vault_config_state_after.total_allocated,
        vault_config_state_before.total_allocated
    );
    assert_eq!(
        vault_config_state_after.next_stream_id,
        vault_config_state_before.next_stream_id
    );
    assert_eq!(vault_config_state_after.version, vault_config_state_before.version);
    assert_eq!(vault_config_state_after.owner, vault_config_state_before.owner);
    assert_eq!(vault_config_state_after.vault_id, vault_config_state_before.vault_id);
    assert_eq!(
        owner_after_deposit,
        owner_balance_start - deposit_amount
    );
}

#[test]
fn test_withdraw_zero_amount_fails() {
    let block_withdraw = 2 as BlockId;
    let nonce_withdraw = Nonce(1);

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        recipient_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault_with_recipient(1_000);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        recipient_account_id,
    ];

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_balance_before = state.get_account_by_id(recipient_account_id).balance;
    let vault_config_data_before = state.get_account_by_id(vault_config_account_id).data.clone();

    let tx_withdraw = build_signed_public_tx(
        program_id,
        Instruction::Withdraw {
            vault_id,
            amount: 0,
        },
        &account_ids,
        &[nonce_withdraw],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_withdraw, block_withdraw);
    assert!(result.is_err(), "withdraw with zero amount succeeded: {:?}", result);

    assert_vault_state_unchanged_with_recipient(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_withdraw_wrong_vault_id_fails() {
    let block_withdraw = 2 as BlockId;
    let nonce_withdraw = Nonce(1);
    let wrong_vault_id: VaultId = 999;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        recipient_account_id,
        _vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault_with_recipient(1_000);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        recipient_account_id,
    ];

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_balance_before = state.get_account_by_id(recipient_account_id).balance;
    let vault_config_data_before = state.get_account_by_id(vault_config_account_id).data.clone();

    let tx_withdraw = build_signed_public_tx(
        program_id,
        Instruction::Withdraw {
            vault_id: wrong_vault_id,
            amount: 100,
        },
        &account_ids,
        &[nonce_withdraw],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_withdraw, block_withdraw);
    assert!(
        result.is_err(),
        "withdraw with mismatched vault_id succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged_with_recipient(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_withdraw_exceeds_available_fails() {
    let block_deposit = 2 as BlockId;
    let block_withdraw = 3 as BlockId;
    let nonce_deposit = Nonce(1);
    let nonce_withdraw = Nonce(2);
    let deposit_amount = 100 as Balance;
    let withdraw_amount = 101 as Balance;

    let (
        mut state,
        program_id,
        owner_private_key,
        owner_account_id,
        recipient_account_id,
        vault_id,
        vault_config_account_id,
        vault_holding_account_id,
    ) = state_with_initialized_vault_with_recipient(1_000);

    let account_ids_deposit =
        [vault_config_account_id, vault_holding_account_id, owner_account_id];
    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids_deposit,
        &[nonce_deposit],
        &[&owner_private_key],
    );
    assert!(
        state
            .transition_from_public_transaction(&tx_deposit, block_deposit)
            .is_ok(),
        "deposit failed"
    );

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_balance_before = state.get_account_by_id(recipient_account_id).balance;
    let vault_config_data_before = state.get_account_by_id(vault_config_account_id).data.clone();

    let account_ids_withdraw = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
        recipient_account_id,
    ];
    let tx_withdraw = build_signed_public_tx(
        program_id,
        Instruction::Withdraw {
            vault_id,
            amount: withdraw_amount,
        },
        &account_ids_withdraw,
        &[nonce_withdraw],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_withdraw, block_withdraw);
    assert!(
        result.is_err(),
        "withdraw exceeding unallocated balance succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged_with_recipient(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_withdraw_owner_mismatch_fails() {
    let block_init = 1 as BlockId;
    let block_withdraw = 2 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_withdraw = Nonce(1);

    let (owner_private_key, owner_account_id) = create_keypair(1);
    let (other_private_key, other_account_id) = create_keypair(2);
    let (_, recipient_account_id) = create_keypair(88);
    let initial_accounts_data = vec![
        (owner_account_id, 1_000 as Balance),
        (other_account_id, 1_000 as Balance),
        (recipient_account_id, 0 as Balance),
    ];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect("guest ELF present (build methods/guest) and state genesis ok");
    let program_id = guest_program.id();

    let vault_id: VaultId = 1;
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids_init = [vault_config_account_id, vault_holding_account_id, owner_account_id];
    let tx_init = build_signed_public_tx(
        program_id,
        Instruction::InitializeVault { vault_id },
        &account_ids_init,
        &[nonce_init],
        &[&owner_private_key],
    );
    assert!(
        state
            .transition_from_public_transaction(&tx_init, block_init)
            .is_ok(),
        "initialize_vault tx failed"
    );

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let other_balance_before = state.get_account_by_id(other_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_balance_before = state.get_account_by_id(recipient_account_id).balance;
    let vault_config_data_before = state.get_account_by_id(vault_config_account_id).data.clone();

    let account_ids_withdraw = [
        vault_config_account_id,
        vault_holding_account_id,
        other_account_id,
        recipient_account_id,
    ];
    let tx_withdraw = build_signed_public_tx(
        program_id,
        Instruction::Withdraw {
            vault_id,
            amount: 50,
        },
        &account_ids_withdraw,
        &[nonce_withdraw],
        &[&other_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_withdraw, block_withdraw);
    assert!(
        result.is_err(),
        "withdraw with non-owner signer slot succeeded: {:?}",
        result
    );

    assert_vault_state_unchanged_with_recipient(
        &state,
        owner_account_id,
        vault_holding_account_id,
        vault_config_account_id,
        recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
    assert_eq!(
        state.get_account_by_id(other_account_id).balance,
        other_balance_before
    );
}