//! `top_up_stream` allocation increases, fold-before-top-up behavior, and pause/resume handling.

use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::{
    error_codes::ErrorCode,
    test_helpers::{
        force_clock_account_monotonic, harness_clock_01_and_provider_account_ids,
        patch_vault_config,
    },
    StreamConfig, StreamState, Timestamp, TokensPerSecond,
};

use super::common::{
    assert_execution_failed_with_code, first_stream_ix_accounts, force_stream_state_closed,
    signed_create_stream, signed_deposit, signed_pause_stream, signed_top_up_stream,
    state_deposited_with_clock, transition_ok, DEFAULT_CLOCK_INITIAL_TS,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};

#[test]
fn test_topup_paused_depleted_stream_succeeds() {
    let t0: Timestamp = 0;
    let t2: Timestamp = 250;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            10 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    transition_ok(
        &mut dep.vault.state,
        &signed_deposit(
            dep.vault.program_id,
            dep.vault.vault_id,
            200 as Balance,
            dep.vault.vault_config_account_id,
            dep.vault.vault_holding_account_id,
            dep.vault.owner_account_id,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        "deposit failed",
    );

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t2);

    // top_up folds at_time(t2) internally: stream was depleted at t=10 (rate=10, alloc=100),
    // so fold gives Paused/accrued=100. top_up then adds 200 and activates at t2.
    transition_ok(
        &mut dep.vault.state,
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            200 as Balance,
            &account_ids,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "top_up_stream failed",
    );

    let s_after_top_up =
        borsh::from_slice::<StreamConfig>(&dep.vault.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    assert_eq!(s_after_top_up.state, StreamState::Active);
    assert_eq!(s_after_top_up.accrued_as_of, t2);
    assert_eq!(s_after_top_up.allocation, 300 as Balance);
    assert_eq!(s_after_top_up.accrued, 100 as Balance);
}

#[test]
fn test_topup_active_stream_increases_allocation_succeeds() {
    let t0: Timestamp = 10;
    let t1: Timestamp = 20;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            2 as TokensPerSecond,
            300 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    transition_ok(
        &mut dep.vault.state,
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            100 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        "top_up_stream failed",
    );

    let s_after_top_up =
        borsh::from_slice::<StreamConfig>(&dep.vault.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    assert_eq!(s_after_top_up.state, StreamState::Active);
    assert_eq!(s_after_top_up.allocation, 400 as Balance);
}

#[test]
fn test_topup_manual_pause_then_active_succeeds() {
    let t0: Timestamp = 5;
    let t1: Timestamp = 15;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            400 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    transition_ok(
        &mut dep.vault.state,
        &signed_pause_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        "pause_stream failed",
    );

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    transition_ok(
        &mut dep.vault.state,
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            50 as Balance,
            &account_ids,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "top_up_stream failed",
    );

    let s_resumed_after_top_up =
        borsh::from_slice::<StreamConfig>(&dep.vault.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    assert_eq!(s_resumed_after_top_up.state, StreamState::Active);
    assert_eq!(s_resumed_after_top_up.accrued_as_of, t1);
    assert_eq!(s_resumed_after_top_up.allocation, 450 as Balance);
}

#[test]
fn test_topup_zero_fails() {
    let t0 = DEFAULT_CLOCK_INITIAL_TS;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, _, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            0 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::ZeroTopUpAmount);
}

#[test]
fn test_topup_closed_fails() {
    let t0 = DEFAULT_CLOCK_INITIAL_TS;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            2 as TokensPerSecond,
            200 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_stream_state_closed(&mut dep.vault.state, stream_pda);

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            10 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::StreamClosed);
}

#[test]
fn test_topup_exceeds_unallocated_fails() {
    let t0 = DEFAULT_CLOCK_INITIAL_TS;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, _, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            DEFAULT_STREAM_TEST_DEPOSIT,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            1 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::AllocationExceedsUnallocated);
}

#[test]
fn test_topup_allocation_overflow_fails() {
    let t0 = DEFAULT_CLOCK_INITIAL_TS;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let mut s_near_max_allocation =
        borsh::from_slice::<StreamConfig>(&dep.vault.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    s_near_max_allocation.allocation = Balance::MAX - 5;
    s_near_max_allocation.accrued = 0 as Balance;
    let mut stream_account = dep.vault.state.get_account_by_id(stream_pda).clone();
    stream_account.data =
        nssa_core::account::Data::try_from(borsh::to_vec(&s_near_max_allocation).unwrap())
            .expect("stream payload fits");
    dep.vault
        .state
        .force_insert_account(stream_pda, stream_account);

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            20 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::ArithmeticOverflow);
}

#[test]
fn test_top_up_stream_owner_mismatch_fails() {
    let t0 = DEFAULT_CLOCK_INITIAL_TS;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, _, account_ids) = first_stream_ix_accounts(&dep);

    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    patch_vault_config(
        &mut dep.vault.state,
        dep.vault.vault_config_account_id,
        |vc| {
            vc.owner = provider_account_id;
        },
    );

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_top_up_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            1 as Balance,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::VaultOwnerMismatch);
}

// ---- PP tests ---- //

use super::common::TEST_PUBLIC_TX_TIMESTAMP;
use super::pp_common::{
    account_meta, owner_vpk, pp_owner_setup, recipient_npk, PpOwnerSetup, OWNER_NSK,
    PP3_OWNER_FUND_AMOUNT, PP3_SIGNER_EPK_SCALAR, PP3_STREAM_RATE, PP3_T0, PP3_TOP_UP_AMOUNT,
};
use crate::Instruction;
use crate::{
    test_helpers::{derive_stream_pda, load_guest_program},
    VaultConfig, CLOCK_01_PROGRAM_ACCOUNT_ID,
};
use nssa::{
    execute_and_prove,
    privacy_preserving_transaction::{
        circuit::ProgramWithDependencies, message::Message, witness_set::WitnessSet,
        PrivacyPreservingTransaction,
    },
    program::Program,
};
use nssa_core::{
    account::{Account, AccountId, AccountWithMetadata, Data},
    encryption::EphemeralPublicKey,
    Commitment, EncryptionScheme, SharedSecretKey,
};

#[test]
fn test_pp_top_up_stream_private_owner_succeeds() {
    let PpOwnerSetup {
        mut fx,
        vault_b_id,
        vault_config_b_id,
        vault_holding_b_id,
        owner_committed_account,
        owner_npk,
    } = pp_owner_setup();

    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let stream_id = 0u64;
    let stream_pda = derive_stream_pda(fx.program_id, vault_config_b_id, stream_id);
    let provider_id = AccountId::from(&recipient_npk());
    let depleted_allocation: Balance = PP3_TOP_UP_AMOUNT;

    let mut stream_config = StreamConfig::new(
        stream_id,
        provider_id,
        PP3_STREAM_RATE,
        depleted_allocation,
        PP3_T0,
        None,
    );
    stream_config.state = StreamState::Paused;
    stream_config.accrued = depleted_allocation;
    let stream_account = Account {
        program_owner: fx.program_id,
        balance: 0,
        data: Data::try_from(borsh::to_vec(&stream_config).unwrap()).expect("stream config fits"),
        ..Account::default()
    };
    fx.state.force_insert_account(stream_pda, stream_account);

    patch_vault_config(&mut fx.state, vault_config_b_id, |cfg| {
        cfg.next_stream_id = 1;
        cfg.total_allocated = depleted_allocation;
    });

    let owner_commitment_obj = Commitment::new(&owner_npk, &owner_committed_account);
    let membership_proof = fx
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment in state after PP withdraw");

    let owner_shared_secret = SharedSecretKey::new(&PP3_SIGNER_EPK_SCALAR, &owner_vpk());
    let owner_epk = EphemeralPublicKey::from_scalar(PP3_SIGNER_EPK_SCALAR);

    let pre_states = vec![
        account_meta(&fx.state, vault_config_b_id, false),
        account_meta(&fx.state, vault_holding_b_id, false),
        account_meta(&fx.state, stream_pda, false),
        AccountWithMetadata {
            account: owner_committed_account.clone(),
            is_authorized: true,
            account_id: AccountId::from(&owner_npk),
        },
        account_meta(&fx.state, clock_id, false),
    ];

    let (output, proof) = execute_and_prove(
        pre_states,
        Program::serialize_instruction(Instruction::TopUpStream {
            vault_id: vault_b_id,
            stream_id,
            vault_total_allocated_increase: PP3_TOP_UP_AMOUNT,
        })
        .expect("top_up_stream instruction serializes"),
        vec![0u8, 0, 0, 1, 0],
        vec![(owner_npk.clone(), owner_shared_secret)],
        vec![OWNER_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(load_guest_program()),
    )
    .expect("execute_and_prove: PP top_up_stream");

    let message = Message::try_from_circuit_output(
        vec![vault_config_b_id, vault_holding_b_id, stream_pda, clock_id],
        vec![],
        vec![(owner_npk.clone(), owner_vpk(), owner_epk)],
        output,
    )
    .expect("try_from_circuit_output: top_up_stream");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    fx.state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("top_up_stream PP transition");

    let stream = borsh::from_slice::<StreamConfig>(&fx.state.get_account_by_id(stream_pda).data)
        .expect("stream config after top_up");
    assert_eq!(stream.state, StreamState::Active);
    assert_eq!(stream.allocation, depleted_allocation + PP3_TOP_UP_AMOUNT);
    assert_eq!(stream.accrued, depleted_allocation);

    let vault =
        borsh::from_slice::<VaultConfig>(&fx.state.get_account_by_id(vault_config_b_id).data)
            .expect("vault config after top_up");
    assert_eq!(
        vault.total_allocated,
        depleted_allocation + PP3_TOP_UP_AMOUNT
    );

    assert_eq!(tx.message().new_commitments.len(), 1);
    let decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &owner_shared_secret,
        &tx.message().new_commitments[0],
        0,
    )
    .expect("decrypt owner post-state after top_up_stream");
    assert_eq!(decrypted.balance, PP3_OWNER_FUND_AMOUNT);
}
