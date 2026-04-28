//! `close_stream` payouts and authorization.

use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::{
    test_helpers::{create_keypair, derive_stream_pda, force_clock_account_monotonic},
    error_codes::ErrorCode, StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond,
    VaultConfig, CLOCK_01_PROGRAM_ACCOUNT_ID,
};

use super::common::{
    assert_execution_failed_with_code, force_stream_state_closed, signed_close_stream,
    signed_create_stream, state_deposited_with_clock, transition_ok,
    CloseStreamIxAccounts, DEFAULT_CLOCK_INITIAL_TS, DEFAULT_OWNER_GENESIS_BALANCE,
    DEFAULT_STREAM_TEST_DEPOSIT, TEST_PUBLIC_TX_TIMESTAMP,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_PROVIDER};

#[test]
fn test_close_unaccrued_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;

    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut dep = state_deposited_with_clock(owner_balance_start, deposit_amount, clock_id, t0);

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(
        dep.vault.program_id,
        dep.vault.vault_config_account_id,
        stream_id,
    );

    let stream_accounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &stream_accounts,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    let vault_before = VaultConfig::from_bytes(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_before.total_allocated, allocation);

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    let close_accounts: CloseStreamIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        provider_account_id,
        clock_id,
    ];

    transition_ok(
        &mut dep.vault.state,
        &signed_close_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &provider_private_key,
        ),
        5 as BlockId,
        "close_stream failed",
    );

    let vault_after = VaultConfig::from_bytes(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_after.total_allocated, 50 as Balance);

    let stream_after =
        StreamConfig::from_bytes(&dep.vault.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    assert_eq!(stream_after.state, StreamState::Closed);
    assert_eq!(stream_after.allocation, 50 as Balance);
    assert_eq!(stream_after.accrued, 50 as Balance);

    let holding_balance = dep
        .vault
        .state
        .get_account_by_id(dep.vault.vault_holding_account_id)
        .balance;
    let unallocated = holding_balance.saturating_sub(vault_after.total_allocated);
    assert_eq!(unallocated, 450 as Balance);
}

#[test]
fn test_close_stream_unauthorized_fails() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;

    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (_, provider_account_id) = create_keypair(SEED_PROVIDER);
    let (alt_signer_private_key, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);

    let mut dep = state_deposited_with_clock(owner_balance_start, deposit_amount, clock_id, t0);

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(
        dep.vault.program_id,
        dep.vault.vault_config_account_id,
        stream_id,
    );

    let stream_accounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            rate,
            allocation,
            &stream_accounts,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    let close_accounts: CloseStreamIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        alt_signer_account_id,
        clock_id,
    ];

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_close_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &alt_signer_private_key,
        ),
        5 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::CloseUnauthorized);
}

#[test]
fn test_close_already_closed_fails() {
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        DEFAULT_CLOCK_INITIAL_TS,
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(
        dep.vault.program_id,
        dep.vault.vault_config_account_id,
        stream_id,
    );

    let stream_accounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    transition_ok(
        &mut dep.vault.state,
        &signed_create_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            provider_account_id,
            1 as TokensPerSecond,
            100 as Balance,
            &stream_accounts,
            Nonce(2),
            &dep.vault.owner_private_key,
        ),
        3 as BlockId,
        "create_stream failed",
    );

    force_stream_state_closed(&mut dep.vault.state, stream_pda);

    let close_accounts: CloseStreamIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        provider_account_id,
        clock_id,
    ];

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_close_stream(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &provider_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::StreamClosed);
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
    program::Program,
};
use nssa_core::{
    account::{AccountId, AccountWithMetadata},
    encryption::EphemeralPublicKey,
    Commitment, EncryptionScheme, SharedSecretKey,
};
use crate::test_helpers::load_guest_program;
use super::pp_common::{
    account_meta, pp_claim_close_setup, PpClaimCloseSetup,
    recipient_npk, recipient_vpk,
    RECIPIENT_NSK, EPK_SCALAR,
    PP_T0, PP_T1, PP_STREAM_RATE, PP_STREAM_ALLOCATION, PP_WITHDRAW_AMOUNT,
};

#[test]
fn test_pp_close_stream_private_provider_authority_succeeds() {
    let PpClaimCloseSetup {
        mut fx,
        stream_id,
        stream_pda,
        provider_committed_account,
    } = pp_claim_close_setup();

    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    force_clock_account_monotonic(&mut fx.state, clock_id, 2, PP_T1);

    let guest_program = load_guest_program();
    assert_eq!(guest_program.id(), fx.program_id);

    let authority_npk = recipient_npk();
    let authority_id = AccountId::from(&authority_npk);
    let authority_commitment = Commitment::new(&authority_npk, &provider_committed_account);
    let membership_proof = fx
        .state
        .get_proof_for_commitment(&authority_commitment)
        .expect("authority commitment not found in state after PP withdraw");

    let pre_states = vec![
        account_meta(&fx.state, fx.vault_config_account_id, false),
        account_meta(&fx.state, fx.vault_holding_account_id, false),
        account_meta(&fx.state, stream_pda, false),
        account_meta(&fx.state, fx.owner_account_id, false),
        AccountWithMetadata {
            account: provider_committed_account.clone(),
            is_authorized: true,
            account_id: authority_id,
        },
        account_meta(&fx.state, clock_id, false),
    ];

    let authority_shared_secret = SharedSecretKey::new(&EPK_SCALAR, &recipient_vpk());
    let authority_epk = EphemeralPublicKey::from_scalar(EPK_SCALAR);

    let vault_total_allocated_before = VaultConfig::from_bytes(
        &fx.state.get_account_by_id(fx.vault_config_account_id).data,
    )
    .expect("vault config")
    .total_allocated;

    let (output, proof) = execute_and_prove(
        pre_states,
        Program::serialize_instruction(crate::Instruction::CloseStream {
            vault_id: fx.vault_id,
            stream_id,
        })
        .expect("close_stream instruction serializes"),
        vec![0u8, 0, 0, 0, 1, 0],
        vec![(authority_npk.clone(), authority_shared_secret)],
        vec![RECIPIENT_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(guest_program),
    )
    .expect("execute_and_prove close_stream");

    let message = Message::try_from_circuit_output(
        vec![
            fx.vault_config_account_id,
            fx.vault_holding_account_id,
            stream_pda,
            fx.owner_account_id,
            clock_id,
        ],
        vec![],
        vec![(authority_npk, recipient_vpk(), authority_epk)],
        output,
    )
    .expect("try_from_circuit_output for close_stream");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    fx.state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("close_stream PP transition");

    let stream_after =
        StreamConfig::from_bytes(&fx.state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(stream_after.state, StreamState::Closed);
    let accrued_at_t1 = PP_STREAM_RATE as Balance * (PP_T1 - PP_T0) as Balance;
    assert_eq!(stream_after.allocation, accrued_at_t1);
    assert_eq!(stream_after.accrued, accrued_at_t1);

    let unaccrued = PP_STREAM_ALLOCATION - accrued_at_t1;
    let vault_after =
        VaultConfig::from_bytes(&fx.state.get_account_by_id(fx.vault_config_account_id).data)
            .expect("vault");
    assert_eq!(
        vault_after.total_allocated,
        vault_total_allocated_before - unaccrued
    );

    assert_eq!(tx.message().new_commitments.len(), 1);
    assert_eq!(tx.message().encrypted_private_post_states.len(), 1);
    let new_commitment = &tx.message().new_commitments[0];
    let decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &authority_shared_secret,
        new_commitment,
        0,
    )
    .expect("decrypt authority post-state after close_stream");
    assert_eq!(decrypted.balance, PP_WITHDRAW_AMOUNT);
}
