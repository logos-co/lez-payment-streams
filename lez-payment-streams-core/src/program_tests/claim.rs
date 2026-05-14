//! `claim` payout accounting, provider authorization, and claims from active/closed streams.

use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::{
    error_codes::ErrorCode, test_helpers::create_keypair, StreamConfig, StreamState, Timestamp,
    TokensPerSecond, VaultConfig, CLOCK_01_PROGRAM_ACCOUNT_ID,
};

use super::common::{
    assert_execution_failed_with_code, claim_stream_prelude_at_t1, signed_claim_stream,
    signed_close_stream, transition_ok, ClaimStreamIxAccounts, CloseStreamIxAccounts,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT, TEST_PUBLIC_TX_TIMESTAMP,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_PROVIDER};

const CLAIM_T0: Timestamp = 12_345;
const CLAIM_T1: Timestamp = CLAIM_T0 + 5;
const CLAIM_ALLOCATION: Balance = 200;
const CLAIM_RATE: TokensPerSecond = 10;

#[test]
fn test_claim_balance_succeeds() {
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut scenario = claim_stream_prelude_at_t1(
        DEFAULT_OWNER_GENESIS_BALANCE,
        deposit_amount,
        clock_id,
        CLAIM_T0,
        CLAIM_T1,
        provider_private_key,
        provider_account_id,
        CLAIM_RATE,
        CLAIM_ALLOCATION,
    );
    let wp = &mut scenario.with_provider;
    let stream_id = scenario.stream_id;
    let stream_pda = scenario.stream_pda;

    let provider_balance_before = wp
        .deposited
        .vault
        .state
        .get_account_by_id(wp.provider_account_id)
        .balance;

    let claim_accounts: ClaimStreamIxAccounts = [
        wp.deposited.vault.vault_config_account_id,
        wp.deposited.vault.vault_holding_account_id,
        stream_pda,
        wp.deposited.vault.owner_account_id,
        wp.provider_account_id,
        wp.deposited.clock_id,
    ];

    transition_ok(
        &mut wp.deposited.vault.state,
        &signed_claim_stream(
            wp.deposited.vault.program_id,
            wp.deposited.vault.vault_id,
            stream_id,
            &claim_accounts,
            Nonce(0),
            &wp.provider_private_key,
        ),
        4 as BlockId,
        "claim failed",
    );

    let payout = 50 as Balance;
    let vault_after = borsh::from_slice::<VaultConfig>(
        &wp.deposited
            .vault
            .state
            .get_account_by_id(wp.deposited.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_after.total_allocated, CLAIM_ALLOCATION - payout);

    let stream_after = borsh::from_slice::<StreamConfig>(
        &wp.deposited.vault.state.get_account_by_id(stream_pda).data,
    )
    .expect("stream");
    assert_eq!(stream_after.state, StreamState::Active);
    assert_eq!(stream_after.accrued, 0 as Balance);
    assert_eq!(stream_after.allocation, CLAIM_ALLOCATION - payout);

    let holding_after = wp
        .deposited
        .vault
        .state
        .get_account_by_id(wp.deposited.vault.vault_holding_account_id)
        .balance;
    assert_eq!(holding_after, deposit_amount - payout);

    let provider_balance_after = wp
        .deposited
        .vault
        .state
        .get_account_by_id(wp.provider_account_id)
        .balance;
    assert_eq!(
        provider_balance_after,
        provider_balance_before.saturating_add(payout)
    );
}

#[test]
fn test_claim_unauthorized_fails() {
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);
    let (alt_signer_private_key, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);

    let mut scenario = claim_stream_prelude_at_t1(
        DEFAULT_OWNER_GENESIS_BALANCE,
        deposit_amount,
        clock_id,
        CLAIM_T0,
        CLAIM_T1,
        provider_private_key,
        provider_account_id,
        CLAIM_RATE,
        CLAIM_ALLOCATION,
    );
    let wp = &mut scenario.with_provider;
    let stream_id = scenario.stream_id;
    let stream_pda = scenario.stream_pda;

    let claim_accounts: ClaimStreamIxAccounts = [
        wp.deposited.vault.vault_config_account_id,
        wp.deposited.vault.vault_holding_account_id,
        stream_pda,
        wp.deposited.vault.owner_account_id,
        alt_signer_account_id,
        wp.deposited.clock_id,
    ];

    let r = wp.deposited.vault.state.transition_from_public_transaction(
        &signed_claim_stream(
            wp.deposited.vault.program_id,
            wp.deposited.vault.vault_id,
            stream_id,
            &claim_accounts,
            Nonce(0),
            &alt_signer_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::ClaimUnauthorized);
}

#[test]
fn test_claim_after_close_succeeds() {
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut scenario = claim_stream_prelude_at_t1(
        DEFAULT_OWNER_GENESIS_BALANCE,
        deposit_amount,
        clock_id,
        CLAIM_T0,
        CLAIM_T1,
        provider_private_key,
        provider_account_id,
        CLAIM_RATE,
        CLAIM_ALLOCATION,
    );
    let wp = &mut scenario.with_provider;
    let stream_id = scenario.stream_id;
    let stream_pda = scenario.stream_pda;

    let close_accounts: CloseStreamIxAccounts = [
        wp.deposited.vault.vault_config_account_id,
        wp.deposited.vault.vault_holding_account_id,
        stream_pda,
        wp.deposited.vault.owner_account_id,
        wp.provider_account_id,
        wp.deposited.clock_id,
    ];

    transition_ok(
        &mut wp.deposited.vault.state,
        &signed_close_stream(
            wp.deposited.vault.program_id,
            wp.deposited.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &wp.provider_private_key,
        ),
        4 as BlockId,
        "close_stream failed",
    );

    let claim_accounts: ClaimStreamIxAccounts = [
        wp.deposited.vault.vault_config_account_id,
        wp.deposited.vault.vault_holding_account_id,
        stream_pda,
        wp.deposited.vault.owner_account_id,
        wp.provider_account_id,
        wp.deposited.clock_id,
    ];

    transition_ok(
        &mut wp.deposited.vault.state,
        &signed_claim_stream(
            wp.deposited.vault.program_id,
            wp.deposited.vault.vault_id,
            stream_id,
            &claim_accounts,
            Nonce(1),
            &wp.provider_private_key,
        ),
        5 as BlockId,
        "claim failed",
    );

    let vault_after = borsh::from_slice::<VaultConfig>(
        &wp.deposited
            .vault
            .state
            .get_account_by_id(wp.deposited.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_after.total_allocated, 0 as Balance);

    let stream_after = borsh::from_slice::<StreamConfig>(
        &wp.deposited.vault.state.get_account_by_id(stream_pda).data,
    )
    .expect("stream");
    assert_eq!(stream_after.state, StreamState::Closed);
    assert_eq!(stream_after.allocation, 0 as Balance);
    assert_eq!(stream_after.accrued, 0 as Balance);

    let r = wp.deposited.vault.state.transition_from_public_transaction(
        &signed_claim_stream(
            wp.deposited.vault.program_id,
            wp.deposited.vault.vault_id,
            stream_id,
            &claim_accounts,
            Nonce(2),
            &wp.provider_private_key,
        ),
        6 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::ZeroClaimAmount);
}

// ---- PP tests ---- //

use super::pp_common::{
    account_meta, pp_claim_close_setup, recipient_npk, recipient_vpk, PpClaimCloseSetup,
    EPK_SCALAR, PP_CLAIM_PAYOUT, PP_STREAM_ALLOCATION, PP_T1, PP_WITHDRAW_AMOUNT, RECIPIENT_NSK,
};
use crate::test_helpers::{force_clock_account_monotonic, load_guest_program};
use nssa::{
    execute_and_prove,
    privacy_preserving_transaction::{
        circuit::ProgramWithDependencies, message::Message, witness_set::WitnessSet,
        PrivacyPreservingTransaction,
    },
    program::Program,
};
use nssa_core::{
    account::{AccountId, AccountWithMetadata},
    encryption::EphemeralPublicKey,
    Commitment, EncryptionScheme, SharedSecretKey,
};

#[test]
fn test_pp_claim_private_provider_succeeds() {
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

    let provider_npk = recipient_npk();
    let provider_id = AccountId::from(&provider_npk);
    let provider_commitment = Commitment::new(&provider_npk, &provider_committed_account);
    let membership_proof = fx
        .state
        .get_proof_for_commitment(&provider_commitment)
        .expect("provider commitment not found in state after PP withdraw");

    let pre_states = vec![
        account_meta(&fx.state, fx.vault_config_account_id, false),
        account_meta(&fx.state, fx.vault_holding_account_id, false),
        account_meta(&fx.state, stream_pda, false),
        account_meta(&fx.state, fx.owner_account_id, false),
        AccountWithMetadata {
            account: provider_committed_account.clone(),
            is_authorized: true,
            account_id: provider_id,
        },
        account_meta(&fx.state, clock_id, false),
    ];

    let provider_shared_secret = SharedSecretKey::new(&EPK_SCALAR, &recipient_vpk());
    let provider_epk = EphemeralPublicKey::from_scalar(EPK_SCALAR);

    let (output, proof) = execute_and_prove(
        pre_states,
        Program::serialize_instruction(crate::Instruction::Claim {
            vault_id: fx.vault_id,
            stream_id,
        })
        .expect("claim instruction serializes"),
        vec![0u8, 0, 0, 0, 1, 0],
        vec![(provider_npk.clone(), provider_shared_secret)],
        vec![RECIPIENT_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(guest_program),
    )
    .expect("execute_and_prove claim");

    let holding_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;

    let message = Message::try_from_circuit_output(
        vec![
            fx.vault_config_account_id,
            fx.vault_holding_account_id,
            stream_pda,
            fx.owner_account_id,
            clock_id,
        ],
        vec![],
        vec![(provider_npk, recipient_vpk(), provider_epk)],
        output,
    )
    .expect("try_from_circuit_output for claim");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    fx.state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("claim PP transition");

    assert_eq!(
        fx.state
            .get_account_by_id(fx.vault_holding_account_id)
            .balance,
        holding_before - PP_CLAIM_PAYOUT
    );

    let stream_after =
        borsh::from_slice::<StreamConfig>(&fx.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    assert_eq!(stream_after.accrued, 0);
    assert_eq!(
        stream_after.allocation,
        PP_STREAM_ALLOCATION - PP_CLAIM_PAYOUT
    );
    assert_eq!(stream_after.state, StreamState::Active);

    let vault_after = borsh::from_slice::<VaultConfig>(
        &fx.state.get_account_by_id(fx.vault_config_account_id).data,
    )
    .expect("vault");
    assert_eq!(
        vault_after.total_allocated,
        PP_STREAM_ALLOCATION - PP_CLAIM_PAYOUT
    );

    assert_eq!(tx.message().new_commitments.len(), 1);
    assert_eq!(tx.message().encrypted_private_post_states.len(), 1);
    let new_commitment = &tx.message().new_commitments[0];
    let decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &provider_shared_secret,
        new_commitment,
        0,
    )
    .expect("decrypt provider post-state after claim");
    assert_eq!(decrypted.balance, PP_WITHDRAW_AMOUNT + PP_CLAIM_PAYOUT);
}
