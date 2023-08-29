multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    claim::ClaimArgsWrapper,
    project::{Project, ProjectId},
};

pub type Week = usize;
pub type PrettyRewards<M> =
    MultiValueEncoded<M, MultiValue3<ProjectId<M>, TokenIdentifier<M>, BigUint<M>>>;

pub const FIRST_WEEK: usize = 1;

#[derive(TypeAbi, TopEncode, TopDecode, Clone)]
pub struct RewardsCheckpoint<M: ManagedTypeApi> {
    pub total_delegation_supply: BigUint<M>,
    pub total_lkmex_staked: BigUint<M>,
}

impl<M: ManagedTypeApi> Default for RewardsCheckpoint<M> {
    fn default() -> Self {
        Self {
            total_delegation_supply: BigUint::zero(),
            total_lkmex_staked: BigUint::zero(),
        }
    }
}

#[multiversx_sc::module]
pub trait RewardsModule:
    multiversx_sc_modules::pause::PauseModule
    + crate::project::ProjectModule
    + crate::access_control::AccessControlModule
    + crate::common_storage::CommonStorageModule
    + crate::math::MathModule
    + crate::validation::ValidationModule
{
    /// Adds a rewards checkpoint for the given Week. Only one checkpoint per week is allowed.
    /// Checkpoints have to be added in order, and only if the current week is equal to the given week
    ///     or the given week is in the past.
    /// Only the SC owner or the signer may add checkpoints. Arguments:
    /// - week - the week for which the checkpoint is added
    /// - total_delegation_supply - The total amount of staked EGLD in the Delegation SC
    /// - total_lkmex_staked - The total LKMEX staked in the Metabonding-Staking SC
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

    /// Deposits rewards for the given project. The full amount has to be deposited all at once.
    #[payable("*")]
    #[endpoint(depositRewards)]
    fn deposit_rewards(&self, project_id: ProjectId<Self::Api>) {
        require!(
            !self.rewards_deposited(&project_id).get(),
            "Rewards already deposited"
        );

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
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
        self.rewards_deposited(&project_id).set(true);
    }

    /// Gets rewards for the given week, assuming the user has the given staked EGLD and LKMEX amounts.
    /// Returned results are pairs of:
    /// - project_id
    /// - project_reward_token
    /// - reward_amount
    #[view(getRewardsForWeek)]
    fn get_rewards_for_week_pretty(
        &self,
        week: Week,
        user_delegation_amount: BigUint,
        user_lkmex_staked_amount: BigUint,
    ) -> PrettyRewards<Self::Api> {
        let checkpoint = self.rewards_checkpoints().get(week);
        let current_week = self.get_current_week();
        let claim_arg = ClaimArgsWrapper {
            week,
            user_delegation_amount,
            user_lkmex_staked_amount,
            checkpoint,
        };

        let mut rewards_pretty = MultiValueEncoded::new();
        for (id, project) in self.projects().iter() {
            let opt_weekly_reward =
                self.get_weekly_reward_for_project(&id, &project, current_week, &claim_arg);

            if let Some(weekly_reward) = opt_weekly_reward {
                rewards_pretty.push((id, project.reward_token, weekly_reward).into());
            }
        }

        rewards_pretty
    }

    fn get_weekly_reward_for_project(
        &self,
        project_id: &ProjectId<Self::Api>,
        project: &Project<Self::Api>,
        current_week: Week,
        claim_arg: &ClaimArgsWrapper<Self::Api>,
    ) -> Option<BigUint> {
        if !self.is_in_range(claim_arg.week, project.start_week, project.end_week) {
            return None;
        }
        if !self.rewards_deposited(project_id).get() {
            return None;
        }
        if project.is_expired(current_week) {
            return None;
        }

        let reward_amount = self.calculate_reward_amount(project, claim_arg);
        if reward_amount > 0 {
            Some(reward_amount)
        } else {
            None
        }
    }

    fn calculate_reward_amount(
        &self,
        project: &Project<Self::Api>,
        claim_arg: &ClaimArgsWrapper<Self::Api>,
    ) -> BigUint {
        let project_duration_weeks = project.get_duration_in_weeks() as u32;
        let rewards_supply_per_week_delegation =
            &project.delegation_reward_supply / project_duration_weeks;
        let rewards_supply_per_week_lkmex = &project.lkmex_reward_supply / project_duration_weeks;

        let rewards_delegation = self.calculate_ratio(
            &rewards_supply_per_week_delegation,
            &claim_arg.user_delegation_amount,
            &claim_arg.checkpoint.total_delegation_supply,
        );
        let rewards_lkmex = self.calculate_ratio(
            &rewards_supply_per_week_lkmex,
            &claim_arg.user_lkmex_staked_amount,
            &claim_arg.checkpoint.total_lkmex_staked,
        );

        rewards_delegation + rewards_lkmex
    }

    #[inline]
    fn get_last_checkpoint_week(&self) -> Week {
        self.rewards_checkpoints().len()
    }

    #[storage_mapper("rewardsCheckpoints")]
    fn rewards_checkpoints(&self) -> VecMapper<RewardsCheckpoint<Self::Api>>;
}
