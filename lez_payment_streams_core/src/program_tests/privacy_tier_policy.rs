//! Harness-side checks for [`crate::VaultPrivacyTier::PseudonymousFunder`] public transitions.

use nssa::error::NssaError;
use nssa_core::account::Nonce;
use nssa_core::BlockId;

use crate::harness_seeds::SEED_PROVIDER;
use crate::Instruction;
use crate::{
    test_helpers::{
        assert_public_payment_streams_instruction_allowed, create_keypair, derive_stream_pda,
        force_clock_account_monotonic, state_with_initialized_vault_pseudonymous_funder_preseeded,
        state_with_initialized_vault_with_privacy_tier, transfer_native_balance_for_tests,
        transition_public_payment_streams_tx_respecting_privacy_tier,
    },
    StreamId, VaultPrivacyTier, CLOCK_01_PROGRAM_ACCOUNT_ID,
};

use super::common::{signed_create_stream, DEFAULT_CLOCK_INITIAL_TS, TEST_PUBLIC_TX_TIMESTAMP};

#[test]
fn harness_public_touch_pseudonymous_funder_vault_fails() {
    let fx = state_with_initialized_vault_with_privacy_tier(
        1_000 as nssa_core::account::Balance,
        VaultPrivacyTier::PseudonymousFunder,
    );
    assert_eq!(
        assert_public_payment_streams_instruction_allowed(&fx.state, fx.vault_config_account_id,),
        Err("public instruction disallowed for PseudonymousFunder vault")
    );
}

#[test]
fn harness_public_touch_public_tier_vault_succeeds() {
    let fx = state_with_initialized_vault_with_privacy_tier(
        1_000 as nssa_core::account::Balance,
        VaultPrivacyTier::Public,
    );
    assert!(assert_public_payment_streams_instruction_allowed(
        &fx.state,
        fx.vault_config_account_id,
    )
    .is_ok());
}

#[test]
fn wrapped_public_deposit_before_transition_pseudonymous_funder_fails() {
    use nssa::program::Program;

    use crate::test_helpers::{
        build_signed_public_tx, transition_public_payment_streams_tx_respecting_privacy_tier,
    };

    use super::common::TEST_PUBLIC_TX_TIMESTAMP;

    let mut fx = state_with_initialized_vault_with_privacy_tier(
        1_000 as nssa_core::account::Balance,
        VaultPrivacyTier::PseudonymousFunder,
    );
    let tx = build_signed_public_tx(
        fx.program_id,
        Instruction::Deposit {
            vault_id: fx.vault_id,
            amount: 50,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &[
            fx.vault_config_account_id,
            fx.vault_holding_account_id,
            fx.owner_account_id,
        ],
        &[Nonce(1)],
        &[&fx.owner_private_key],
    );

    assert!(matches!(
        transition_public_payment_streams_tx_respecting_privacy_tier(
            &mut fx.state,
            fx.vault_config_account_id,
            &tx,
            2 as BlockId,
            TEST_PUBLIC_TX_TIMESTAMP,
        ),
        Err(NssaError::InvalidInput(_))
    ));
}

#[test]
fn public_create_stream_pseudonymous_funder_vault_fails() {
    let (_, provider_account_id) = create_keypair(SEED_PROVIDER);
    let mut fx = state_with_initialized_vault_pseudonymous_funder_preseeded(
        2_000 as nssa_core::account::Balance,
        &[(provider_account_id, 0 as nssa_core::account::Balance)],
    );
    transfer_native_balance_for_tests(
        &mut fx.state,
        fx.owner_account_id,
        fx.vault_holding_account_id,
        500 as nssa_core::account::Balance,
    );
    force_clock_account_monotonic(
        &mut fx.state,
        CLOCK_01_PROGRAM_ACCOUNT_ID,
        0,
        DEFAULT_CLOCK_INITIAL_TS,
    );

    let stream_id = StreamId::MIN;
    let stream_pda = derive_stream_pda(fx.program_id, fx.vault_config_account_id, stream_id);
    let accounts = [
        fx.vault_config_account_id,
        fx.vault_holding_account_id,
        stream_pda,
        fx.owner_account_id,
        CLOCK_01_PROGRAM_ACCOUNT_ID,
    ];
    let tx = signed_create_stream(
        fx.program_id,
        fx.vault_id,
        stream_id,
        provider_account_id,
        10,
        200,
        &accounts,
        Nonce(2),
        &fx.owner_private_key,
    );

    assert!(matches!(
        transition_public_payment_streams_tx_respecting_privacy_tier(
            &mut fx.state,
            fx.vault_config_account_id,
            &tx,
            3 as BlockId,
            TEST_PUBLIC_TX_TIMESTAMP,
        ),
        Err(NssaError::InvalidInput(_))
    ));
}
