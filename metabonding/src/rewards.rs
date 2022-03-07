elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::project::{Project, ProjectId};
use core::ops::Deref;

pub type ManagedHash<M> = ManagedByteArray<M, 32>;
pub type Week = usize;
pub type PrettyRewards<M> =
    MultiValueEncoded<M, MultiValue3<ProjectId<M>, TokenIdentifier<M>, BigUint<M>>>;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct RewardsCheckpoint<M: ManagedTypeApi> {
    pub root_hash: ManagedHash<M>,
    pub total_delegation_supply: BigUint<M>,
}

pub struct WeeklyRewards<M: ManagedTypeApi> {
    pub project_ids: ManagedVec<M, ProjectId<M>>,
    pub payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

impl<M: ManagedTypeApi> WeeklyRewards<M> {
    pub fn iter(
        &self,
    ) -> core::iter::Zip<
        ManagedVecRefIterator<M, ProjectId<M>>,
        ManagedVecRefIterator<M, EsdtTokenPayment<M>>,
    > {
        self.project_ids.iter().zip(self.payments.iter())
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.project_ids.is_empty()
    }
}

#[elrond_wasm::module]
pub trait RewardsModule: crate::project::ProjectModule {
    #[only_owner]
    #[endpoint(addRewardsCheckpoint)]
    fn add_rewards_checkpoint(
        &self,
        week: Week,
        root_hash: ManagedHash<Self::Api>,
        total_delegation_supply: BigUint,
    ) {
        let last_checkpoint_week = self.get_last_checkpoint_week();
        let current_week = self.get_current_week();
        require!(
            week == last_checkpoint_week + 1 && week <= current_week,
            "Invalid checkpoint week"
        );

        require!(
            !self.root_hash_known(&root_hash).get(),
            "Root hash already used"
        );
        require!(
            total_delegation_supply > 0,
            "Invalid total delegation supply"
        );

        self.root_hash_known(&root_hash).set(&true);

        let checkpoint = RewardsCheckpoint {
            total_delegation_supply,
            root_hash,
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
        let project: Project<Self::Api> = match self.projects().get(&project_id) {
            Some(p) => p,
            None => sc_panic!("Invalid project ID"),
        };

        require!(
            project.reward_token == payment_token,
            "Invalid payment token"
        );
        require!(project.reward_supply == payment_amount, "Invalid amount");

        self.rewards_deposited(&project_id).set(&true);
    }

    #[view(getRewardsForWeek)]
    fn get_rewards_for_week_pretty(
        &self,
        week: Week,
        user_delegation_amount: BigUint,
    ) -> PrettyRewards<Self::Api> {
        let checkpoint: RewardsCheckpoint<Self::Api> = self.rewards_checkpoints().get(week);
        let weekly_rewards = self.get_rewards_for_week(
            week,
            &user_delegation_amount,
            &checkpoint.total_delegation_supply,
        );

        let mut rewards_pretty = MultiValueEncoded::new();
        for (id, payment) in weekly_rewards.iter() {
            rewards_pretty
                .push((id.deref().clone(), payment.token_identifier, payment.amount).into());
        }

        rewards_pretty
    }

    fn get_rewards_for_week(
        &self,
        week: Week,
        user_delegation_amount: &BigUint,
        total_delegation_supply: &BigUint,
    ) -> WeeklyRewards<Self::Api> {
        let mut project_ids = ManagedVec::new();
        let mut user_rewards = ManagedVec::new();
        for (id, project) in self.projects().iter() {
            if !self.rewards_deposited(&id).get() {
                continue;
            }
            if !self.is_in_range(week, project.start_week, project.end_week) {
                continue;
            }

            let reward_amount = self.calculate_reward_amount(
                &project,
                user_delegation_amount,
                total_delegation_supply,
            );
            if reward_amount > 0 {
                project_ids.push(id);

                let user_payment = EsdtTokenPayment {
                    token_type: EsdtTokenType::Fungible,
                    token_identifier: project.reward_token,
                    token_nonce: 0,
                    amount: reward_amount,
                };
                user_rewards.push(user_payment);
            }
        }

        WeeklyRewards {
            project_ids,
            payments: user_rewards,
        }
    }

    fn calculate_reward_amount(
        &self,
        project: &Project<Self::Api>,
        user_delegation_amount: &BigUint,
        total_delegation_supply: &BigUint,
    ) -> BigUint {
        let project_duration_weeks = project.get_duration_in_weeks() as u32;
        let rewards_supply_per_week = &project.reward_supply / project_duration_weeks;

        &(&rewards_supply_per_week * user_delegation_amount) / total_delegation_supply
    }

    #[inline]
    fn get_last_checkpoint_week(&self) -> Week {
        self.rewards_checkpoints().len()
    }

    #[inline]
    fn is_in_range(&self, value: Week, min: Week, max: Week) -> bool {
        (min..=max).contains(&value)
    }

    #[storage_mapper("rewardsCheckpoints")]
    fn rewards_checkpoints(&self) -> VecMapper<RewardsCheckpoint<Self::Api>>;

    #[storage_mapper("rootHashKnown")]
    fn root_hash_known(&self, root_hash: &ManagedHash<Self::Api>) -> SingleValueMapper<bool>;

    #[storage_mapper("rewardsClaimed")]
    fn rewards_claimed(&self, user: &ManagedAddress, week: Week) -> SingleValueMapper<bool>;
}
