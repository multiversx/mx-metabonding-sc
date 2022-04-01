pub mod metabonding_setup;

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
    mb_setup.deposit_rewards_default_projects();
}
