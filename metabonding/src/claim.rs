elrond_wasm::imports!();

use crate::{
    project::PROJECT_EXPIRATION_WEEKS,
    rewards::{RewardsCheckpoint, Week},
    validation::Signature,
};

const MAX_CLAIM_ARG_PAIRS: usize = 5;
const CLAIM_NR_ARGS_PER_PAIR: usize = 4;

pub type ClaimArgPair<M> = MultiValue4<Week, BigUint<M>, BigUint<M>, Signature<M>>;

pub struct ClaimArgsWrapper<M: ManagedTypeApi> {
    pub week: Week,
    pub user_delegation_amount: BigUint<M>,
    pub user_lkmex_staked_amount: BigUint<M>,
    pub checkpoint: RewardsCheckpoint<M>,
}

#[elrond_wasm::module]
pub trait ClaimModule:
    elrond_wasm_modules::pause::PauseModule
    + crate::project::ProjectModule
    + crate::access_control::AccessControlModule
    + crate::common_storage::CommonStorageModule
    + crate::math::MathModule
    + crate::validation::ValidationModule
    + crate::rewards::RewardsModule
{
    /// Claims rewards for the given weeks. Maximum of MAX_CLAIM_ARG_PAIRS weeks can be claimed per call.
    /// Arguments are pairs of:
    /// week: number,
    /// user_delegation_amount: BigUint,
    /// user_lkmex_staked_amount: BigUint,
    /// signature: 120 bytes
    #[endpoint(claimRewards)]
    fn claim_rewards(&self, claim_args: MultiValueEncoded<ClaimArgPair<Self::Api>>) {
        require!(self.not_paused(), "May not claim rewards while paused");
        require!(
            claim_args.raw_len() / CLAIM_NR_ARGS_PER_PAIR <= MAX_CLAIM_ARG_PAIRS,
            "Too many arguments"
        );

        let caller = self.blockchain().get_caller();
        let current_week = self.get_current_week();
        let last_checkpoint_week = self.get_last_checkpoint_week();
        let rewards_nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();

        let mut args = ArrayVec::<ClaimArgsWrapper<Self::Api>, MAX_CLAIM_ARG_PAIRS>::new();
        for arg in claim_args {
            let (week, user_delegation_amount, user_lkmex_staked_amount, signature) =
                arg.into_tuple();

            require!(
                !self.rewards_claimed(&caller, week).get(),
                "Already claimed rewards for this week"
            );
            require!(week <= last_checkpoint_week, "No checkpoint for week yet");
            require!(
                self.is_claim_in_time(week, current_week, rewards_nr_first_grace_weeks),
                "Claiming too late"
            );

            let checkpoint: RewardsCheckpoint<Self::Api> = self.rewards_checkpoints().get(week);
            self.verify_signature(
                week,
                &caller,
                &user_delegation_amount,
                &user_lkmex_staked_amount,
                &signature,
            );

            self.rewards_claimed(&caller, week).set(true);

            args.push(ClaimArgsWrapper {
                week,
                user_delegation_amount,
                user_lkmex_staked_amount,
                checkpoint,
            });
        }

        let mut weekly_rewards = ManagedVec::new();
        for (id, project) in self.projects().iter() {
            let mut opt_rewards_for_project = None;

            for arg in &args {
                let opt_weekly_reward = self.get_weekly_reward_for_project(
                    &id,
                    &project,
                    current_week,
                    arg.week,
                    &arg.user_delegation_amount,
                    &arg.user_lkmex_staked_amount,
                    &arg.checkpoint.total_delegation_supply,
                    &arg.checkpoint.total_lkmex_staked,
                );

                if let Some(weekly_reward) = opt_weekly_reward {
                    match &mut opt_rewards_for_project {
                        Some(prev_amt) => *prev_amt += weekly_reward,
                        None => opt_rewards_for_project = Some(weekly_reward),
                    }
                }
            }

            if let Some(rewards_for_project) = opt_rewards_for_project {
                self.leftover_project_funds(&id)
                    .update(|leftover| *leftover -= &rewards_for_project);

                weekly_rewards.push(EsdtTokenPayment::new(
                    project.reward_token,
                    0,
                    rewards_for_project,
                ));
            }
        }

        if !weekly_rewards.is_empty() {
            self.send().direct_multi(&caller, &weekly_rewards);
        }
    }

    #[view(getUserClaimableWeeks)]
    fn get_user_claimable_weeks(&self, user_address: ManagedAddress) -> MultiValueEncoded<Week> {
        let last_checkpoint_week = self.get_last_checkpoint_week();
        let current_week = self.get_current_week();
        let rewards_nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();

        let start_week = if current_week <= rewards_nr_first_grace_weeks
            || PROJECT_EXPIRATION_WEEKS >= last_checkpoint_week
        {
            1
        } else {
            last_checkpoint_week - PROJECT_EXPIRATION_WEEKS
        };

        let mut weeks_list = MultiValueEncoded::new();
        for week in start_week..=last_checkpoint_week {
            if !self.rewards_claimed(&user_address, week).get()
                && self.is_claim_in_time(week, current_week, rewards_nr_first_grace_weeks)
            {
                weeks_list.push(week);
            }
        }

        weeks_list
    }

    #[storage_mapper("rewardsNrFirstGraceWeeks")]
    fn rewards_nr_first_grace_weeks(&self) -> SingleValueMapper<Week>;

    #[storage_mapper("rewardsClaimed")]
    fn rewards_claimed(&self, user: &ManagedAddress, week: Week) -> SingleValueMapper<bool>;
}
