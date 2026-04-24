//! `withdraw` to third-party recipients.

use nssa::program::Program;
use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        assert_vault_state_unchanged_with_recipient, build_signed_public_tx, create_keypair,
        create_state_with_guest_program, derive_stream_pda, derive_vault_pdas,
        force_clock_account_monotonic, harness_clock_01_and_provider_account_ids,
        patch_vault_config, state_with_initialized_vault_with_recipient,
    },
    error_codes::ErrorCode, TokensPerSecond, VaultConfig, VaultId,
};

use super::common::{
    assert_execution_failed_with_code, DEFAULT_CLOCK_INITIAL_TS, DEFAULT_OWNER_GENESIS_BALANCE,
    DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_OWNER, SEED_RECIPIENT};

#[test]
fn test_withdraw_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = 400 as Balance;
    let withdraw_amount = 100 as Balance;
    let block_deposit = 2 as BlockId;
    let block_withdraw = 3 as BlockId;
    let nonce_deposit = Nonce(1);
    let nonce_withdraw = Nonce(2);

    let mut wr = state_with_initialized_vault_with_recipient(owner_balance_start);

    let account_ids_deposit = [
        wr.vault.vault_config_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.owner_account_id,
    ];
    let tx_deposit = build_signed_public_tx(
        wr.vault.program_id,
        Instruction::Deposit {
            vault_id: wr.vault.vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids_deposit,
        &[nonce_deposit],
        &[&wr.vault.owner_private_key],
    );
    let result_deposit = wr.vault.state.transition_from_public_transaction(
        &tx_deposit,
        block_deposit,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result_deposit.is_ok(),
        "deposit tx failed: {:?}",
        result_deposit
    );

    let vault_config_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_config_account_id);
    let vault_config_state_before =
        VaultConfig::from_bytes(&vault_config_before.data).expect("valid vault config bytes");
    let owner_after_deposit = wr
        .vault
        .state
        .get_account_by_id(wr.vault.owner_account_id)
        .balance;
    let vault_holding_before_withdraw = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_holding_account_id)
        .balance;
    let recipient_before_withdraw = wr
        .vault
        .state
        .get_account_by_id(wr.recipient_account_id)
        .balance;

    let account_ids_withdraw = [
        wr.vault.vault_config_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.owner_account_id,
        wr.recipient_account_id,
    ];
    let tx_withdraw = build_signed_public_tx(
        wr.vault.program_id,
        Instruction::Withdraw {
            vault_id: wr.vault.vault_id,
            amount: withdraw_amount,
        },
        &account_ids_withdraw,
        &[nonce_withdraw],
        &[&wr.vault.owner_private_key],
    );
    let result_withdraw = wr.vault.state.transition_from_public_transaction(
        &tx_withdraw,
        block_withdraw,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result_withdraw.is_ok(),
        "withdraw tx failed: {:?}",
        result_withdraw
    );

    let vault_config_after = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_config_account_id);
    let vault_config_state_after =
        VaultConfig::from_bytes(&vault_config_after.data).expect("valid vault config bytes");

    assert_eq!(
        wr.vault
            .state
            .get_account_by_id(wr.vault.owner_account_id)
            .balance,
        owner_after_deposit
    );
    assert_eq!(
        wr.vault
            .state
            .get_account_by_id(wr.vault.vault_holding_account_id)
            .balance,
        vault_holding_before_withdraw - withdraw_amount
    );
    assert_eq!(
        wr.vault
            .state
            .get_account_by_id(wr.recipient_account_id)
            .balance,
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
    assert_eq!(
        vault_config_state_after.version,
        vault_config_state_before.version
    );
    assert_eq!(
        vault_config_state_after.owner,
        vault_config_state_before.owner
    );
    assert_eq!(
        vault_config_state_after.vault_id,
        vault_config_state_before.vault_id
    );
    assert_eq!(owner_after_deposit, owner_balance_start - deposit_amount);
}

#[test]
fn test_withdraw_zero_amount_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let withdraw_amount = 0 as Balance;
    let block_withdraw = 2 as BlockId;
    let nonce_withdraw = Nonce(1);

    let mut wr = state_with_initialized_vault_with_recipient(owner_balance_start);
    let account_ids = [
        wr.vault.vault_config_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.owner_account_id,
        wr.recipient_account_id,
    ];

    let owner_balance_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.owner_account_id)
        .balance;
    let vault_holding_balance_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_holding_account_id)
        .balance;
    let recipient_balance_before = wr
        .vault
        .state
        .get_account_by_id(wr.recipient_account_id)
        .balance;
    let vault_config_data_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_config_account_id)
        .data
        .clone();

    let tx_withdraw = build_signed_public_tx(
        wr.vault.program_id,
        Instruction::Withdraw {
            vault_id: wr.vault.vault_id,
            amount: withdraw_amount,
        },
        &account_ids,
        &[nonce_withdraw],
        &[&wr.vault.owner_private_key],
    );

    let result = wr.vault.state.transition_from_public_transaction(
        &tx_withdraw,
        block_withdraw,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ErrorCode::ZeroWithdrawAmount);

    assert_vault_state_unchanged_with_recipient(
        &wr.vault.state,
        wr.vault.owner_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.vault_config_account_id,
        wr.recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_withdraw_wrong_vault_id_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let withdraw_amount = 100 as Balance;
    let block_withdraw = 2 as BlockId;
    let nonce_withdraw = Nonce(1);
    let mut wr = state_with_initialized_vault_with_recipient(owner_balance_start);
    patch_vault_config(
        &mut wr.vault.state,
        wr.vault.vault_config_account_id,
        |vc| {
            vc.vault_id = VaultId::from(999u64);
        },
    );
    let account_ids = [
        wr.vault.vault_config_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.owner_account_id,
        wr.recipient_account_id,
    ];

    let owner_balance_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.owner_account_id)
        .balance;
    let vault_holding_balance_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_holding_account_id)
        .balance;
    let recipient_balance_before = wr
        .vault
        .state
        .get_account_by_id(wr.recipient_account_id)
        .balance;
    let vault_config_data_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_config_account_id)
        .data
        .clone();

    let tx_withdraw = build_signed_public_tx(
        wr.vault.program_id,
        Instruction::Withdraw {
            vault_id: wr.vault.vault_id,
            amount: withdraw_amount,
        },
        &account_ids,
        &[nonce_withdraw],
        &[&wr.vault.owner_private_key],
    );

    let result = wr.vault.state.transition_from_public_transaction(
        &tx_withdraw,
        block_withdraw,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ErrorCode::VaultIdMismatch);

    assert_vault_state_unchanged_with_recipient(
        &wr.vault.state,
        wr.vault.owner_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.vault_config_account_id,
        wr.recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_withdraw_exceeds_unallocated_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let block_deposit = 2 as BlockId;
    let block_withdraw = 3 as BlockId;
    let nonce_deposit = Nonce(1);
    let nonce_withdraw = Nonce(2);
    let deposit_amount = 100 as Balance;
    let withdraw_amount = deposit_amount + 1 as Balance;

    let mut wr = state_with_initialized_vault_with_recipient(owner_balance_start);

    let account_ids_deposit = [
        wr.vault.vault_config_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.owner_account_id,
    ];
    let tx_deposit = build_signed_public_tx(
        wr.vault.program_id,
        Instruction::Deposit {
            vault_id: wr.vault.vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids_deposit,
        &[nonce_deposit],
        &[&wr.vault.owner_private_key],
    );
    assert!(
        wr.vault
            .state
            .transition_from_public_transaction(
                &tx_deposit,
                block_deposit,
                crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP
            )
            .is_ok(),
        "deposit failed"
    );

    let owner_balance_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.owner_account_id)
        .balance;
    let vault_holding_balance_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_holding_account_id)
        .balance;
    let recipient_balance_before = wr
        .vault
        .state
        .get_account_by_id(wr.recipient_account_id)
        .balance;
    let vault_config_data_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_config_account_id)
        .data
        .clone();

    let account_ids_withdraw = [
        wr.vault.vault_config_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.owner_account_id,
        wr.recipient_account_id,
    ];
    let tx_withdraw = build_signed_public_tx(
        wr.vault.program_id,
        Instruction::Withdraw {
            vault_id: wr.vault.vault_id,
            amount: withdraw_amount,
        },
        &account_ids_withdraw,
        &[nonce_withdraw],
        &[&wr.vault.owner_private_key],
    );

    let result = wr.vault.state.transition_from_public_transaction(
        &tx_withdraw,
        block_withdraw,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ErrorCode::InsufficientFunds);

    assert_vault_state_unchanged_with_recipient(
        &wr.vault.state,
        wr.vault.owner_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.vault_config_account_id,
        wr.recipient_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        recipient_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_withdraw_full_unallocated_with_stream_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let withdraw_amount = deposit_amount - allocation;
    let rate = 10 as TokensPerSecond;
    let block_deposit = 2 as BlockId;
    let block_stream = 3 as BlockId;
    let block_withdraw = 4 as BlockId;
    let nonce_deposit = Nonce(1);
    let nonce_stream = Nonce(2);
    let nonce_withdraw = Nonce(3);

    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut wr = state_with_initialized_vault_with_recipient(owner_balance_start);

    force_clock_account_monotonic(&mut wr.vault.state, clock_id, 0, DEFAULT_CLOCK_INITIAL_TS);

    let account_ids_deposit = [
        wr.vault.vault_config_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.owner_account_id,
    ];
    assert!(
        wr.vault
            .state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    wr.vault.program_id,
                    Instruction::Deposit {
                        vault_id: wr.vault.vault_id,
                        amount: deposit_amount,
                        authenticated_transfer_program_id: Program::authenticated_transfer_program(
                        )
                        .id(),
                    },
                    &account_ids_deposit,
                    &[nonce_deposit],
                    &[&wr.vault.owner_private_key],
                ),
                block_deposit,
                crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP
            )
            .is_ok(),
        "deposit failed"
    );

    let stream_pda = derive_stream_pda(wr.vault.program_id, wr.vault.vault_config_account_id, 0);
    let account_ids_create = [
        wr.vault.vault_config_account_id,
        wr.vault.vault_holding_account_id,
        stream_pda,
        wr.vault.owner_account_id,
        clock_id,
    ];
    assert!(
        wr.vault
            .state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    wr.vault.program_id,
                    Instruction::CreateStream {
                        vault_id: wr.vault.vault_id,
                        stream_id: 0,
                        provider: provider_account_id,
                        rate,
                        allocation,
                    },
                    &account_ids_create,
                    &[nonce_stream],
                    &[&wr.vault.owner_private_key],
                ),
                block_stream,
                crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP
            )
            .is_ok(),
        "create_stream failed"
    );

    let vault_config_before = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_config_account_id);
    let vault_config_state_before =
        VaultConfig::from_bytes(&vault_config_before.data).expect("vault config");
    let owner_after_funding = wr
        .vault
        .state
        .get_account_by_id(wr.vault.owner_account_id)
        .balance;
    let vault_holding_before_withdraw = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_holding_account_id)
        .balance;
    let recipient_before_withdraw = wr
        .vault
        .state
        .get_account_by_id(wr.recipient_account_id)
        .balance;

    let account_ids_withdraw = [
        wr.vault.vault_config_account_id,
        wr.vault.vault_holding_account_id,
        wr.vault.owner_account_id,
        wr.recipient_account_id,
    ];
    assert!(
        wr.vault
            .state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    wr.vault.program_id,
                    Instruction::Withdraw {
                        vault_id: wr.vault.vault_id,
                        amount: withdraw_amount,
                    },
                    &account_ids_withdraw,
                    &[nonce_withdraw],
                    &[&wr.vault.owner_private_key],
                ),
                block_withdraw,
                crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP
            )
            .is_ok(),
        "withdraw failed"
    );

    let vault_config_after = wr
        .vault
        .state
        .get_account_by_id(wr.vault.vault_config_account_id);
    let vault_config_state_after =
        VaultConfig::from_bytes(&vault_config_after.data).expect("vault config");

    assert_eq!(
        wr.vault
            .state
            .get_account_by_id(wr.vault.owner_account_id)
            .balance,
        owner_after_funding
    );
    assert_eq!(
        wr.vault
            .state
            .get_account_by_id(wr.vault.vault_holding_account_id)
            .balance,
        vault_holding_before_withdraw - withdraw_amount
    );
    assert_eq!(
        wr.vault
            .state
            .get_account_by_id(wr.recipient_account_id)
            .balance,
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
    assert_eq!(
        vault_config_state_after.version,
        vault_config_state_before.version
    );
    assert_eq!(
        vault_config_state_after.owner,
        vault_config_state_before.owner
    );
    assert_eq!(
        vault_config_state_after.vault_id,
        vault_config_state_before.vault_id
    );
    assert_eq!(owner_after_funding, owner_balance_start - deposit_amount);
}

#[test]
fn test_withdraw_owner_mismatch_fails() {
    let signer_account_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let recipient_genesis_balance = 0 as Balance;
    let withdraw_amount = 50 as Balance;
    let block_init = 1 as BlockId;
    let block_withdraw = 2 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_withdraw = Nonce(1);

    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let (_, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);
    let (_, recipient_account_id) = create_keypair(SEED_RECIPIENT);
    let initial_accounts_data = vec![
        (owner_account_id, signer_account_balance),
        (alt_signer_account_id, signer_account_balance),
        (recipient_account_id, recipient_genesis_balance),
    ];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect(
            "guest image present (cargo build -p lez_payment_streams-methods) and state genesis ok",
        );
    let program_id = guest_program.id();

    let vault_id = VaultId::from(1u64);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids_init = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];
    let tx_init = build_signed_public_tx(
        program_id,
        Instruction::initialize_vault_public(vault_id),
        &account_ids_init,
        &[nonce_init],
        &[&owner_private_key],
    );
    assert!(
        state
            .transition_from_public_transaction(
                &tx_init,
                block_init,
                crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP
            )
            .is_ok(),
        "initialize_vault tx failed"
    );

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let alt_signer_balance_before = state.get_account_by_id(alt_signer_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;
    let recipient_balance_before = state.get_account_by_id(recipient_account_id).balance;

    patch_vault_config(&mut state, vault_config_account_id, |vc| {
        vc.owner = alt_signer_account_id;
    });
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

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

    let result = state.transition_from_public_transaction(
        &tx_withdraw,
        block_withdraw,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ErrorCode::VaultOwnerMismatch);

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
        state.get_account_by_id(alt_signer_account_id).balance,
        alt_signer_balance_before
    );
}

#[test]
fn test_withdraw_recipient_balance_overflow_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = 100 as Balance;
    let withdraw_amount = 10 as Balance;
    let mut wr = state_with_initialized_vault_with_recipient(owner_balance_start);

    assert!(
        wr.vault
            .state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    wr.vault.program_id,
                    Instruction::Deposit {
                        vault_id: wr.vault.vault_id,
                        amount: deposit_amount,
                        authenticated_transfer_program_id: Program::authenticated_transfer_program(
                        )
                        .id(),
                    },
                    &[
                        wr.vault.vault_config_account_id,
                        wr.vault.vault_holding_account_id,
                        wr.vault.owner_account_id,
                    ],
                    &[Nonce(1)],
                    &[&wr.vault.owner_private_key],
                ),
                2 as BlockId,
                crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP
            )
            .is_ok(),
        "deposit failed"
    );

    let mut recipient = wr
        .vault
        .state
        .get_account_by_id(wr.recipient_account_id)
        .clone();
    recipient.balance = Balance::MAX - 5;
    wr.vault
        .state
        .force_insert_account(wr.recipient_account_id, recipient);

    let result = wr.vault.state.transition_from_public_transaction(
        &build_signed_public_tx(
            wr.vault.program_id,
            Instruction::Withdraw {
                vault_id: wr.vault.vault_id,
                amount: withdraw_amount,
            },
            &[
                wr.vault.vault_config_account_id,
                wr.vault.vault_holding_account_id,
                wr.vault.owner_account_id,
                wr.recipient_account_id,
            ],
            &[Nonce(2)],
            &[&wr.vault.owner_private_key],
        ),
        3 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ErrorCode::ArithmeticOverflow);
}

#[test]
fn test_withdraw_recipient_not_present_in_state_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = 100 as Balance;
    let withdraw_amount = 10 as Balance;
    let mut wr = state_with_initialized_vault_with_recipient(owner_balance_start);
    let (_, unknown_recipient_id) = create_keypair(0x99);

    assert!(
        wr.vault
            .state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    wr.vault.program_id,
                    Instruction::Deposit {
                        vault_id: wr.vault.vault_id,
                        amount: deposit_amount,
                        authenticated_transfer_program_id: Program::authenticated_transfer_program(
                        )
                        .id(),
                    },
                    &[
                        wr.vault.vault_config_account_id,
                        wr.vault.vault_holding_account_id,
                        wr.vault.owner_account_id,
                    ],
                    &[Nonce(1)],
                    &[&wr.vault.owner_private_key],
                ),
                2 as BlockId,
                crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP
            )
            .is_ok(),
        "deposit failed"
    );

    let result = wr.vault.state.transition_from_public_transaction(
        &build_signed_public_tx(
            wr.vault.program_id,
            Instruction::Withdraw {
                vault_id: wr.vault.vault_id,
                amount: withdraw_amount,
            },
            &[
                wr.vault.vault_config_account_id,
                wr.vault.vault_holding_account_id,
                wr.vault.owner_account_id,
                unknown_recipient_id,
            ],
            &[Nonce(2)],
            &[&wr.vault.owner_private_key],
        ),
        3 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result.is_err(),
        "withdraw to an account id absent from public state should fail: {result:?}"
    );
}
