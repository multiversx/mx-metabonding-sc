elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{
    project::{Project, ProjectId, PROJECT_EXPIRATION_WEEKS},
    validation::Signature,
};

pub type Week = usize;
pub type PrettyRewards<M> =
    MultiValueEncoded<M, MultiValue3<ProjectId<M>, TokenIdentifier<M>, BigUint<M>>>;
pub type ClaimArgPair<M> = MultiValue4<Week, BigUint<M>, BigUint<M>, Signature<M>>;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct RewardsCheckpoint<M: ManagedTypeApi> {
    pub total_delegation_supply: BigUint<M>,
    pub total_lkmex_staked: BigUint<M>,
}

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

    // Arguments are pairs of:
    // week: number,
    // user_delegation_amount: BigUint,
    // user_lkmex_staked_amount: BigUint,
    // signature: 120 bytes
    #[endpoint(claimRewards)]
    fn claim_rewards(&self, #[var_args] claim_args: MultiValueEncoded<ClaimArgPair<Self::Api>>) {
        require!(self.not_paused(), "May not claim rewards while paused");

        let caller = self.blockchain().get_caller();
        let current_week = self.get_current_week();
        let last_checkpoint_week = self.get_last_checkpoint_week();
        let rewards_nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();

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

            self.rewards_claimed(&caller, week).set(&true);

            let mut weekly_rewards = ManagedVec::new();
            for (id, project) in self.projects().iter() {
                let opt_weekly_reward = self.get_weekly_reward_for_project(
                    &id,
                    &project,
                    current_week,
                    week,
                    &user_delegation_amount,
                    &user_lkmex_staked_amount,
                    &checkpoint.total_delegation_supply,
                    &checkpoint.total_lkmex_staked,
                );

                if let Some(weekly_reward) = opt_weekly_reward {
                    self.leftover_project_funds(&id)
                        .update(|leftover| *leftover -= &weekly_reward.amount);

                    weekly_rewards.push(weekly_reward);
                }
            }

            if !weekly_rewards.is_empty() {
                self.send().direct_multi(&caller, &weekly_rewards, &[]);
            }
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
        let current_week = self.get_current_week();
        let mut rewards_pretty = MultiValueEncoded::new();

        for (id, project) in self.projects().iter() {
            let opt_weekly_reward = self.get_weekly_reward_for_project(
                &id,
                &project,
                current_week,
                week,
                &user_delegation_amount,
                &user_lkmex_staked_amount,
                &checkpoint.total_delegation_supply,
                &checkpoint.total_lkmex_staked,
            );

            if let Some(weekly_reward) = opt_weekly_reward {
                rewards_pretty
                    .push((id, weekly_reward.token_identifier, weekly_reward.amount).into());
            }
        }

        rewards_pretty
    }

    fn get_weekly_reward_for_project(
        &self,
        project_id: &ProjectId<Self::Api>,
        project: &Project<Self::Api>,
        current_week: Week,
        week: Week,
        user_delegation_amount: &BigUint,
        user_lkmex_staked_amount: &BigUint,
        total_delegation_supply: &BigUint,
        total_lkmex_staked: &BigUint,
    ) -> Option<EsdtTokenPayment<Self::Api>> {
        if !self.is_in_range(week, project.start_week, project.end_week)
            || !self.rewards_deposited(project_id).get()
            || project.is_expired(current_week)
        {
            return None;
        }

        let reward_amount = self.calculate_reward_amount(
            project,
            user_delegation_amount,
            user_lkmex_staked_amount,
            total_delegation_supply,
            total_lkmex_staked,
        );

        Some(EsdtTokenPayment {
            token_type: EsdtTokenType::Fungible,
            token_identifier: project.reward_token.clone(),
            token_nonce: 0,
            amount: reward_amount,
        })
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
