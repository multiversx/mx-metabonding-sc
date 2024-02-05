#![allow(deprecated)]

pub mod growth_program_setup;
use growth_program::{
    rewards::{common_rewards::CommonRewardsModule, deposit::DepositRewardsModule},
    DEFAULT_MIN_REWARDS_PERIOD,
};
use growth_program_setup::*;
use multiversx_sc_scenario::{managed_biguint, rust_biguint, DebugApi};

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
                sc.deposit_initial_rewards(1, 2, 28, managed_biguint!(1));
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
                sc.deposit_initial_rewards(1, 2, 5, managed_biguint!(1));
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
