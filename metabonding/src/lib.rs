#![no_std]

use rewards::Week;

elrond_wasm::imports!();

pub mod access_control;
pub mod claim;
pub mod common_storage;
pub mod math;
pub mod project;
pub mod rewards;
pub mod validation;

/// Source code for the pause module:
/// https://github.com/ElrondNetwork/elrond-wasm-rs/blob/master/elrond-wasm-modules/src/pause.rs
#[elrond_wasm::contract]
pub trait Metabonding:
    elrond_wasm_modules::pause::PauseModule
    + project::ProjectModule
    + rewards::RewardsModule
    + claim::ClaimModule
    + access_control::AccessControlModule
    + common_storage::CommonStorageModule
    + math::MathModule
    + validation::ValidationModule
{
    /// Arguments:
    /// - signer - public key that will be used for checking the claim signatures
    /// - opt_rewards_nr_first_grace_weeks - Optional argument that will make it so
    ///     the first X weeks can be claimed at any time (i.e. they will never expire)
    /// - opt_first_week_start_epoch - The epoch which signals the start of week 0.
    ///     Can also be an epoch from the past.
    ///     By default, the current epoch on deploy will be used
    #[init]
    fn init(
        &self,
        signer: ManagedAddress,
        opt_rewards_nr_first_grace_weeks: OptionalValue<Week>,
        opt_first_week_start_epoch: OptionalValue<u64>,
    ) {
        self.signer().set(&signer);
        self.set_paused(true);

        let rewards_nr_first_grace_weeks = match opt_rewards_nr_first_grace_weeks {
            OptionalValue::Some(nr) => nr,
            OptionalValue::None => 0,
        };
        self.rewards_nr_first_grace_weeks()
            .set_if_empty(rewards_nr_first_grace_weeks);

        let first_week_start_epoch = match opt_first_week_start_epoch {
            OptionalValue::Some(epoch) => epoch,
            OptionalValue::None => self.blockchain().get_block_epoch(),
        };
        self.first_week_start_epoch()
            .set_if_empty(first_week_start_epoch);
    }

    #[only_owner]
    #[endpoint(changeSigner)]
    fn change_signer(&self, new_signer: ManagedAddress) {
        self.signer().set(&new_signer);
    }
}
