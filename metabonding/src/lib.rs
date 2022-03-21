#![no_std]

elrond_wasm::imports!();

mod project;
mod rewards;
mod util;

use core::borrow::Borrow;
use rewards::{RewardsCheckpoint, Week};
use util::Signature;

/// Source code for the pause module:
/// https://github.com/ElrondNetwork/elrond-wasm-rs/blob/master/elrond-wasm-modules/src/pause.rs
#[elrond_wasm::contract]
pub trait Metabonding:
    elrond_wasm_modules::pause::PauseModule
    + project::ProjectModule
    + rewards::RewardsModule
    + util::UtilModule
{
    #[init]
    fn init(&self, signer: ManagedAddress) {
        self.signer().set(&signer);
        self.set_paused(true);

        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set_if_empty(&current_epoch);
    }

    #[only_owner]
    #[endpoint(changeSigner)]
    fn change_signer(&self, new_signer: ManagedAddress) {
        self.signer().set(&new_signer);
    }

    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        week: Week,
        user_delegation_amount: BigUint,
        user_lkmex_staked_amount: BigUint,
        signature: Signature<Self::Api>,
    ) {
        require!(self.not_paused(), "May not claim rewards while paused");

        let caller = self.blockchain().get_caller();
        require!(
            !self.rewards_claimed(&caller, week).get(),
            "Already claimed rewards for this week"
        );

        let last_checkpoint_week = self.get_last_checkpoint_week();
        require!(week <= last_checkpoint_week, "No checkpoint for week yet");

        let checkpoint: RewardsCheckpoint<Self::Api> = self.rewards_checkpoints().get(week);
        self.verify_signature(
            week,
            &caller,
            &user_delegation_amount,
            &user_lkmex_staked_amount,
            &signature,
        );

        self.rewards_claimed(&caller, week).set(&true);

        let weekly_rewards = self.get_rewards_for_week(
            week,
            &user_delegation_amount,
            &user_lkmex_staked_amount,
            &checkpoint.total_delegation_supply,
            &checkpoint.total_lkmex_staked,
        );
        if !weekly_rewards.is_empty() {
            for (id, payment) in weekly_rewards.iter() {
                self.leftover_project_funds(id.borrow())
                    .update(|leftover| *leftover -= &payment.amount);
            }

            self.send()
                .direct_multi(&caller, &weekly_rewards.payments, &[]);
        }
    }
}
