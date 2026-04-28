//! `withdraw` to third-party recipients.

use nssa::{
    execute_and_prove,
    privacy_preserving_transaction::{
        circuit::ProgramWithDependencies,
        message::Message,
        witness_set::WitnessSet,
        PrivacyPreservingTransaction,
    },
    program::Program,
};
use nssa_core::{
    account::{Account, AccountId, AccountWithMetadata, Balance, Nonce},
    encryption::EphemeralPublicKey,
    BlockId, Commitment, EncryptionScheme, SharedSecretKey,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        assert_vault_state_unchanged_with_recipient, build_signed_public_tx, create_keypair,
        create_state_with_guest_program, derive_stream_pda, derive_vault_pdas,
        force_clock_account_monotonic, harness_clock_01_and_provider_account_ids,
        patch_vault_config, state_with_initialized_vault_with_recipient,
    },
    error_codes::ErrorCode, TokensPerSecond, VaultConfig, VaultId, VaultPrivacyTier,
};
use crate::test_helpers::load_guest_program;

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

// ---- PP tests ---- //

use super::pp_common::{
    account_meta, owner_vpk, pp3_recipient_npk, pp3_recipient_vpk, pp_owner_setup,
    run_pp_withdraw_to_private_recipient, vault_fixture_pseudonymous_funder_funded_via_native_transfer,
    vault_fixture_public_tier_funded_via_deposit,
    OWNER_NSK, PP3_OWNER_FUND_AMOUNT, PP3_RECIPIENT_EPK_SCALAR, PP3_SIGNER_EPK_SCALAR,
    PP3_WITHDRAW_AMOUNT,
};

#[test]
fn test_withdraw_private_recipient_pp_transition_succeeds() {
    let withdraw_amount = 100 as Balance;
    let block_withdraw = 3 as BlockId;

    let mut fx = vault_fixture_public_tier_funded_via_deposit();
    let holding_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;
    let owner_before = fx.state.get_account_by_id(fx.owner_account_id);

    let receipt = run_pp_withdraw_to_private_recipient(&mut fx, withdraw_amount, block_withdraw);

    assert_eq!(
        fx.state
            .get_account_by_id(fx.vault_holding_account_id)
            .balance,
        holding_before - withdraw_amount
    );
    let owner_after = fx.state.get_account_by_id(fx.owner_account_id);
    assert_eq!(owner_after.balance, owner_before.balance);
    let mut expected_nonce = owner_before.nonce;
    expected_nonce.public_account_nonce_increment();
    assert_eq!(owner_after.nonce, expected_nonce);

    let cfg = VaultConfig::from_bytes(&fx.state.get_account_by_id(fx.vault_config_account_id).data)
        .expect("vault");
    assert_eq!(cfg.total_allocated, 0u128);

    assert_eq!(receipt.tx.message().new_commitments.len(), 1);
    let commitment = receipt.tx.message().new_commitments[0].clone();
    let ciphertext = &receipt.tx.message().encrypted_private_post_states[0].ciphertext;
    let decrypted = EncryptionScheme::decrypt(ciphertext, &receipt.shared_secret, &commitment, 0)
        .expect("decrypt private withdraw_to post-state");
    assert_eq!(decrypted.balance, withdraw_amount);
}

#[test]
fn test_pp_withdraw_private_recipient_pseudonymous_funded_vault_succeeds() {
    let withdraw_amount = 100 as Balance;
    let block_withdraw = 3 as BlockId;

    let mut fx = vault_fixture_pseudonymous_funder_funded_via_native_transfer();
    let holding_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;
    let owner_before = fx.state.get_account_by_id(fx.owner_account_id);

    let _receipt = run_pp_withdraw_to_private_recipient(&mut fx, withdraw_amount, block_withdraw);

    assert_eq!(
        fx.state
            .get_account_by_id(fx.vault_holding_account_id)
            .balance,
        holding_before - withdraw_amount
    );
    let owner_after = fx.state.get_account_by_id(fx.owner_account_id);
    assert_eq!(owner_after.balance, owner_before.balance);
    let mut expected_nonce = owner_before.nonce;
    expected_nonce.public_account_nonce_increment();
    assert_eq!(owner_after.nonce, expected_nonce);

    let cfg = VaultConfig::from_bytes(&fx.state.get_account_by_id(fx.vault_config_account_id).data)
        .expect("vault");
    assert_eq!(cfg.privacy_tier, VaultPrivacyTier::PseudonymousFunder);
    assert_eq!(cfg.total_allocated, 0u128);
}

#[test]
fn test_pp_withdraw_private_owner_succeeds() {
    let mut setup = pp_owner_setup();

    let recipient_npk_val = pp3_recipient_npk();
    let recipient_id = AccountId::from(&recipient_npk_val);

    let owner_commitment_obj = Commitment::new(&setup.owner_npk, &setup.owner_committed_account);
    let membership_proof = setup
        .fx
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment in state after PP withdraw");

    let owner_shared_secret = SharedSecretKey::new(&PP3_SIGNER_EPK_SCALAR, &owner_vpk());
    let owner_epk = EphemeralPublicKey::from_scalar(PP3_SIGNER_EPK_SCALAR);
    let recipient_shared_secret =
        SharedSecretKey::new(&PP3_RECIPIENT_EPK_SCALAR, &pp3_recipient_vpk());
    let recipient_epk = EphemeralPublicKey::from_scalar(PP3_RECIPIENT_EPK_SCALAR);

    let holding_before = setup.fx.state.get_account_by_id(setup.vault_holding_b_id).balance;

    let pre_states = vec![
        account_meta(&setup.fx.state, setup.vault_config_b_id, false),
        account_meta(&setup.fx.state, setup.vault_holding_b_id, false),
        AccountWithMetadata {
            account: setup.owner_committed_account.clone(),
            is_authorized: true,
            account_id: AccountId::from(&setup.owner_npk),
        },
        AccountWithMetadata {
            account: Account::default(),
            is_authorized: false,
            account_id: recipient_id,
        },
    ];

    let (output, proof) = execute_and_prove(
        pre_states,
        Program::serialize_instruction(Instruction::Withdraw {
            vault_id: setup.vault_b_id,
            amount: PP3_WITHDRAW_AMOUNT,
        })
        .expect("withdraw instruction serializes"),
        vec![0u8, 0, 1, 2],
        vec![
            (setup.owner_npk.clone(), owner_shared_secret),
            (recipient_npk_val.clone(), recipient_shared_secret),
        ],
        vec![OWNER_NSK],
        vec![Some(membership_proof), None],
        &ProgramWithDependencies::from(load_guest_program()),
    )
    .expect("execute_and_prove: PP withdraw private owner");

    let message = Message::try_from_circuit_output(
        vec![setup.vault_config_b_id, setup.vault_holding_b_id],
        vec![],
        vec![
            (setup.owner_npk.clone(), owner_vpk(), owner_epk),
            (recipient_npk_val.clone(), pp3_recipient_vpk(), recipient_epk),
        ],
        output,
    )
    .expect("try_from_circuit_output: withdraw private owner");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    setup
        .fx
        .state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, super::common::TEST_PUBLIC_TX_TIMESTAMP)
        .expect("withdraw private owner PP transition");

    assert_eq!(
        setup.fx.state.get_account_by_id(setup.vault_holding_b_id).balance,
        holding_before - PP3_WITHDRAW_AMOUNT
    );

    assert_eq!(tx.message().new_commitments.len(), 2);
    assert_eq!(tx.message().encrypted_private_post_states.len(), 2);

    let owner_decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &owner_shared_secret,
        &tx.message().new_commitments[0],
        0,
    )
    .expect("decrypt owner post-state after withdraw");
    assert_eq!(owner_decrypted.balance, PP3_OWNER_FUND_AMOUNT);

    // `output_index` increments for each private slot in account order:
    // owner (vis-1, mask index 2) → 0; recipient (vis-2, mask index 3) → 1.
    let recipient_decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[1].ciphertext,
        &recipient_shared_secret,
        &tx.message().new_commitments[1],
        1,
    )
    .expect("decrypt recipient post-state after withdraw");
    assert_eq!(recipient_decrypted.balance, PP3_WITHDRAW_AMOUNT);
}
