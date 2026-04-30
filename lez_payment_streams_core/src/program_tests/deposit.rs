//! `deposit` and `authenticated_transfer` wiring.

use nssa::program::Program;
use nssa_core::{
    account::{Balance, Data, Nonce},
    BlockId,
};

use crate::Instruction;
use crate::{
    error_codes::ErrorCode,
    test_helpers::{
        assert_vault_state_unchanged, build_signed_public_tx, create_keypair,
        create_state_with_guest_program, derive_stream_pda, derive_vault_pdas,
        harness_clock_01_and_provider_account_ids, patch_vault_config,
        state_with_initialized_vault,
    },
    TokensPerSecond, VaultConfig, VaultHolding, VaultId, VersionId,
};

use super::common::{
    assert_execution_failed_with_code, state_deposited_with_clock, DEFAULT_CLOCK_INITIAL_TS,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT, TEST_PUBLIC_TX_TIMESTAMP,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_OWNER};

#[test]
fn test_deposit_succeeds() {
    let owner_balance_before = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = 300 as Balance;
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);

    let mut fx = state_with_initialized_vault(owner_balance_before);
    let account_ids = [
        fx.vault_config_account_id,
        fx.vault_holding_account_id,
        fx.owner_account_id,
    ];

    let vault_config_before = fx.state.get_account_by_id(fx.vault_config_account_id);
    let vault_config_state_before = borsh::from_slice::<VaultConfig>(&vault_config_before.data)
        .expect("valid vault config bytes");
    let instruction_deposit = Instruction::Deposit {
        vault_id: fx.vault_id,
        amount: deposit_amount,
        authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
    };
    let tx_deposit = build_signed_public_tx(
        fx.program_id,
        instruction_deposit,
        &account_ids,
        &[nonce_deposit],
        &[&fx.owner_private_key],
    );

    let vault_holding_balance_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;

    let result_deposit = fx.state.transition_from_public_transaction(
        &tx_deposit,
        block_deposit,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result_deposit.is_ok(),
        "deposit tx failed: {:?}",
        result_deposit
    );

    let owner_balance_after = fx.state.get_account_by_id(fx.owner_account_id).balance;
    let vault_holding_balance_after = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;
    let vault_config_after = fx.state.get_account_by_id(fx.vault_config_account_id);
    let vault_config_state_after = borsh::from_slice::<VaultConfig>(&vault_config_after.data)
        .expect("valid vault config bytes");

    assert_eq!(owner_balance_after, owner_balance_before - deposit_amount);
    assert_eq!(
        vault_holding_balance_after,
        vault_holding_balance_before + deposit_amount
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
}

#[test]
fn test_deposit_after_create_stream_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let initial_deposit = DEFAULT_STREAM_TEST_DEPOSIT;
    let second_deposit = 50 as Balance;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        owner_balance_start,
        initial_deposit,
        clock_id,
        DEFAULT_CLOCK_INITIAL_TS,
    );

    let stream_pda = derive_stream_pda(dep.vault.program_id, dep.vault.vault_config_account_id, 0);
    let account_ids_create = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    assert!(
        dep.vault
            .state
            .transition_from_public_transaction(
                &build_signed_public_tx(
                    dep.vault.program_id,
                    Instruction::CreateStream {
                        vault_id: dep.vault.vault_id,
                        stream_id: 0,
                        provider: provider_account_id,
                        rate,
                        allocation,
                    },
                    &account_ids_create,
                    &[Nonce(2)],
                    &[&dep.vault.owner_private_key],
                ),
                3 as BlockId,
                crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP
            )
            .is_ok(),
        "create_stream failed"
    );

    let vc_after_stream = borsh::from_slice::<VaultConfig>(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    let stream_data_after_create = dep.vault.state.get_account_by_id(stream_pda).data.clone();
    let owner_before_second = dep
        .vault
        .state
        .get_account_by_id(dep.vault.owner_account_id)
        .balance;
    let holding_before_second = dep
        .vault
        .state
        .get_account_by_id(dep.vault.vault_holding_account_id)
        .balance;

    let account_ids_deposit = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        dep.vault.owner_account_id,
    ];
    let result = dep.vault.state.transition_from_public_transaction(
        &build_signed_public_tx(
            dep.vault.program_id,
            Instruction::Deposit {
                vault_id: dep.vault.vault_id,
                amount: second_deposit,
                authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
            },
            &account_ids_deposit,
            &[Nonce(3)],
            &[&dep.vault.owner_private_key],
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(result.is_ok(), "second deposit failed: {:?}", result);

    let vc_after = borsh::from_slice::<VaultConfig>(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vc_after.total_allocated, vc_after_stream.total_allocated);
    assert_eq!(vc_after.next_stream_id, vc_after_stream.next_stream_id);
    assert_eq!(
        dep.vault
            .state
            .get_account_by_id(dep.vault.owner_account_id)
            .balance,
        owner_before_second - second_deposit
    );
    assert_eq!(
        dep.vault
            .state
            .get_account_by_id(dep.vault.vault_holding_account_id)
            .balance,
        holding_before_second + second_deposit
    );
    assert_eq!(
        dep.vault.state.get_account_by_id(stream_pda).data,
        stream_data_after_create
    );
}

#[test]
fn test_deposit_zero_amount_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let deposit_amount = 0 as Balance;

    let mut fx = state_with_initialized_vault(owner_balance_start);
    let account_ids = [
        fx.vault_config_account_id,
        fx.vault_holding_account_id,
        fx.owner_account_id,
    ];

    let owner_balance_before = fx.state.get_account_by_id(fx.owner_account_id).balance;
    let vault_holding_balance_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;
    let vault_config_data_before = fx
        .state
        .get_account_by_id(fx.vault_config_account_id)
        .data
        .clone();

    let tx_deposit = build_signed_public_tx(
        fx.program_id,
        Instruction::Deposit {
            vault_id: fx.vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids,
        &[nonce_deposit],
        &[&fx.owner_private_key],
    );

    let result = fx.state.transition_from_public_transaction(
        &tx_deposit,
        block_deposit,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ErrorCode::ZeroDepositAmount);

    assert_vault_state_unchanged(
        &fx.state,
        fx.owner_account_id,
        fx.vault_holding_account_id,
        fx.vault_config_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_deposit_wrong_vault_id_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let deposit_amount = 100 as Balance;

    let mut fx = state_with_initialized_vault(owner_balance_start);
    patch_vault_config(&mut fx.state, fx.vault_config_account_id, |vc| {
        vc.vault_id = VaultId::from(999u64);
    });
    let account_ids = [
        fx.vault_config_account_id,
        fx.vault_holding_account_id,
        fx.owner_account_id,
    ];

    let owner_balance_before = fx.state.get_account_by_id(fx.owner_account_id).balance;
    let vault_holding_balance_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;
    let vault_config_data_before = fx
        .state
        .get_account_by_id(fx.vault_config_account_id)
        .data
        .clone();

    let tx_deposit = build_signed_public_tx(
        fx.program_id,
        Instruction::Deposit {
            vault_id: fx.vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids,
        &[nonce_deposit],
        &[&fx.owner_private_key],
    );

    let result = fx.state.transition_from_public_transaction(
        &tx_deposit,
        block_deposit,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ErrorCode::VaultIdMismatch);

    assert_vault_state_unchanged(
        &fx.state,
        fx.owner_account_id,
        fx.vault_holding_account_id,
        fx.vault_config_account_id,
        owner_balance_before,
        vault_holding_balance_before,
        vault_config_data_before,
    );
}

#[test]
fn test_deposit_wrong_authenticated_transfer_program_id_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);
    let deposit_amount = 100 as Balance;

    let mut fx = state_with_initialized_vault(owner_balance_start);
    let account_ids = [
        fx.vault_config_account_id,
        fx.vault_holding_account_id,
        fx.owner_account_id,
    ];

    let owner_balance_before = fx.state.get_account_by_id(fx.owner_account_id).balance;
    let vault_holding_balance_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;
    let vault_config_data_before = fx
        .state
        .get_account_by_id(fx.vault_config_account_id)
        .data
        .clone();

    // Chained transfer must target `Program::authenticated_transfer_program().id()`; using the
    // payment-streams guest `program_id` here is deliberately wrong and fails chained execution.
    let tx_deposit = build_signed_public_tx(
        fx.program_id,
        Instruction::Deposit {
            vault_id: fx.vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: fx.program_id,
        },
        &account_ids,
        &[nonce_deposit],
        &[&fx.owner_private_key],
    );

    let result = fx.state.transition_from_public_transaction(
        &tx_deposit,
        block_deposit,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    // Failure is from the chained authenticated-transfer program, not a payment-streams `ERR_*`.
    assert!(result.is_err());

    assert_vault_state_unchanged(
        &fx.state,
        fx.owner_account_id,
        fx.vault_holding_account_id,
        fx.vault_config_account_id,
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

    let mut fx = state_with_initialized_vault(genesis_owner_balance);
    let account_ids = [
        fx.vault_config_account_id,
        fx.vault_holding_account_id,
        fx.owner_account_id,
    ];

    let vault_holding_balance_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;
    let vault_config_data_before = fx
        .state
        .get_account_by_id(fx.vault_config_account_id)
        .data
        .clone();

    let tx_deposit = build_signed_public_tx(
        fx.program_id,
        Instruction::Deposit {
            vault_id: fx.vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids,
        &[nonce_deposit],
        &[&fx.owner_private_key],
    );

    let result = fx.state.transition_from_public_transaction(
        &tx_deposit,
        block_deposit,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    // Insufficient balance is enforced inside authenticated-transfer, not a lez custom code.
    assert!(result.is_err());

    assert_vault_state_unchanged(
        &fx.state,
        fx.owner_account_id,
        fx.vault_holding_account_id,
        fx.vault_config_account_id,
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
    let signer_account_balance = DEFAULT_OWNER_GENESIS_BALANCE;

    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let (_, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);
    let initial_accounts_data = vec![
        (owner_account_id, signer_account_balance),
        (alt_signer_account_id, signer_account_balance),
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
    let result_init = state.transition_from_public_transaction(
        &tx_init,
        block_init,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result_init.is_ok(),
        "initialize_vault tx failed: {:?}",
        result_init
    );

    let owner_balance_before = state.get_account_by_id(owner_account_id).balance;
    let alt_signer_balance_before = state.get_account_by_id(alt_signer_account_id).balance;
    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;

    patch_vault_config(&mut state, vault_config_account_id, |vc| {
        vc.owner = alt_signer_account_id;
    });
    let vault_config_data_before = state
        .get_account_by_id(vault_config_account_id)
        .data
        .clone();

    let account_ids_deposit = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
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
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(
        &tx_deposit,
        block_deposit,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ErrorCode::VaultOwnerMismatch);

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
        state.get_account_by_id(alt_signer_account_id).balance,
        alt_signer_balance_before
    );
}

#[test]
fn test_deposit_vault_holding_version_mismatch_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = 10 as Balance;
    let mut fx = state_with_initialized_vault(owner_balance_start);

    let mut holding = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .clone();
    holding.data = Data::try_from(borsh::to_vec(&VaultHolding::new(Some(2 as VersionId))).unwrap())
        .expect("vault holding payload fits Data limits");
    fx.state
        .force_insert_account(fx.vault_holding_account_id, holding);

    let result = fx.state.transition_from_public_transaction(
        &build_signed_public_tx(
            fx.program_id,
            Instruction::Deposit {
                vault_id: fx.vault_id,
                amount: deposit_amount,
                authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
            },
            &[
                fx.vault_config_account_id,
                fx.vault_holding_account_id,
                fx.owner_account_id,
            ],
            &[Nonce(1)],
            &[&fx.owner_private_key],
        ),
        2 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(result, ErrorCode::VersionMismatch);
}

// ---- PP tests ---- //

use super::pp_common::{
    account_meta, load_payment_streams_with_auth_transfer, owner_npk, owner_vpk,
    vault_fixture_public_tier_funded_via_deposit, OWNER_FUND_EPK_SCALAR, OWNER_NSK,
    PP_DEPOSIT_AMOUNT, PP_DEPOSIT_EPK_SCALAR, PP_OWNER_FUND_AMOUNT,
};
use crate::VaultPrivacyTier;
use nssa::{
    execute_and_prove,
    privacy_preserving_transaction::{
        circuit::ProgramWithDependencies, message::Message, witness_set::WitnessSet,
        PrivacyPreservingTransaction,
    },
};
use nssa_core::{
    account::{Account, AccountId, AccountWithMetadata},
    encryption::EphemeralPublicKey,
    Commitment, EncryptionScheme, MembershipProof, SharedSecretKey,
};

#[test]
fn test_pp_deposit_private_owner_succeeds() {
    let mut fx_a = vault_fixture_public_tier_funded_via_deposit();

    let owner_npk = owner_npk();
    let owner_id = AccountId::from(&owner_npk);
    let owner_fund_shared_secret = SharedSecretKey::new(&OWNER_FUND_EPK_SCALAR, &owner_vpk());
    let owner_fund_epk = EphemeralPublicKey::from_scalar(OWNER_FUND_EPK_SCALAR);

    let fx_a_sender_before = fx_a.state.get_account_by_id(fx_a.owner_account_id);

    let auth_transfer_program = Program::authenticated_transfer_program();
    let pre_states_fund = vec![
        account_meta(&fx_a.state, fx_a.owner_account_id, true),
        AccountWithMetadata {
            account: Account::default(),
            is_authorized: false,
            account_id: owner_id,
        },
    ];

    let (fund_output, fund_proof) = execute_and_prove(
        pre_states_fund,
        Program::serialize_instruction(PP_OWNER_FUND_AMOUNT)
            .expect("serialize auth_transfer amount"),
        vec![0u8, 2],
        vec![(owner_npk.clone(), owner_fund_shared_secret)],
        vec![],
        vec![None::<MembershipProof>],
        &ProgramWithDependencies::from(auth_transfer_program),
    )
    .expect("execute_and_prove: fund owner via PP auth_transfer");

    let fund_message = Message::try_from_circuit_output(
        vec![fx_a.owner_account_id],
        vec![fx_a_sender_before.nonce],
        vec![(owner_npk.clone(), owner_vpk(), owner_fund_epk)],
        fund_output,
    )
    .expect("try_from_circuit_output: fund owner");

    let fund_witness =
        WitnessSet::for_message(&fund_message, fund_proof, &[&fx_a.owner_private_key]);
    let fund_tx = PrivacyPreservingTransaction::new(fund_message, fund_witness);

    fx_a.state
        .transition_from_privacy_preserving_transaction(
            &fund_tx,
            3 as BlockId,
            TEST_PUBLIC_TX_TIMESTAMP,
        )
        .expect("transition: fund owner via PP auth_transfer");

    let owner_commitment = &fund_tx.message().new_commitments[0];
    let owner_committed_account = EncryptionScheme::decrypt(
        &fund_tx.message().encrypted_private_post_states[0].ciphertext,
        &owner_fund_shared_secret,
        owner_commitment,
        0,
    )
    .expect("decrypt owner state from PP auth_transfer");
    assert_eq!(owner_committed_account.balance, PP_OWNER_FUND_AMOUNT);

    let vault_b_id = 2u64;
    let (vault_config_b_id, vault_holding_b_id) =
        derive_vault_pdas(fx_a.program_id, owner_id, vault_b_id);

    let vault_config_b = Account {
        program_owner: fx_a.program_id,
        balance: 0,
        data: Data::try_from(
            borsh::to_vec(&VaultConfig::new(
                owner_id,
                vault_b_id,
                None,
                Some(VaultPrivacyTier::PseudonymousFunder),
            ))
            .unwrap(),
        )
        .expect("vault_config_b data fits"),
        ..Account::default()
    };
    fx_a.state
        .force_insert_account(vault_config_b_id, vault_config_b);

    let vault_holding_b = Account {
        program_owner: fx_a.program_id,
        balance: 0,
        data: Data::try_from(borsh::to_vec(&VaultHolding::new(None)).unwrap())
            .expect("vault_holding_b data fits"),
        ..Account::default()
    };
    fx_a.state
        .force_insert_account(vault_holding_b_id, vault_holding_b);

    let owner_commitment_obj = Commitment::new(&owner_npk, &owner_committed_account);
    let membership_proof = fx_a
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment not in state after PP auth_transfer");

    let deposit_shared_secret = SharedSecretKey::new(&PP_DEPOSIT_EPK_SCALAR, &owner_vpk());
    let deposit_epk = EphemeralPublicKey::from_scalar(PP_DEPOSIT_EPK_SCALAR);

    let pre_states_deposit = vec![
        account_meta(&fx_a.state, vault_config_b_id, false),
        account_meta(&fx_a.state, vault_holding_b_id, false),
        AccountWithMetadata {
            account: owner_committed_account.clone(),
            is_authorized: true,
            account_id: owner_id,
        },
    ];

    let (deposit_output, deposit_proof) = execute_and_prove(
        pre_states_deposit,
        Program::serialize_instruction(Instruction::Deposit {
            vault_id: vault_b_id,
            amount: PP_DEPOSIT_AMOUNT,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        })
        .expect("deposit instruction serializes"),
        vec![0u8, 0, 1],
        vec![(owner_npk.clone(), deposit_shared_secret)],
        vec![OWNER_NSK],
        vec![Some(membership_proof)],
        &load_payment_streams_with_auth_transfer(),
    )
    .expect("execute_and_prove: PP deposit");

    let holding_b_before = fx_a.state.get_account_by_id(vault_holding_b_id).balance;

    let deposit_message = Message::try_from_circuit_output(
        vec![vault_config_b_id, vault_holding_b_id],
        vec![],
        vec![(owner_npk, owner_vpk(), deposit_epk)],
        deposit_output,
    )
    .expect("try_from_circuit_output: deposit");

    let deposit_witness = WitnessSet::for_message(&deposit_message, deposit_proof, &[]);
    let deposit_tx = PrivacyPreservingTransaction::new(deposit_message, deposit_witness);

    fx_a.state
        .transition_from_privacy_preserving_transaction(
            &deposit_tx,
            4 as BlockId,
            TEST_PUBLIC_TX_TIMESTAMP,
        )
        .expect("PP deposit transition");

    assert_eq!(
        fx_a.state.get_account_by_id(vault_holding_b_id).balance,
        holding_b_before + PP_DEPOSIT_AMOUNT
    );

    assert_eq!(deposit_tx.message().new_commitments.len(), 1);
    assert_eq!(deposit_tx.message().encrypted_private_post_states.len(), 1);
    let new_commitment = &deposit_tx.message().new_commitments[0];
    let decrypted = EncryptionScheme::decrypt(
        &deposit_tx.message().encrypted_private_post_states[0].ciphertext,
        &deposit_shared_secret,
        new_commitment,
        0,
    )
    .expect("decrypt owner post-state after deposit");
    assert_eq!(decrypted.balance, PP_OWNER_FUND_AMOUNT - PP_DEPOSIT_AMOUNT);
}
