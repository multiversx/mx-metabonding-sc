pub mod metabonding_setup;

use elrond_wasm_debug::rust_biguint;
use metabonding::rewards::RewardsModule;
use metabonding_setup::*;

#[test]
fn init_test() {
    let _ = MetabondingSetup::new(metabonding::contract_obj);
}

#[test]
fn add_projects_test() {
    let mut mb_setup = MetabondingSetup::new(metabonding::contract_obj);
    mb_setup.add_default_projects();

    // try add project 2 again
    let second_proj_owner = mb_setup.second_project_owner.clone();
    mb_setup
        .call_add_project(
            SECOND_PROJ_ID,
            &second_proj_owner,
            SECOND_PROJ_TOKEN,
            TOTAL_SECOND_PROJ_TOKENS,
            2,
            5,
            0,
        )
        .assert_user_error("ID already in use");

    // get project IDs
    let project_ids = mb_setup.get_all_project_ids();
    assert_eq!(project_ids.len(), 2);
    assert_eq!(project_ids[0], FIRST_PROJ_ID);
    assert_eq!(project_ids[1], SECOND_PROJ_ID);

    // get first project
    let (token, reward_amount, lkmex_rewards_supply, start_week, duration) =
        mb_setup.get_project_by_id(FIRST_PROJ_ID);
    assert_eq!(token, FIRST_PROJ_TOKEN);
    assert_eq!(reward_amount, TOTAL_FIRST_PROJ_TOKENS);
    assert_eq!(lkmex_rewards_supply, 0);
    assert_eq!(start_week, 1);
    assert_eq!(duration, 3);

    // get second project
    let (token, reward_amount, lkmex_rewards_supply, start_week, duration) =
        mb_setup.get_project_by_id(SECOND_PROJ_ID);
    assert_eq!(token, SECOND_PROJ_TOKEN);
    assert_eq!(reward_amount, TOTAL_SECOND_PROJ_TOKENS);
    assert_eq!(lkmex_rewards_supply, 0);
    assert_eq!(start_week, 2);
    assert_eq!(duration, 6);
}

#[test]
fn deposit_rewards_test() {
    let mut mb_setup = MetabondingSetup::new(metabonding::contract_obj);
    mb_setup.add_default_projects();

    // try deposit rewards not project owner
    let rand_user = mb_setup.b_mock.create_user_account(&rust_biguint!(0));
    mb_setup.b_mock.set_esdt_balance(
        &rand_user,
        &FIRST_PROJ_TOKEN,
        &rust_biguint!(TOTAL_FIRST_PROJ_TOKENS),
    );
    mb_setup
        .call_deposit_rewards(
            &rand_user,
            FIRST_PROJ_ID,
            FIRST_PROJ_TOKEN,
            TOTAL_FIRST_PROJ_TOKENS,
        )
        .assert_user_error("Only project owner may deposit the rewards");

    mb_setup.deposit_rewards_default_projects();

    // try deposit rewards again
    let first_proj_owner = mb_setup.first_project_owner.clone();
    mb_setup.b_mock.set_esdt_balance(
        &first_proj_owner,
        &FIRST_PROJ_TOKEN,
        &rust_biguint!(TOTAL_FIRST_PROJ_TOKENS),
    );
    mb_setup
        .call_deposit_rewards(
            &first_proj_owner,
            FIRST_PROJ_ID,
            FIRST_PROJ_TOKEN,
            TOTAL_FIRST_PROJ_TOKENS,
        )
        .assert_user_error("Rewards already deposited");
}

#[test]
fn add_rewards_checkpoints_test() {
    let mut mb_setup = MetabondingSetup::new(metabonding::contract_obj);
    mb_setup.add_default_projects();
    mb_setup.deposit_rewards_default_projects();
    mb_setup.add_default_checkpoints();

    // try add checkpoint for week 1 again
    mb_setup
        .call_add_rewards_checkpoint(1, 500_000, 0)
        .assert_user_error("Invalid checkpoint week");

    // try add checkpoint week 4 (next week should be week 3)
    mb_setup
        .call_add_rewards_checkpoint(4, 500_000, 0)
        .assert_user_error("Invalid checkpoint week");
}

#[test]
fn claim_rewards_test() {
    let mut mb_setup = MetabondingSetup::new(metabonding::contract_obj);
    mb_setup.add_default_projects();
    mb_setup.deposit_rewards_default_projects();
    mb_setup.add_default_checkpoints();

    let first_user_addr = mb_setup.first_user_addr.clone();
    let second_user_addr = mb_setup.second_user_addr.clone();
    let sig_first_user_week_1 = hex_literal::hex!("d47c0d67b2d25de8b4a3f43d91a2b5ccb522afac47321ae80bf89c90a4445b26adefa693ab685fa20891f736d74eb2dedc11c4b1a8d6e642fa28df270d6ebe08");
    let sig_second_user_week_1 = hex_literal::hex!("301e68ce4c473d891f033bc53cc4fd62974cb1c2b80c3fc531d4289cdde4b8f09a650686f2233fd83cb1620b73b8649d3bdd94ab4af5cd479139d04b565a920e");
    let sig_first_user_week_2 = hex_literal::hex!("b4aadf08eea4cc7c636922511943edbab2ff6ef2558528e0e7b03c7448367989fe860ac091be4d942304f04c86b1eaa0501f36e02819a3c628b4c53f3d3ac801");
    let sig_second_user_week_2 = hex_literal::hex!("19042097100e5dcd0c11c47d5f8b85d11c744625deeea370b5127797c8b30467c0eb04c153c3d644394a06fba3eef4794a9cdd7cbe61c722984da2f6c7a1f90b");

    // get claimable weeks
    let claimable_weeks = mb_setup.get_user_claimable_weeks(&first_user_addr);
    assert_eq!(claimable_weeks, &[1usize, 2usize]);

    // get rewards week 1
    let rewards_week_1_first_user = mb_setup.get_pretty_rewards(1, 25_000, 0);
    assert_eq!(
        rewards_week_1_first_user,
        &[(
            b"FirstProj".to_vec(),
            b"PROJ-123456".to_vec(),
            83_333_333u64
        )]
    );

    let rewards_week_1_second_user = mb_setup.get_pretty_rewards(1, 50_000, 0);
    assert_eq!(
        rewards_week_1_second_user,
        &[(
            b"FirstProj".to_vec(),
            b"PROJ-123456".to_vec(),
            166_666_666u64
        )]
    );

    // try claim while paused
    mb_setup
        .call_claim_rewards(&first_user_addr, 1, 25_000, 0, &sig_first_user_week_1)
        .assert_user_error("May not claim rewards while paused");

    // unpause
    mb_setup.call_unpause().assert_ok();

    // try claim wrong user delegation supply
    mb_setup
        .call_claim_rewards(&first_user_addr, 1, 30_000, 0, &sig_first_user_week_1)
        .assert_user_error("Invalid signature");

    // try claim wrong week
    mb_setup
        .call_claim_rewards(&first_user_addr, 5, 25_000, 0, &sig_first_user_week_1)
        .assert_user_error("No checkpoint for week yet");

    // claim first user week 1 ok
    mb_setup
        .call_claim_rewards(&first_user_addr, 1, 25_000, 0, &sig_first_user_week_1)
        .assert_ok();
    mb_setup.b_mock.check_esdt_balance(
        &first_user_addr,
        FIRST_PROJ_TOKEN,
        &rust_biguint!(83_333_333),
    );

    // try claim week 1 again
    mb_setup
        .call_claim_rewards(&first_user_addr, 1, 25_000, 0, &sig_first_user_week_1)
        .assert_user_error("Already claimed rewards for this week");

    // claim second user week 1 ok
    mb_setup
        .call_claim_rewards(&second_user_addr, 1, 50_000, 0, &sig_second_user_week_1)
        .assert_ok();
    mb_setup.b_mock.check_esdt_balance(
        &second_user_addr,
        FIRST_PROJ_TOKEN,
        &rust_biguint!(166_666_666),
    );

    // get rewards week 2
    let rewards_week_2_first_user = mb_setup.get_pretty_rewards(2, 25_000, 0);
    assert_eq!(
        rewards_week_2_first_user,
        &[
            (
                b"FirstProj".to_vec(),
                b"PROJ-123456".to_vec(),
                41_666_666u64
            ),
            (
                b"SecondProj".to_vec(),
                b"COOL-123456".to_vec(),
                50_000_000u64
            )
        ]
    );
    let rewards_week_2_second_user = mb_setup.get_pretty_rewards(2, 50_000, 0);
    assert_eq!(
        rewards_week_2_second_user,
        &[
            (
                b"FirstProj".to_vec(),
                b"PROJ-123456".to_vec(),
                83_333_333u64
            ),
            (
                b"SecondProj".to_vec(),
                b"COOL-123456".to_vec(),
                100_000_000u64
            )
        ]
    );

    // claim first user week 2 ok
    mb_setup
        .call_claim_rewards(&first_user_addr, 2, 25_000, 0, &sig_first_user_week_2)
        .assert_ok();
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

    // claim second user week 2 ok
    mb_setup
        .call_claim_rewards(&second_user_addr, 2, 50_000, 0, &sig_second_user_week_2)
        .assert_ok();
    mb_setup.b_mock.check_esdt_balance(
        &second_user_addr,
        FIRST_PROJ_TOKEN,
        &rust_biguint!(166_666_666 + 83_333_333),
    );
    mb_setup.b_mock.check_esdt_balance(
        &second_user_addr,
        SECOND_PROJ_TOKEN,
        &rust_biguint!(100_000_000),
    );
}

#[test]
fn grace_period_test() {
    let mut mb_setup = MetabondingSetup::new(metabonding::contract_obj);
    mb_setup.add_default_projects();
    mb_setup.deposit_rewards_default_projects();
    mb_setup.add_default_checkpoints();
    mb_setup.call_unpause().assert_ok();

    let owner_addr = mb_setup.owner_addr.clone();
    let first_user_addr = mb_setup.first_user_addr.clone();
    let sig_first_user_week_1 = hex_literal::hex!("d47c0d67b2d25de8b4a3f43d91a2b5ccb522afac47321ae80bf89c90a4445b26adefa693ab685fa20891f736d74eb2dedc11c4b1a8d6e642fa28df270d6ebe08");

    // set current week = 6
    mb_setup.b_mock.set_block_epoch(50);

    // get claimable weeks - only able to claim week 2
    let claimable_weeks = mb_setup.get_user_claimable_weeks(&first_user_addr);
    assert_eq!(claimable_weeks, &[2usize]);

    // set grace period of 5 weeks,
    mb_setup
        .b_mock
        .execute_tx(&owner_addr, &mb_setup.mb_wrapper, &rust_biguint!(0), |sc| {
            sc.rewards_nr_first_grace_weeks().set(5);
        })
        .assert_ok();

    // get claimable weeks - user still can only claim for week 2
    let claimable_weeks = mb_setup.get_user_claimable_weeks(&first_user_addr);
    assert_eq!(claimable_weeks, &[2usize]);

    // user try claim week 1
    mb_setup
        .call_claim_rewards(&first_user_addr, 1, 25_000, 0, &sig_first_user_week_1)
        .assert_user_error("Claiming too late");

    // set grace weeks to 6
    mb_setup
        .b_mock
        .execute_tx(&owner_addr, &mb_setup.mb_wrapper, &rust_biguint!(0), |sc| {
            sc.rewards_nr_first_grace_weeks().set(6);
        })
        .assert_ok();

    // get claimable weeks - user can now claim for week 1 and 2
    let claimable_weeks = mb_setup.get_user_claimable_weeks(&first_user_addr);
    assert_eq!(claimable_weeks, &[1usize, 2usize]);

    // user claim week 1 ok
    mb_setup
        .call_claim_rewards(&first_user_addr, 1, 25_000, 0, &sig_first_user_week_1)
        .assert_ok();
}
