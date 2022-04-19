#![no_std]
#![feature(generic_associated_types)]

elrond_wasm::imports!();

pub mod access_control;
pub mod common_storage;
pub mod math;
pub mod project;
pub mod reward_data_types;
pub mod rewards;
pub mod validation;

use reward_data_types::Week;

/// Source code for the pause module:
/// https://github.com/ElrondNetwork/elrond-wasm-rs/blob/master/elrond-wasm-modules/src/pause.rs
#[elrond_wasm::contract]
pub trait Metabonding:
    elrond_wasm_modules::pause::PauseModule
    + project::ProjectModule
    + rewards::RewardsModule
    + access_control::AccessControlModule
    + common_storage::CommonStorageModule
    + math::MathModule
    + validation::ValidationModule
{
    #[init]
    fn init(
        &self,
        signer: ManagedAddress,
        #[var_args] opt_rewards_nr_first_grace_weeks: OptionalValue<Week>,
        #[var_args] opt_first_week_start_epoch: OptionalValue<u64>,
    ) {
        self.signer().set(&signer);
        self.set_paused(true);

        let rewards_nr_first_grace_weeks = match opt_rewards_nr_first_grace_weeks {
            OptionalValue::Some(nr) => nr,
            OptionalValue::None => 0,
        };
        self.rewards_nr_first_grace_weeks()
            .set(rewards_nr_first_grace_weeks);

        let first_week_start_epoch = match opt_first_week_start_epoch {
            OptionalValue::Some(epoch) => epoch,
            OptionalValue::None => self.blockchain().get_block_epoch(),
        };
        self.first_week_start_epoch()
            .set_if_empty(&first_week_start_epoch);
    }

    #[only_owner]
    #[endpoint(changeSigner)]
    fn change_signer(&self, new_signer: ManagedAddress) {
        self.signer().set(&new_signer);
    }
}
