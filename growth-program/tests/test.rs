#![allow(deprecated)]

pub mod growth_program_setup;
use growth_program_setup::*;

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
