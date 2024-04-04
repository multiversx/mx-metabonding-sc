#![allow(deprecated)]

pub mod growth_program_setup;
use growth_program::{
    rewards::{
        claim::ClaimRewardsModule,
        claim_types::{ClaimType, LockOption},
        common_rewards::{CommonRewardsModule, RewardsInfo},
        deposit::DepositRewardsModule,
        energy::EnergyModule,
        week_timekeeping::MONDAY_19_02_2024_GMT_TIMESTAMP,
        withdraw::WithdrawRewardsModule,
    },
    DEFAULT_MIN_REWARDS_PERIOD, WEEK_IN_SECONDS,
};
use growth_program_setup::*;
use multiversx_sc::{
    codec::multi_types::OptionalValue,
    types::{Address, ManagedByteArray},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, managed_token_id_wrapped,
    rust_biguint, DebugApi,
};
use num_traits::FromPrimitive;
use simple_lock::locked_token::LockedTokenAttributes;

#[test]
fn setup_test() {
    let _ = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );
}

#[test]
fn add_projects_test() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();
}

#[test]
fn deposit_too_few_rewards_test() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();

    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.first_project_owner,
            &setup.gp_wrapper,
            FIRST_PROJ_TOKEN,
            0,
            &rust_biguint!(10),
            |sc| {
                let signer_addr = managed_address!(&Address::from(&SIGNER_ADDRESS));

                sc.deposit_initial_rewards(1, 2, 28, signer_addr);
            },
        )
        .assert_user_error("Too few rewards");
}

#[test]
fn deposit_wrong_week_amount_test() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();

    let amount = StaticMethods::get_first_token_full_amount();

    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.first_project_owner,
            &setup.gp_wrapper,
            FIRST_PROJ_TOKEN,
            0,
            &amount,
            |sc| {
                let signer_addr = managed_address!(&Address::from(&SIGNER_ADDRESS));

                sc.deposit_initial_rewards(1, 2, 5, signer_addr);
            },
        )
        .assert_user_error("Too few reward weeks");
}

#[test]
fn deposit_rewards_ok_test() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();
    setup.set_first_week_apr(5000);
    setup.deposit_rewards();

    setup
        .b_mock
        .execute_query(&setup.gp_wrapper, |sc| {
            let rewards_per_week_amount = sc.rewards_total_amount(1, 2).get();
            assert_eq!(
                rewards_per_week_amount,
                StaticMethods::get_first_token_full_amount_managed::<DebugApi>()
                    / DEFAULT_MIN_REWARDS_PERIOD as u32
            );
        })
        .assert_ok();
}

#[test]
fn deposit_additional_rewards_ok_test() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();
    setup.set_first_week_apr(5000);
    setup.deposit_rewards();

    let required_balance = StaticMethods::get_first_token_full_amount();
    setup.b_mock.set_esdt_balance(
        &setup.first_project_owner,
        FIRST_PROJ_TOKEN,
        &required_balance,
    );

    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.first_project_owner,
            &setup.gp_wrapper,
            FIRST_PROJ_TOKEN,
            0,
            &required_balance,
            |sc| {
                sc.deposit_additional_rewards(1, 4, 4 + DEFAULT_MIN_REWARDS_PERIOD);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_query(&setup.gp_wrapper, |sc| {
            let rewards_week_two = sc.rewards_total_amount(1, 2).get();
            assert_eq!(
                rewards_week_two,
                StaticMethods::get_first_token_full_amount_managed::<DebugApi>()
                    / DEFAULT_MIN_REWARDS_PERIOD as u32
            );

            let rewards_week_four = sc.rewards_total_amount(1, 4).get();
            assert_eq!(
                rewards_week_four,
                StaticMethods::get_first_token_full_amount_managed::<DebugApi>() * 2u32
                    / DEFAULT_MIN_REWARDS_PERIOD as u32
            );

            let rewards_week_twenty_nine = sc.rewards_total_amount(1, 29).get();
            assert_eq!(
                rewards_week_twenty_nine,
                StaticMethods::get_first_token_full_amount_managed::<DebugApi>()
                    / DEFAULT_MIN_REWARDS_PERIOD as u32
            );

            let rewards_info = sc.rewards_info(1).get();
            let expected_rewards_info = RewardsInfo {
                reward_token_id: managed_token_id!(FIRST_PROJ_TOKEN),
                undistributed_rewards: managed_biguint!(0),
                start_week: 2,
                last_update_week: 2,
                end_week: 30,
            };
            assert_eq!(rewards_info, expected_rewards_info);
        })
        .assert_ok();
}

#[test]
fn claim_ok_first_week_unlocked_test() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();
    setup.set_first_week_apr(5000);
    setup.deposit_rewards();

    // advance to week 2
    setup.advance_week();

    let sig_first_user_week_2 = hex_literal::hex!("3360e54f357cbb67b1c34771b633d0f7ad9779019a0dcee252d972315c1edb8178012f057c94714e52b3d461ef333cb3020c29e3f98e467a4d3341880891690e");
    setup
        .claim(
            &setup.first_user_addr.clone(),
            1,
            LockOption::None,
            0,
            &sig_first_user_week_2,
        )
        .assert_ok();

    setup
        .b_mock
        .execute_query(&setup.gp_wrapper, |sc| {
            let total_energy = sc.total_energy_for_week(1, 2).get();
            assert_eq!(
                total_energy,
                multiversx_sc::types::BigUint::from(57600000000002995200000000191692u128)
            );

            let interested_energy = sc.interested_energy_rewards_claimers(1, 2).get();
            assert_eq!(interested_energy, managed_biguint!(348000) / 4u32); // 25% for full unlocked
        })
        .assert_ok();

    setup.b_mock.check_esdt_balance(
        &setup.first_user_addr,
        FIRST_PROJ_TOKEN,
        &num_bigint::BigUint::from_u128(435000039150003523500317).unwrap(),
    );

    // first user try claim again
    setup
        .claim(
            &setup.first_user_addr.clone(),
            1,
            LockOption::None,
            0,
            &sig_first_user_week_2,
        )
        .assert_user_error("Already claimed");
}

#[test]
fn claim_ok_first_week_locked_test() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();
    setup.set_first_week_apr(5000);
    setup.deposit_rewards();

    // advance to week 2
    setup.advance_week();

    let sig_first_user_week_2 = hex_literal::hex!("3360e54f357cbb67b1c34771b633d0f7ad9779019a0dcee252d972315c1edb8178012f057c94714e52b3d461ef333cb3020c29e3f98e467a4d3341880891690e");
    setup
        .claim(
            &setup.first_user_addr.clone(),
            1,
            LockOption::OneWeek,
            0,
            &sig_first_user_week_2,
        )
        .assert_ok();

    setup
        .b_mock
        .execute_query(&setup.gp_wrapper, |sc| {
            let total_energy = sc.total_energy_for_week(1, 2).get();
            assert_eq!(
                total_energy,
                multiversx_sc::types::BigUint::from(57600000000002995200000000191692u128)
            );

            let interested_energy = sc.interested_energy_rewards_claimers(1, 2).get();
            assert_eq!(interested_energy, managed_biguint!(348000) / 2u32); // 50% for one week lock
        })
        .assert_ok();

    DebugApi::dummy();
    setup.b_mock.check_nft_balance(
        &setup.first_user_addr,
        LOCKED_TOKEN_ID,
        1,
        &num_bigint::BigUint::from_u128(870000078300007047000634).unwrap(),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(FIRST_PROJ_TOKEN),
            original_token_nonce: 0,
            unlock_epoch: 19,
        }),
    );
}

#[test]
fn claim_too_many_rewards_test() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();
    setup.set_first_week_apr(5000);
    setup.deposit_rewards();

    // advance to week 2
    setup.advance_week();

    let sig_first_user_week_2 = hex_literal::hex!("3360e54f357cbb67b1c34771b633d0f7ad9779019a0dcee252d972315c1edb8178012f057c94714e52b3d461ef333cb3020c29e3f98e467a4d3341880891690e");
    setup
        .b_mock
        .execute_tx(
            &setup.first_user_addr,
            &setup.gp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let multi_value_arg = (
                    managed_buffer!(b"lala"),
                    ManagedByteArray::new_from_bytes(&sig_first_user_week_2),
                )
                    .into();
                let _ = sc.claim_rewards(
                    1,
                    multiversx_sc::types::BigUint::from(870000078300007047000634u128) + 1u32,
                    ClaimType::Rewards(LockOption::None),
                    OptionalValue::Some(multi_value_arg),
                );
            },
        )
        .assert_user_error("Too few rewards");
}

#[test]
fn claim_attempts_test() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();
    setup.set_first_week_apr(5000);
    setup.deposit_rewards();

    // advance to week 2
    setup.advance_week();

    let sig_first_user_week_2 = hex_literal::hex!("3360e54f357cbb67b1c34771b633d0f7ad9779019a0dcee252d972315c1edb8178012f057c94714e52b3d461ef333cb3020c29e3f98e467a4d3341880891690e");
    setup
        .claim(
            &setup.first_user_addr.clone(),
            1,
            LockOption::OneWeek,
            0,
            &sig_first_user_week_2,
        )
        .assert_ok();

    setup
        .b_mock
        .execute_query(&setup.gp_wrapper, |sc| {
            let total_energy = sc.total_energy_for_week(1, 2).get();
            assert_eq!(
                total_energy,
                multiversx_sc::types::BigUint::from(57600000000002995200000000191692u128)
            );

            let interested_energy = sc.interested_energy_rewards_claimers(1, 2).get();
            assert_eq!(interested_energy, managed_biguint!(348000) / 2u32); // 50% for one week lock
        })
        .assert_ok();

    DebugApi::dummy();
    setup.b_mock.check_nft_balance(
        &setup.first_user_addr,
        LOCKED_TOKEN_ID,
        1,
        &num_bigint::BigUint::from_u128(870000078300007047000634).unwrap(),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(FIRST_PROJ_TOKEN),
            original_token_nonce: 0,
            unlock_epoch: 19,
        }),
    );

    // try claim rewards project 2, same signature
    setup
        .claim(
            &setup.first_user_addr.clone(),
            2,
            LockOption::OneWeek,
            0,
            &sig_first_user_week_2,
        )
        .assert_error(10, "invalid signature");

    // try claim exemption after claim
    setup
        .b_mock
        .execute_tx(
            &setup.first_user_addr,
            &setup.gp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let _ = sc.claim_rewards(
                    1,
                    managed_biguint!(0),
                    ClaimType::Exemption,
                    OptionalValue::None,
                );
            },
        )
        .assert_user_error("Already claimed");

    // advance to week 3
    setup.advance_week();

    // first user try claim with the same signature
    setup
        .claim(
            &setup.first_user_addr.clone(),
            1,
            LockOption::OneWeek,
            0,
            &sig_first_user_week_2,
        )
        .assert_error(10, "invalid signature");
}

#[test]
fn exempted_user_claim_next_week_test() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();
    setup.set_first_week_apr(5000);
    setup.deposit_rewards();

    // advance to week 2
    setup.advance_week();

    // first user try claim exemption while rewards remain
    let sig_first_user_week_2 = hex_literal::hex!("3360e54f357cbb67b1c34771b633d0f7ad9779019a0dcee252d972315c1edb8178012f057c94714e52b3d461ef333cb3020c29e3f98e467a4d3341880891690e");
    setup
        .b_mock
        .execute_tx(
            &setup.first_user_addr,
            &setup.gp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let multi_value_arg = (
                    managed_buffer!(b"lala"),
                    ManagedByteArray::new_from_bytes(&sig_first_user_week_2),
                )
                    .into();

                let _ = sc.claim_rewards(
                    1,
                    managed_biguint!(0),
                    ClaimType::Exemption,
                    OptionalValue::Some(multi_value_arg),
                );
            },
        )
        .assert_user_error("Can claim full rewards");

    // first user claim exemption
    let sig_first_user_week_2 = hex_literal::hex!("3360e54f357cbb67b1c34771b633d0f7ad9779019a0dcee252d972315c1edb8178012f057c94714e52b3d461ef333cb3020c29e3f98e467a4d3341880891690e");
    setup
        .b_mock
        .execute_tx(
            &setup.first_user_addr,
            &setup.gp_wrapper,
            &rust_biguint!(0),
            |sc| {
                // set remaining rewards to 0 so user can claim exemption
                sc.rewards_remaining_amount(1, 2).clear();

                let multi_value_arg = (
                    managed_buffer!(b"lala"),
                    ManagedByteArray::new_from_bytes(&sig_first_user_week_2),
                )
                    .into();

                let _ = sc.claim_rewards(
                    1,
                    managed_biguint!(0),
                    ClaimType::Exemption,
                    OptionalValue::Some(multi_value_arg),
                );
            },
        )
        .assert_ok();

    // advance to week 3
    setup.advance_week();

    // user claim next week without signature
    setup
        .b_mock
        .execute_tx(
            &setup.first_user_addr,
            &setup.gp_wrapper,
            &rust_biguint!(0),
            |sc| {
                let _ = sc.claim_rewards(
                    1,
                    managed_biguint!(0),
                    ClaimType::Rewards(LockOption::OneWeek),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    DebugApi::dummy();
    setup.b_mock.check_nft_balance(
        &setup.first_user_addr,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(1705000), // way less rewards due to calculated rdpe decreasing
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(FIRST_PROJ_TOKEN),
            original_token_nonce: 0,
            unlock_epoch: 26,
        }),
    );
}

#[ignore = "Comment suggested part of code for it to work"]
#[test]
fn start_program_again_after_end() {
    let mut setup = GrowthProgramSetup::new(
        growth_program::contract_obj,
        pair_mock::contract_obj,
        router_mock::contract_obj,
        simple_lock::contract_obj,
        energy_factory::contract_obj,
    );

    setup.add_projects();
    setup.set_first_week_apr(5000);
    setup.deposit_rewards();

    setup
        .b_mock
        .execute_query(&setup.gp_wrapper, |sc| {
            let rewards_per_week_amount = sc.rewards_total_amount(1, 2).get();
            assert_eq!(
                rewards_per_week_amount,
                StaticMethods::get_first_token_full_amount_managed::<DebugApi>()
                    / DEFAULT_MIN_REWARDS_PERIOD as u32
            );
        })
        .assert_ok();

    setup
        .b_mock
        .set_block_timestamp(MONDAY_19_02_2024_GMT_TIMESTAMP + WEEK_IN_SECONDS * 27);

    setup
        .b_mock
        .execute_tx(
            &setup.first_project_owner,
            &setup.gp_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.finish_program(1);
            },
        )
        .assert_ok();

    setup.b_mock.set_esdt_balance(
        &setup.first_project_owner,
        SECOND_PROJ_TOKEN,
        &StaticMethods::get_second_token_full_amount(),
    );

    // start program again, different token
    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.first_project_owner,
            &setup.gp_wrapper,
            SECOND_PROJ_TOKEN,
            0,
            &StaticMethods::get_second_token_full_amount(),
            |sc| {
                let signer_addr = managed_address!(&Address::from(&SIGNER_ADDRESS));

                sc.deposit_initial_rewards(1, 30, 30 + DEFAULT_MIN_REWARDS_PERIOD, signer_addr);
            },
        )
        .assert_ok();
}
