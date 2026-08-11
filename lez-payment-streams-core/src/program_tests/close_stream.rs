//! `close_stream_by_owner` / `close_stream_by_provider` authorization, accounting, and PP coverage.

use lee_core::{
    account::{Balance, Nonce},
    BlockId,
};

use crate::{
    error_codes::ErrorCode,
    test_helpers::{create_keypair, derive_stream_pda, force_clock_account_monotonic},
    StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultConfig,
    CLOCK_01_PROGRAM_ACCOUNT_ID,
};

use super::common::{
    assert_execution_failed_with_code, force_stream_state_closed, signed_close_stream_by_owner,
    signed_close_stream_by_provider, signed_create_stream, state_deposited_with_clock, transition_ok,
    CloseStreamByOwnerIxAccounts, CloseStreamByProviderIxAccounts, DEFAULT_CLOCK_INITIAL_TS,
    DEFAULT_OWNER_GENESIS_BALANCE, DEFAULT_STREAM_TEST_DEPOSIT,
};
use crate::harness_seeds::{SEED_ALT_SIGNER, SEED_PROVIDER};

fn create_active_stream(
    dep: &mut super::common::DepositedVaultFixture,
    provider_account_id: lee_core::account::AccountId,
    rate: TokensPerSecond,
    allocation: Balance,
    clock_id: lee_core::account::AccountId,
) -> (StreamId, lee_core::account::AccountId) {
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
    (stream_id, stream_pda)
}

#[test]
fn test_close_stream_by_provider_unaccrued_succeeds() {
    let owner_balance_start = DEFAULT_OWNER_GENESIS_BALANCE;
    let deposit_amount = DEFAULT_STREAM_TEST_DEPOSIT;
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;

    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut dep = state_deposited_with_clock(owner_balance_start, deposit_amount, clock_id, t0);
    let (stream_id, stream_pda) =
        create_active_stream(&mut dep, provider_account_id, rate, allocation, clock_id);

    let vault_before = borsh::from_slice::<VaultConfig>(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_before.total_allocated, allocation);

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    let close_accounts: CloseStreamByProviderIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        provider_account_id,
        clock_id,
    ];

    transition_ok(
        &mut dep.vault.state,
        &signed_close_stream_by_provider(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(0),
            &provider_private_key,
        ),
        5 as BlockId,
        "close_stream_by_provider failed",
    );

    let vault_after = borsh::from_slice::<VaultConfig>(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_after.total_allocated, 50 as Balance);

    let stream_after =
        borsh::from_slice::<StreamConfig>(&dep.vault.state.get_account_by_id(stream_pda).data)
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
fn test_close_stream_by_owner_unaccrued_succeeds() {
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (_, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );
    let (stream_id, stream_pda) =
        create_active_stream(&mut dep, provider_account_id, rate, allocation, clock_id);

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    let close_accounts: CloseStreamByOwnerIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];

    transition_ok(
        &mut dep.vault.state,
        &signed_close_stream_by_owner(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        5 as BlockId,
        "close_stream_by_owner failed",
    );

    let vault_after = borsh::from_slice::<VaultConfig>(
        &dep.vault
            .state
            .get_account_by_id(dep.vault.vault_config_account_id)
            .data,
    )
    .expect("vault config");
    assert_eq!(vault_after.total_allocated, 50 as Balance);

    let stream_after =
        borsh::from_slice::<StreamConfig>(&dep.vault.state.get_account_by_id(stream_pda).data)
            .expect("stream");
    assert_eq!(stream_after.state, StreamState::Closed);
    assert_eq!(stream_after.allocation, 50 as Balance);
    assert_eq!(stream_after.accrued, 50 as Balance);
}

#[test]
fn test_close_stream_by_provider_unauthorized_fails() {
    let allocation = 200 as Balance;
    let rate = 10 as TokensPerSecond;
    let t0: Timestamp = 12_345;
    let t1: Timestamp = t0 + 5;

    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (_, provider_account_id) = create_keypair(SEED_PROVIDER);
    let (alt_signer_private_key, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        t0,
    );
    let (stream_id, stream_pda) =
        create_active_stream(&mut dep, provider_account_id, rate, allocation, clock_id);

    force_clock_account_monotonic(&mut dep.vault.state, clock_id, 0, t1);

    let close_accounts: CloseStreamByProviderIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        alt_signer_account_id,
        clock_id,
    ];

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_close_stream_by_provider(
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
fn test_close_stream_by_owner_wrong_signer_fails() {
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (_, provider_account_id) = create_keypair(SEED_PROVIDER);
    let (_, alt_signer_account_id) = create_keypair(SEED_ALT_SIGNER);

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        DEFAULT_CLOCK_INITIAL_TS,
    );
    let (stream_id, stream_pda) = create_active_stream(
        &mut dep,
        provider_account_id,
        1 as TokensPerSecond,
        100 as Balance,
        clock_id,
    );

    crate::test_helpers::patch_vault_config(
        &mut dep.vault.state,
        dep.vault.vault_config_account_id,
        |vc| {
            vc.owner = alt_signer_account_id;
        },
    );

    let close_accounts: CloseStreamByOwnerIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_close_stream_by_owner(
            dep.vault.program_id,
            dep.vault.vault_id,
            stream_id,
            &close_accounts,
            Nonce(3),
            &dep.vault.owner_private_key,
        ),
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert_execution_failed_with_code(r, ErrorCode::VaultOwnerMismatch);
}

#[test]
fn test_close_stream_by_provider_already_closed_fails() {
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        DEFAULT_CLOCK_INITIAL_TS,
    );
    let (stream_id, stream_pda) = create_active_stream(
        &mut dep,
        provider_account_id,
        1 as TokensPerSecond,
        100 as Balance,
        clock_id,
    );

    force_stream_state_closed(&mut dep.vault.state, stream_pda);

    let close_accounts: CloseStreamByProviderIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        provider_account_id,
        clock_id,
    ];

    let r = dep.vault.state.transition_from_public_transaction(
        &signed_close_stream_by_provider(
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

#[test]
fn test_close_stream_by_owner_wrong_account_count_fails() {
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        DEFAULT_CLOCK_INITIAL_TS,
    );
    let (stream_id, stream_pda) = create_active_stream(
        &mut dep,
        provider_account_id,
        1 as TokensPerSecond,
        100 as Balance,
        clock_id,
    );

    // Six-slot provider layout against the five-slot owner instruction.
    let wrong_accounts: CloseStreamByProviderIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        provider_account_id,
        clock_id,
    ];
    let tx = crate::test_helpers::build_signed_public_tx(
        dep.vault.program_id,
        crate::Instruction::CloseStreamByOwner {
            vault_id: dep.vault.vault_id,
            stream_id,
        },
        &wrong_accounts,
        &[Nonce(0)],
        &[&dep.vault.owner_private_key],
    );
    let r = dep.vault.state.transition_from_public_transaction(
        &tx,
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(r.is_err(), "expected account-count mismatch, got Ok");
    let _ = provider_private_key;
}

#[test]
fn test_close_stream_by_provider_wrong_account_count_fails() {
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

    let mut dep = state_deposited_with_clock(
        DEFAULT_OWNER_GENESIS_BALANCE,
        DEFAULT_STREAM_TEST_DEPOSIT,
        clock_id,
        DEFAULT_CLOCK_INITIAL_TS,
    );
    let (stream_id, stream_pda) = create_active_stream(
        &mut dep,
        provider_account_id,
        1 as TokensPerSecond,
        100 as Balance,
        clock_id,
    );

    // Five-slot owner layout against the six-slot provider instruction.
    let wrong_accounts: CloseStreamByOwnerIxAccounts = [
        dep.vault.vault_config_account_id,
        dep.vault.vault_holding_account_id,
        stream_pda,
        dep.vault.owner_account_id,
        clock_id,
    ];
    let tx = crate::test_helpers::build_signed_public_tx(
        dep.vault.program_id,
        crate::Instruction::CloseStreamByProvider {
            vault_id: dep.vault.vault_id,
            stream_id,
        },
        &wrong_accounts,
        &[Nonce(0)],
        &[&provider_private_key],
    );
    let r = dep.vault.state.transition_from_public_transaction(
        &tx,
        4 as BlockId,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(r.is_err(), "expected account-count mismatch, got Ok");
}

#[cfg(feature = "pp-program-tests")]
mod pp_program_tests {
    use super::*;

    use crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP;
    use crate::program_tests::pp_common::{
        account_meta, decrypt_account, encapsulate, identity_authorized_update, identity_public,
        owner_vpk, pp_claim_close_setup, pp_owner_setup, private_account_id, recipient_npk,
        recipient_vpk, PpClaimCloseSetup, PpOwnerSetup, EPK_SCALAR, OWNER_NSK, PP3_OWNER_FUND_AMOUNT,
        PP3_SIGNER_EPK_SCALAR, PP3_STREAM_ALLOCATION, PP3_STREAM_RATE, PP3_T0, PP3_T1,
        PP_STREAM_ALLOCATION, PP_STREAM_RATE, PP_T0, PP_T1, PP_WITHDRAW_AMOUNT, RECIPIENT_NSK,
    };
    use crate::test_helpers::{force_clock_account_monotonic, load_guest_program, patch_vault_config};
    use crate::{Instruction, VaultConfig};
    use lee::{
        execute_and_prove,
        privacy_preserving_transaction::{
            circuit::ProgramWithDependencies, message::Message, witness_set::WitnessSet,
            PrivacyPreservingTransaction,
        },
        program::Program,
    };
    use lee_core::{
        account::{Account, AccountWithMetadata, Data},
        Commitment,
    };

    #[test]
    fn test_pp_close_stream_by_provider_private_provider_succeeds() {
        crate::program_tests::pp_common::guard_pp_tests_run_in_risc0_dev_mode_only();

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
        let authority_id = private_account_id(&authority_npk);
        let authority_commitment = Commitment::new(&authority_id, &provider_committed_account);
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

        let (authority_shared_secret, authority_epk) = encapsulate(&recipient_vpk(), &EPK_SCALAR, 0);

        let vault_total_allocated_before = borsh::from_slice::<VaultConfig>(
            &fx.state.get_account_by_id(fx.vault_config_account_id).data,
        )
        .expect("vault config")
        .total_allocated;

        let (output, proof) = execute_and_prove(
            pre_states,
            Program::serialize_instruction(Instruction::CloseStreamByProvider {
                vault_id: fx.vault_id,
                stream_id,
            })
            .expect("close_stream_by_provider instruction serializes"),
            vec![
                identity_public(),
                identity_public(),
                identity_public(),
                identity_public(),
                identity_authorized_update(
                    RECIPIENT_NSK,
                    &recipient_vpk(),
                    authority_shared_secret.clone(),
                    authority_epk,
                    membership_proof,
                ),
                identity_public(),
            ],
            &ProgramWithDependencies::from(guest_program),
        )
        .expect("execute_and_prove close_stream_by_provider");

        let message = Message::try_from_circuit_output(
            vec![
                fx.vault_config_account_id,
                fx.vault_holding_account_id,
                stream_pda,
                fx.owner_account_id,
                clock_id,
            ],
            vec![],
            output,
        )
        .expect("try_from_circuit_output for close_stream_by_provider");

        let witness_set = WitnessSet::for_message(&message, proof, &[]);
        let tx = PrivacyPreservingTransaction::new(message, witness_set);

        fx.state
            .transition_from_privacy_preserving_transaction(
                &tx,
                5 as BlockId,
                TEST_PUBLIC_TX_TIMESTAMP,
            )
            .expect("close_stream_by_provider PP transition");

        let stream_after =
            borsh::from_slice::<StreamConfig>(&fx.state.get_account_by_id(stream_pda).data)
                .expect("stream");
        assert_eq!(stream_after.state, StreamState::Closed);
        let accrued_at_t1 = PP_STREAM_RATE as Balance * (PP_T1 - PP_T0) as Balance;
        assert_eq!(stream_after.allocation, accrued_at_t1);
        assert_eq!(stream_after.accrued, accrued_at_t1);

        let unaccrued = PP_STREAM_ALLOCATION - accrued_at_t1;
        let vault_after = borsh::from_slice::<VaultConfig>(
            &fx.state.get_account_by_id(fx.vault_config_account_id).data,
        )
        .expect("vault");
        assert_eq!(
            vault_after.total_allocated,
            vault_total_allocated_before - unaccrued
        );

        assert_eq!(tx.message().new_commitments.len(), 1);
        assert_eq!(tx.message().encrypted_private_post_states.len(), 1);
        let new_commitment = &tx.message().new_commitments[0];
        let decrypted = decrypt_account(
            &tx.message().encrypted_private_post_states[0].ciphertext,
            &authority_shared_secret,
            new_commitment,
            0,
        );
        assert_eq!(decrypted.balance, PP_WITHDRAW_AMOUNT);
    }

    #[test]
    fn test_pp_close_stream_by_owner_private_owner_succeeds() {
        crate::program_tests::pp_common::guard_pp_tests_run_in_risc0_dev_mode_only();

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
        let provider_id = private_account_id(&recipient_npk());

        let stream_config = StreamConfig::new(
            stream_id,
            provider_id,
            PP3_STREAM_RATE,
            PP3_STREAM_ALLOCATION,
            PP3_T0,
            None,
        );
        let stream_account = Account {
            program_owner: fx.program_id,
            balance: 0,
            data: Data::try_from(borsh::to_vec(&stream_config).unwrap())
                .expect("stream config fits"),
            ..Account::default()
        };
        fx.state.force_insert_account(stream_pda, stream_account);

        patch_vault_config(&mut fx.state, vault_config_b_id, |cfg| {
            cfg.next_stream_id = 1;
            cfg.total_allocated = PP3_STREAM_ALLOCATION;
        });

        force_clock_account_monotonic(&mut fx.state, clock_id, 5, PP3_T1);

        let owner_id = private_account_id(&owner_npk);
        let owner_commitment_obj = Commitment::new(&owner_id, &owner_committed_account);
        let membership_proof = fx
            .state
            .get_proof_for_commitment(&owner_commitment_obj)
            .expect("owner commitment in state after PP withdraw");

        let (owner_shared_secret, owner_epk) =
            encapsulate(&owner_vpk(), &PP3_SIGNER_EPK_SCALAR, 0);

        let vault_total_allocated_before = borsh::from_slice::<VaultConfig>(
            &fx.state.get_account_by_id(vault_config_b_id).data,
        )
        .expect("vault config")
        .total_allocated;

        let pre_states = vec![
            account_meta(&fx.state, vault_config_b_id, false),
            account_meta(&fx.state, vault_holding_b_id, false),
            account_meta(&fx.state, stream_pda, false),
            AccountWithMetadata {
                account: owner_committed_account.clone(),
                is_authorized: true,
                account_id: owner_id,
            },
            account_meta(&fx.state, clock_id, false),
        ];

        let (output, proof) = execute_and_prove(
            pre_states,
            Program::serialize_instruction(Instruction::CloseStreamByOwner {
                vault_id: vault_b_id,
                stream_id,
            })
            .expect("close_stream_by_owner instruction serializes"),
            vec![
                identity_public(),
                identity_public(),
                identity_public(),
                identity_authorized_update(
                    OWNER_NSK,
                    &owner_vpk(),
                    owner_shared_secret.clone(),
                    owner_epk,
                    membership_proof,
                ),
                identity_public(),
            ],
            &ProgramWithDependencies::from(load_guest_program()),
        )
        .expect("execute_and_prove: PP close_stream_by_owner");

        let message = Message::try_from_circuit_output(
            vec![vault_config_b_id, vault_holding_b_id, stream_pda, clock_id],
            vec![],
            output,
        )
        .expect("try_from_circuit_output: close_stream_by_owner");

        let witness_set = WitnessSet::for_message(&message, proof, &[]);
        let tx = PrivacyPreservingTransaction::new(message, witness_set);

        fx.state
            .transition_from_privacy_preserving_transaction(
                &tx,
                5 as BlockId,
                TEST_PUBLIC_TX_TIMESTAMP,
            )
            .expect("close_stream_by_owner PP transition");

        let stream =
            borsh::from_slice::<StreamConfig>(&fx.state.get_account_by_id(stream_pda).data)
                .expect("stream config after close");
        assert_eq!(stream.state, StreamState::Closed);
        let accrued_at_t1 = PP3_STREAM_RATE as Balance * (PP3_T1 - PP3_T0) as Balance;
        assert_eq!(stream.accrued, accrued_at_t1);
        assert_eq!(stream.allocation, accrued_at_t1);

        let unaccrued = PP3_STREAM_ALLOCATION - accrued_at_t1;
        let vault_after = borsh::from_slice::<VaultConfig>(
            &fx.state.get_account_by_id(vault_config_b_id).data,
        )
        .expect("vault");
        assert_eq!(
            vault_after.total_allocated,
            vault_total_allocated_before - unaccrued
        );

        assert_eq!(tx.message().new_commitments.len(), 1);
        let decrypted = decrypt_account(
            &tx.message().encrypted_private_post_states[0].ciphertext,
            &owner_shared_secret,
            &tx.message().new_commitments[0],
            0,
        );
        assert_eq!(decrypted.balance, PP3_OWNER_FUND_AMOUNT);
    }

    #[test]
    fn test_pp_close_stream_by_provider_private_owner_succeeds() {
        crate::program_tests::pp_common::guard_pp_tests_run_in_risc0_dev_mode_only();

        // PF vault: private owner in non-signing six-slot; public stream provider signs close.
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
        let (provider_private_key, provider_account_id) = create_keypair(SEED_PROVIDER);

        let stream_config = StreamConfig::new(
            stream_id,
            provider_account_id,
            PP3_STREAM_RATE,
            PP3_STREAM_ALLOCATION,
            PP3_T0,
            None,
        );
        let stream_account = Account {
            program_owner: fx.program_id,
            balance: 0,
            data: Data::try_from(borsh::to_vec(&stream_config).unwrap())
                .expect("stream config fits"),
            ..Account::default()
        };
        fx.state.force_insert_account(stream_pda, stream_account);

        patch_vault_config(&mut fx.state, vault_config_b_id, |cfg| {
            cfg.next_stream_id = 1;
            cfg.total_allocated = PP3_STREAM_ALLOCATION;
        });

        // Ensure public provider account exists for signing.
        fx.state.force_insert_account(
            provider_account_id,
            Account {
                balance: 0,
                ..Account::default()
            },
        );

        force_clock_account_monotonic(&mut fx.state, clock_id, 5, PP3_T1);

        let owner_id = private_account_id(&owner_npk);
        let owner_commitment_obj = Commitment::new(&owner_id, &owner_committed_account);
        let membership_proof = fx
            .state
            .get_proof_for_commitment(&owner_commitment_obj)
            .expect("owner commitment in state after PP withdraw");

        let (owner_shared_secret, owner_epk) =
            encapsulate(&owner_vpk(), &PP3_SIGNER_EPK_SCALAR, 0);

        let pre_states = vec![
            account_meta(&fx.state, vault_config_b_id, false),
            account_meta(&fx.state, vault_holding_b_id, false),
            account_meta(&fx.state, stream_pda, false),
            AccountWithMetadata {
                account: owner_committed_account.clone(),
                is_authorized: true,
                account_id: owner_id,
            },
            account_meta(&fx.state, provider_account_id, true),
            account_meta(&fx.state, clock_id, false),
        ];

        let provider_before = fx.state.get_account_by_id(provider_account_id);

        let (output, proof) = execute_and_prove(
            pre_states,
            Program::serialize_instruction(Instruction::CloseStreamByProvider {
                vault_id: vault_b_id,
                stream_id,
            })
            .expect("close_stream_by_provider instruction serializes"),
            vec![
                identity_public(),
                identity_public(),
                identity_public(),
                identity_authorized_update(
                    OWNER_NSK,
                    &owner_vpk(),
                    owner_shared_secret.clone(),
                    owner_epk,
                    membership_proof,
                ),
                identity_public(),
                identity_public(),
            ],
            &ProgramWithDependencies::from(load_guest_program()),
        )
        .expect("execute_and_prove: PP close_stream_by_provider with private owner");

        let message = Message::try_from_circuit_output(
            vec![
                vault_config_b_id,
                vault_holding_b_id,
                stream_pda,
                provider_account_id,
                clock_id,
            ],
            vec![provider_before.nonce],
            output,
        )
        .expect("try_from_circuit_output: close_stream_by_provider PF");

        let witness_set = WitnessSet::for_message(&message, proof, &[&provider_private_key]);
        let tx = PrivacyPreservingTransaction::new(message, witness_set);

        fx.state
            .transition_from_privacy_preserving_transaction(
                &tx,
                5 as BlockId,
                TEST_PUBLIC_TX_TIMESTAMP,
            )
            .expect("close_stream_by_provider PF PP transition");

        let stream =
            borsh::from_slice::<StreamConfig>(&fx.state.get_account_by_id(stream_pda).data)
                .expect("stream config after close");
        assert_eq!(stream.state, StreamState::Closed);
        assert_eq!(tx.message().new_commitments.len(), 1);
        let decrypted = decrypt_account(
            &tx.message().encrypted_private_post_states[0].ciphertext,
            &owner_shared_secret,
            &tx.message().new_commitments[0],
            0,
        );
        assert_eq!(decrypted.balance, PP3_OWNER_FUND_AMOUNT);
    }
}
