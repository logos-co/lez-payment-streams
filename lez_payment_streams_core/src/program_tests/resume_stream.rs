//! `resume_stream` success and failure cases.

use nssa::program::Program;
use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        create_keypair, create_state_with_guest_program, derive_stream_pda, derive_vault_pdas,
        force_clock_account_monotonic, harness_clock_01_and_provider_account_ids,
        patch_vault_config,
    },
    error_codes::ErrorCode, StreamConfig, StreamState, Timestamp, TokensPerSecond, VaultId,
};

use super::common::{
    assert_execution_failed_with_code, first_stream_ix_accounts, force_stream_state_closed,
    signed_create_stream, signed_pause_stream, signed_resume_stream,
    state_deposited_with_clock, transition_ok, DEFAULT_CLOCK_INITIAL_TS,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_OWNER};
use crate::test_helpers::build_signed_public_tx;

#[test]
fn test_resume_succeeds() {
    let t0: Timestamp = 100;
    let t1: Timestamp = 200;
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
            5 as TokensPerSecond,
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
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "resume_stream failed",
    );

    let s_resumed = StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data)
        .expect("stream");
    assert_eq!(s_resumed.state, StreamState::Active);
    assert_eq!(s_resumed.accrued, 0 as Balance);
    assert_eq!(s_resumed.accrued_as_of, t1);
}

#[test]
fn test_resume_active_fails() {
    let t0: Timestamp = 50;
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
            400 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::StreamNotPaused);
}

#[test]
fn test_resume_zero_remaining_fails() {
    let t0: Timestamp = 0;
    let t1: Timestamp = 100;
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
            10 as TokensPerSecond,
            100 as Balance,
            &account_ids,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::ResumeZeroUnaccrued);
}

#[test]
fn test_resume_twice_fails() {
    let t0: Timestamp = 10;
    let t1: Timestamp = 20;
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
            500 as Balance,
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
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "resume_stream failed",
    );

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(5),
            &dep.vault.owner_private_key,
        ),
        6 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::StreamNotPaused);
}

#[test]
fn test_resume_closed_fails() {
    let t0: Timestamp = 8;
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

    force_stream_state_closed(&mut dep.vault.state, stream_pda);

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::StreamNotPaused);
}

#[test]
fn test_resume_then_accrual_ignores_paused_gap_succeeds() {
    let t0: Timestamp = 100;
    let t1: Timestamp = 105;
    let t_gap: Timestamp = 200;
    let (clock_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );

    let (stream_id, stream_pda, account_ids) = first_stream_ix_accounts(&dep);

    let rate = 10 as TokensPerSecond;
    let allocation = 500 as Balance;
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
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

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t_gap);
    transition_ok(
        &mut dep.vault.state,
        &signed_resume_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &account_ids,
            Nonce(4),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "resume_stream failed",
    );

    // Verify resume sets accrued_as_of = t_gap (gap is not counted).
    // pause_stream folds at_time(t1) internally: accrued = rate*(t1-t0) = 10*5 = 50.
    // resume_from_paused_at(t_gap) sets accrued_as_of = t_gap, leaving accrued unchanged.
    let s_after_resume =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    assert_eq!(s_after_resume.accrued, 50 as Balance);
    assert_eq!(s_after_resume.accrued_as_of, t_gap);
    assert_eq!(s_after_resume.state, StreamState::Active);
}

#[test]
fn test_resume_stream_owner_mismatch_fails() {
    let signer_account_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let block_init = 1 as BlockId;
    let block_deposit = 2 as BlockId;
    let block_stream = 3 as BlockId;
    let block_pause = 4 as BlockId;
    let block_resume = 5 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_deposit = Nonce(1);
    let nonce_stream = Nonce(2);
    let nonce_pause = Nonce(3);
    let nonce_resume = Nonce(4);

    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let (_, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);
    let (clock_account_id, provider_account_id) = harness_clock_01_and_provider_account_ids();

    let initial_accounts_data = vec![
        (owner_account_id, signer_account_balance),
        (alt_signer_account_id, signer_account_balance),
    ];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect("guest present and state genesis ok");
    let program_id = guest_program.id();

    let vault_id = VaultId::from(1u64);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    transition_ok(
        &mut state,
        &build_signed_public_tx(
            program_id,
            Instruction::initialize_vault_public(vault_id),
            &[
                vault_config_account_id,
                vault_holding_account_id,
                owner_account_id,
            ],
            &[nonce_init],
            &[&owner_private_key],
        ),
        block_init,
        "initialize_vault failed",
    );

    transition_ok(
        &mut state,
        &build_signed_public_tx(
            program_id,
            Instruction::Deposit {
                vault_id,
                amount: deposit_amount,
                authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
            },
            &[
                vault_config_account_id,
                vault_holding_account_id,
                owner_account_id,
            ],
            &[nonce_deposit],
            &[&owner_private_key],
        ),
        block_deposit,
        "deposit failed",
    );

    force_clock_account_monotonic(&mut state, clock_account_id, 0, DEFAULT_CLOCK_INITIAL_TS);

    let stream_pda = derive_stream_pda(program_id, vault_config_account_id, 0);
    transition_ok(
        &mut state,
        &signed_create_stream(
            program_id,
            vault_id,
            0,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &[
                vault_config_account_id,
                vault_holding_account_id,
                stream_pda,
                owner_account_id,
                clock_account_id,
            ],
            nonce_stream,
            &owner_private_key,
        ),
        block_stream,
        "create_stream failed",
    );

    let account_ids_pause = [
        vault_config_account_id,
        vault_holding_account_id,
        stream_pda,
        owner_account_id,
        clock_account_id,
    ];
    transition_ok(
        &mut state,
        &signed_pause_stream(
            program_id,
            vault_id,
            0,
            &account_ids_pause,
            nonce_pause,
            &owner_private_key,
        ),
        block_pause,
        "pause_stream failed",
    );

    patch_vault_config(&mut state, vault_config_account_id, |vc| {
        vc.owner = alt_signer_account_id;
    });

    let r = state.transition_from_public_transaction(
        &signed_resume_stream(
            program_id,
            vault_id,
            0,
            &account_ids_pause,
            nonce_resume,
            &owner_private_key,
        ),
        block_resume,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::VaultOwnerMismatch);
}

// ---- PP tests ---- //

use nssa::{
    execute_and_prove,
    privacy_preserving_transaction::{
        circuit::ProgramWithDependencies,
        message::Message,
        witness_set::WitnessSet,
        PrivacyPreservingTransaction,
    },
};
use nssa_core::{
    account::{Account, AccountId, AccountWithMetadata, Data},
    encryption::EphemeralPublicKey,
    Commitment, EncryptionScheme, SharedSecretKey,
};
use crate::{test_helpers::load_guest_program, CLOCK_01_PROGRAM_ACCOUNT_ID};
use super::common::TEST_PUBLIC_TX_TIMESTAMP;
use super::pp_common::{
    account_meta, owner_vpk, pp_owner_setup, recipient_npk, PpOwnerSetup,
    OWNER_NSK, PP3_OWNER_FUND_AMOUNT, PP3_SIGNER_EPK_SCALAR, PP3_STREAM_ALLOCATION,
    PP3_STREAM_RATE, PP3_T0, PP3_T1,
};

#[test]
fn test_pp_resume_stream_private_owner_succeeds() {
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
    let accrued = PP3_STREAM_RATE as Balance * (PP3_T1 - PP3_T0) as Balance;

    let mut stream_config = StreamConfig::new(
        stream_id,
        provider_id,
        PP3_STREAM_RATE,
        PP3_STREAM_ALLOCATION,
        PP3_T0,
        None,
    );
    stream_config.state = StreamState::Paused;
    stream_config.accrued = accrued;
    let stream_account = Account {
        program_owner: fx.program_id,
        balance: 0,
        data: Data::try_from(stream_config.to_bytes()).expect("stream config fits"),
        ..Account::default()
    };
    fx.state.force_insert_account(stream_pda, stream_account);

    patch_vault_config(&mut fx.state, vault_config_b_id, |cfg| {
        cfg.next_stream_id = 1;
        cfg.total_allocated = PP3_STREAM_ALLOCATION;
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
        Program::serialize_instruction(Instruction::ResumeStream {
            vault_id: vault_b_id,
            stream_id,
        })
        .expect("resume_stream instruction serializes"),
        vec![0u8, 0, 0, 1, 0],
        vec![(owner_npk.clone(), owner_shared_secret)],
        vec![OWNER_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(load_guest_program()),
    )
    .expect("execute_and_prove: PP resume_stream");

    let message = Message::try_from_circuit_output(
        vec![vault_config_b_id, vault_holding_b_id, stream_pda, clock_id],
        vec![],
        vec![(owner_npk.clone(), owner_vpk(), owner_epk)],
        output,
    )
    .expect("try_from_circuit_output: resume_stream");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    fx.state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("resume_stream PP transition");

    let stream =
        StreamConfig::from_bytes(&fx.state.get_account_by_id(stream_pda).data)
            .expect("stream config after resume");
    assert_eq!(stream.state, StreamState::Active);
    assert_eq!(stream.accrued, accrued);
    assert_eq!(stream.allocation, PP3_STREAM_ALLOCATION);

    assert_eq!(tx.message().new_commitments.len(), 1);
    let decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &owner_shared_secret,
        &tx.message().new_commitments[0],
        0,
    )
    .expect("decrypt owner post-state after resume_stream");
    assert_eq!(decrypted.balance, PP3_OWNER_FUND_AMOUNT);
}
