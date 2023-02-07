pub mod metabonding_setup;

use std::iter::FromIterator;

use metabonding::claim_progress::{ClaimFlag, ClaimProgressModule, ShiftingClaimProgress};
use metabonding_setup::*;
use multiversx_sc::types::ManagedVec;
use multiversx_sc_scenario::{managed_address, managed_buffer, rust_biguint};

#[test]
fn claim_partial_ok_test() {
    let mut mb_setup = MetabondingSetup::new(metabonding::contract_obj);
    mb_setup.add_default_projects();
    mb_setup.deposit_rewards_default_projects();
    mb_setup.add_default_checkpoints();
    mb_setup.call_unpause().assert_ok();

    let first_user_addr = mb_setup.first_user_addr.clone();
    let sig_first_user_week_1 = hex_literal::hex!("d47c0d67b2d25de8b4a3f43d91a2b5ccb522afac47321ae80bf89c90a4445b26adefa693ab685fa20891f736d74eb2dedc11c4b1a8d6e642fa28df270d6ebe08");
    let sig_first_user_week_2 = hex_literal::hex!("b4aadf08eea4cc7c636922511943edbab2ff6ef2558528e0e7b03c7448367989fe860ac091be4d942304f04c86b1eaa0501f36e02819a3c628b4c53f3d3ac801");

    // claim only second token for week 2
    mb_setup
        .call_claim_partial_rewards(
            &first_user_addr,
            2,
            25_000,
            0,
            &sig_first_user_week_2,
            &[SECOND_PROJ_ID],
        )
        .assert_ok();

    mb_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, FIRST_PROJ_TOKEN, &rust_biguint!(0));
    mb_setup.b_mock.check_esdt_balance(
        &first_user_addr,
        SECOND_PROJ_TOKEN,
        &rust_biguint!(50_000_000),
    );

    mb_setup
        .b_mock
        .execute_query(&mb_setup.mb_wrapper, |sc| {
            let not_claimed = ClaimFlag::NotClaimed;

            // check shifting progress
            let shifting_progress = sc.claim_progress(&managed_address!(&first_user_addr)).get();
            let expected_shifting_progress = ShiftingClaimProgress::new(
                [
                    not_claimed.clone(),
                    ClaimFlag::Claimed {
                        unclaimed_projects: ManagedVec::from_single_item(managed_buffer!(
                            FIRST_PROJ_ID
                        )),
                    },
                    not_claimed.clone(),
                    not_claimed.clone(),
                    not_claimed.clone(),
                ]
                .into(),
                2,
            );
            assert_eq!(shifting_progress, expected_shifting_progress);
        })
        .assert_ok();

    // try claim second project for week 2 again - no rewards given, no state changes
    mb_setup
        .call_claim_partial_rewards(
            &first_user_addr,
            2,
            25_000,
            0,
            &sig_first_user_week_2,
            &[SECOND_PROJ_ID],
        )
        .assert_ok();

    mb_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, FIRST_PROJ_TOKEN, &rust_biguint!(0));
    mb_setup.b_mock.check_esdt_balance(
        &first_user_addr,
        SECOND_PROJ_TOKEN,
        &rust_biguint!(50_000_000),
    );

    mb_setup
        .b_mock
        .execute_query(&mb_setup.mb_wrapper, |sc| {
            let not_claimed = ClaimFlag::NotClaimed;

            // check shifting progress
            let shifting_progress = sc.claim_progress(&managed_address!(&first_user_addr)).get();
            let expected_shifting_progress = ShiftingClaimProgress::new(
                [
                    not_claimed.clone(),
                    ClaimFlag::Claimed {
                        unclaimed_projects: ManagedVec::from_single_item(managed_buffer!(
                            FIRST_PROJ_ID
                        )),
                    },
                    not_claimed.clone(),
                    not_claimed.clone(),
                    not_claimed.clone(),
                ]
                .into(),
                2,
            );
            assert_eq!(shifting_progress, expected_shifting_progress);
        })
        .assert_ok();

    // claim first proj for week 2
    mb_setup
        .call_claim_partial_rewards(
            &first_user_addr,
            2,
            25_000,
            0,
            &sig_first_user_week_2,
            &[FIRST_PROJ_ID],
        )
        .assert_ok();

    mb_setup.b_mock.check_esdt_balance(
        &first_user_addr,
        FIRST_PROJ_TOKEN,
        &rust_biguint!(41_666_666),
    );
    mb_setup.b_mock.check_esdt_balance(
        &first_user_addr,
        SECOND_PROJ_TOKEN,
        &rust_biguint!(50_000_000),
    );

    mb_setup
        .b_mock
        .execute_query(&mb_setup.mb_wrapper, |sc| {
            let not_claimed = ClaimFlag::NotClaimed;

            // check shifting progress
            let shifting_progress = sc.claim_progress(&managed_address!(&first_user_addr)).get();
            let expected_shifting_progress = ShiftingClaimProgress::new(
                [
                    not_claimed.clone(),
                    ClaimFlag::Claimed {
                        unclaimed_projects: ManagedVec::new(),
                    },
                    not_claimed.clone(),
                    not_claimed.clone(),
                    not_claimed.clone(),
                ]
                .into(),
                2,
            );
            assert_eq!(shifting_progress, expected_shifting_progress);
        })
        .assert_ok();

    // try claim week 2 again - no tokens left
    mb_setup
        .call_claim_partial_rewards(
            &first_user_addr,
            2,
            25_000,
            0,
            &sig_first_user_week_2,
            &[FIRST_PROJ_ID, SECOND_PROJ_ID],
        )
        .assert_user_error("Already claimed rewards for this week");

    // claim for first week with invalid project IDs
    mb_setup
        .call_claim_partial_rewards(
            &first_user_addr,
            1,
            25_000,
            0,
            &sig_first_user_week_1,
            &[b"SCAM-123456", b"BAD-123456"],
        )
        .assert_ok();

    mb_setup
        .b_mock
        .execute_query(&mb_setup.mb_wrapper, |sc| {
            let not_claimed = ClaimFlag::NotClaimed;

            // check shifting progress
            let shifting_progress = sc.claim_progress(&managed_address!(&first_user_addr)).get();
            let expected_shifting_progress = ShiftingClaimProgress::new(
                [
                    ClaimFlag::Claimed {
                        unclaimed_projects: ManagedVec::from_iter(vec![
                            managed_buffer!(FIRST_PROJ_ID),
                            managed_buffer!(SECOND_PROJ_ID),
                        ]),
                    },
                    ClaimFlag::Claimed {
                        unclaimed_projects: ManagedVec::new(),
                    },
                    not_claimed.clone(),
                    not_claimed.clone(),
                    not_claimed.clone(),
                ]
                .into(),
                2,
            );
            assert_eq!(shifting_progress, expected_shifting_progress);
        })
        .assert_ok();

    // claim all for week 1
    mb_setup
        .call_claim_partial_rewards(
            &first_user_addr,
            1,
            25_000,
            0,
            &sig_first_user_week_1,
            &[FIRST_PROJ_ID, SECOND_PROJ_ID],
        )
        .assert_ok();

    // same result as if the user claimed full intially
    mb_setup.b_mock.check_esdt_balance(
        &first_user_addr,
        FIRST_PROJ_TOKEN,
        &rust_biguint!(83_333_333 + 41_666_666),
    );
    mb_setup.b_mock.check_esdt_balance(
        &first_user_addr,
        SECOND_PROJ_TOKEN,
        &rust_biguint!(50_000_000),
    );

    mb_setup
        .b_mock
        .execute_query(&mb_setup.mb_wrapper, |sc| {
            let not_claimed = ClaimFlag::NotClaimed;

            // check shifting progress
            let shifting_progress = sc.claim_progress(&managed_address!(&first_user_addr)).get();
            let expected_shifting_progress = ShiftingClaimProgress::new(
                [
                    ClaimFlag::Claimed {
                        unclaimed_projects: ManagedVec::new(),
                    },
                    ClaimFlag::Claimed {
                        unclaimed_projects: ManagedVec::new(),
                    },
                    not_claimed.clone(),
                    not_claimed.clone(),
                    not_claimed.clone(),
                ]
                .into(),
                2,
            );
            assert_eq!(shifting_progress, expected_shifting_progress);
        })
        .assert_ok();
}
