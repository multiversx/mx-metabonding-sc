#![allow(deprecated)]
pub mod metabonding_setup;

use multiversx_sc::types::{ManagedVec, MultiValueEncoded};
use multiversx_sc_scenario::{managed_address, rust_biguint};

use metabonding::{
    claim_progress::{ClaimFlag, ClaimProgressModule, ShiftingClaimProgress},
    legacy_storage_cleanup::LegacyStorageCleanupModule,
};
use metabonding_setup::*;

#[test]
fn claim_progress_migration_test() {
    let mut mb_setup = MetabondingSetup::new(metabonding::contract_obj);
    let first_user = mb_setup.first_user_addr.clone();

    mb_setup
        .b_mock
        .execute_tx(
            &mb_setup.owner_addr,
            &mb_setup.mb_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.legacy_rewards_claimed_flag(&managed_address!(&first_user), 1)
                    .set(true);
                sc.legacy_rewards_claimed_flag(&managed_address!(&first_user), 2)
                    .set(true);
                sc.legacy_rewards_claimed_flag(&managed_address!(&first_user), 5)
                    .set(true);

                let not_claimed = ClaimFlag::NotClaimed;
                let claimed = ClaimFlag::Claimed {
                    unclaimed_projects: ManagedVec::new(),
                };

                // check shifting progress
                let shifting_progress = sc.get_claim_progress(&managed_address!(&first_user), 5);
                let expected_shifting_progress = ShiftingClaimProgress::new(
                    [
                        claimed.clone(),
                        claimed.clone(),
                        not_claimed.clone(),
                        not_claimed.clone(),
                        claimed.clone(),
                    ]
                    .into(),
                    5,
                );
                assert_eq!(shifting_progress, expected_shifting_progress);

                // check shifted by 1
                let shifting_progress_after_1 =
                    sc.get_claim_progress(&managed_address!(&first_user), 6);
                let expected_shifting_progress_after_1 = ShiftingClaimProgress::new(
                    [
                        claimed.clone(),
                        not_claimed.clone(),
                        not_claimed.clone(),
                        claimed,
                        not_claimed,
                    ]
                    .into(),
                    6,
                );
                assert_eq!(
                    shifting_progress_after_1,
                    expected_shifting_progress_after_1
                );

                // check shifted when getting from storage
                sc.claim_progress(&managed_address!(&first_user))
                    .set(&shifting_progress);
                let shifted_from_storage = sc.get_claim_progress(&managed_address!(&first_user), 6);
                assert_eq!(shifted_from_storage, expected_shifting_progress_after_1);
            },
        )
        .assert_ok();
}

#[test]
fn claim_progress_cleanup_test() {
    let mut mb_setup = MetabondingSetup::new(metabonding::contract_obj);
    let first_user = mb_setup.first_user_addr.clone();

    // set current week = 5
    mb_setup.b_mock.set_block_epoch(40);

    mb_setup
        .b_mock
        .execute_tx(
            &mb_setup.owner_addr,
            &mb_setup.mb_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.legacy_rewards_claimed_flag(&managed_address!(&first_user), 1)
                    .set(true);
                sc.legacy_rewards_claimed_flag(&managed_address!(&first_user), 2)
                    .set(true);
                sc.legacy_rewards_claimed_flag(&managed_address!(&first_user), 5)
                    .set(true);

                let mut args = MultiValueEncoded::new();
                args.push(managed_address!(&first_user));
                sc.clear_old_storage_flags(args);

                let not_claimed = ClaimFlag::NotClaimed;
                let claimed = ClaimFlag::Claimed {
                    unclaimed_projects: ManagedVec::new(),
                };

                // check shifting progress
                let shifting_progress = sc.claim_progress(&managed_address!(&first_user)).get();
                let expected_shifting_progress = ShiftingClaimProgress::new(
                    [
                        claimed.clone(),
                        claimed.clone(),
                        not_claimed.clone(),
                        not_claimed,
                        claimed,
                    ]
                    .into(),
                    5,
                );
                assert_eq!(shifting_progress, expected_shifting_progress);

                assert!(!sc
                    .legacy_rewards_claimed_flag(&managed_address!(&first_user), 1)
                    .get(),);
                assert!(!sc
                    .legacy_rewards_claimed_flag(&managed_address!(&first_user), 2)
                    .get(),);
                assert!(!sc
                    .legacy_rewards_claimed_flag(&managed_address!(&first_user), 5)
                    .get(),);
            },
        )
        .assert_ok();
}
