elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{
    project::{Project, ProjectId, ProjectsMapperCache, PROJECT_EXPIRATION_WEEKS},
    reward_data_types::*,
    validation::Signature,
};
use core::{borrow::Borrow, ops::Deref};

#[elrond_wasm::module]
pub trait RewardsModule:
    elrond_wasm_modules::pause::PauseModule
    + crate::project::ProjectModule
    + crate::access_control::AccessControlModule
    + crate::common_storage::CommonStorageModule
    + crate::math::MathModule
    + crate::validation::ValidationModule
{
    #[endpoint(addRewardsCheckpoint)]
    fn add_rewards_checkpoint(
        &self,
        week: Week,
        total_delegation_supply: BigUint,
        total_lkmex_staked: BigUint,
    ) {
        self.require_caller_owner_or_signer();

        let last_checkpoint_week = self.get_last_checkpoint_week();
        let current_week = self.get_current_week();
        require!(
            week == last_checkpoint_week + 1 && week <= current_week,
            "Invalid checkpoint week"
        );

        let checkpoint = RewardsCheckpoint {
            total_delegation_supply,
            total_lkmex_staked,
        };
        self.rewards_checkpoints().push(&checkpoint);
    }

    #[payable("*")]
    #[endpoint(depositRewards)]
    fn deposit_rewards(&self, project_id: ProjectId<Self::Api>) {
        require!(
            !self.rewards_deposited(&project_id).get(),
            "Rewards already deposited"
        );

        let (payment_amount, payment_token) = self.call_value().payment_token_pair();
        let project = self.get_project_or_panic(&project_id);

        let caller = self.blockchain().get_caller();
        let project_owner = self.project_owner(&project_id).get();
        require!(
            caller == project_owner,
            "Only project owner may deposit the rewards"
        );

        let current_week = self.get_current_week();
        require!(!project.is_expired(current_week), "Project is expired");

        let total_reward_supply = project.lkmex_reward_supply + project.delegation_reward_supply;
        require!(
            project.reward_token == payment_token,
            "Invalid payment token"
        );
        require!(total_reward_supply == payment_amount, "Invalid amount");

        self.leftover_project_funds(&project_id)
            .set(&total_reward_supply);
        self.rewards_deposited(&project_id).set(&true);
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

        let current_week = self.get_current_week();
        let rewards_nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();
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

        self.rewards_claimed(&caller, week).set(&true);

        let mut projects_cache = self.get_projects_mapper_cache();
        let weekly_rewards = self.get_rewards_for_week(
            &projects_cache,
            week,
            &user_delegation_amount,
            &user_lkmex_staked_amount,
            &checkpoint.total_delegation_supply,
            &checkpoint.total_lkmex_staked,
        );
        let trimmed_payments = weekly_rewards.get_trimmed_payments(&mut projects_cache.project_ids);

        if !trimmed_payments.is_empty() {
            for (id, payment) in projects_cache
                .project_ids
                .iter()
                .zip(trimmed_payments.iter())
            {
                self.leftover_project_funds(id.borrow())
                    .update(|leftover| *leftover -= &payment.amount);
            }

            self.send().direct_multi(&caller, &trimmed_payments, &[]);
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

    #[view(getRewardsForWeek)]
    fn get_rewards_for_week_pretty(
        &self,
        week: Week,
        user_delegation_amount: BigUint,
        user_lkmex_staked_amount: BigUint,
    ) -> PrettyRewards<Self::Api> {
        let checkpoint: RewardsCheckpoint<Self::Api> = self.rewards_checkpoints().get(week);
        let mut projects_cache = self.get_projects_mapper_cache();
        let weekly_rewards = self.get_rewards_for_week(
            &projects_cache,
            week,
            &user_delegation_amount,
            &user_lkmex_staked_amount,
            &checkpoint.total_delegation_supply,
            &checkpoint.total_lkmex_staked,
        );
        let trimmed_payments = weekly_rewards.get_trimmed_payments(&mut projects_cache.project_ids);

        let mut rewards_pretty = MultiValueEncoded::new();
        for (id, payment) in projects_cache
            .project_ids
            .iter()
            .zip(trimmed_payments.iter())
        {
            rewards_pretty
                .push((id.deref().clone(), payment.token_identifier, payment.amount).into());
        }

        rewards_pretty
    }

    fn get_rewards_multiple_weeks(&self) {}

    fn get_rewards_for_week(
        &self,
        projects_cache: &ProjectsMapperCache<Self::Api>,
        week: Week,
        user_delegation_amount: &BigUint,
        user_lkmex_staked_amount: &BigUint,
        total_delegation_supply: &BigUint,
        total_lkmex_staked: &BigUint,
    ) -> WeeklyRewards<Self::Api> {
        let current_week = self.get_current_week();
        let mut user_rewards = ManagedVec::new();

        for (id, project) in projects_cache.iter() {
            let mut is_zero_amount = false;
            if !self.is_in_range(week, project.start_week, project.end_week) {
                is_zero_amount = true;
            } else if !self.rewards_deposited(&id).get() {
                is_zero_amount = true;
            } else if project.is_expired(current_week) {
                is_zero_amount = true;
            }

            let reward_amount = if is_zero_amount {
                BigUint::zero()
            } else {
                self.calculate_reward_amount(
                    &project,
                    user_delegation_amount,
                    user_lkmex_staked_amount,
                    total_delegation_supply,
                    total_lkmex_staked,
                )
            };
            let user_payment = EsdtTokenPayment {
                token_type: EsdtTokenType::Fungible,
                token_identifier: project.reward_token,
                token_nonce: 0,
                amount: reward_amount,
            };
            user_rewards.push(user_payment);
        }

        WeeklyRewards::new(user_rewards)
    }

    fn calculate_reward_amount(
        &self,
        project: &Project<Self::Api>,
        user_delegation_amount: &BigUint,
        user_lkmex_staked_amount: &BigUint,
        total_delegation_supply: &BigUint,
        total_lkmex_staked: &BigUint,
    ) -> BigUint {
        let project_duration_weeks = project.get_duration_in_weeks() as u32;
        let rewards_supply_per_week_delegation =
            &project.delegation_reward_supply / project_duration_weeks;
        let rewards_supply_per_week_lkmex = &project.lkmex_reward_supply / project_duration_weeks;

        let rewards_delegation = self.calculate_ratio(
            &rewards_supply_per_week_delegation,
            user_delegation_amount,
            total_delegation_supply,
        );
        let rewards_lkmex = self.calculate_ratio(
            &rewards_supply_per_week_lkmex,
            user_lkmex_staked_amount,
            total_lkmex_staked,
        );

        rewards_delegation + rewards_lkmex
    }

    fn is_claim_in_time(
        &self,
        claim_week: Week,
        current_week: Week,
        rewards_nr_first_grace_weeks: Week,
    ) -> bool {
        current_week <= rewards_nr_first_grace_weeks
            || current_week <= claim_week + PROJECT_EXPIRATION_WEEKS
    }

    #[inline]
    fn get_last_checkpoint_week(&self) -> Week {
        self.rewards_checkpoints().len()
    }

    #[storage_mapper("rewardsNrFirstGraceWeeks")]
    fn rewards_nr_first_grace_weeks(&self) -> SingleValueMapper<Week>;

    #[storage_mapper("rewardsCheckpoints")]
    fn rewards_checkpoints(&self) -> VecMapper<RewardsCheckpoint<Self::Api>>;

    #[storage_mapper("rewardsClaimed")]
    fn rewards_claimed(&self, user: &ManagedAddress, week: Week) -> SingleValueMapper<bool>;
}
