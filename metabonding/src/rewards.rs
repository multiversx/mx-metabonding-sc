elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::project::{Project, ProjectId};

pub type ManagedHash<M> = ManagedByteArray<M, 32>;
pub type Week = usize;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct RewardsCheckpoint<M: ManagedTypeApi> {
    pub root_hash: ManagedHash<M>,
    pub total_delegation_supply: BigUint<M>,
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
        require!(week == last_checkpoint_week + 1, "Invalid checkpoint week");

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

    #[storage_mapper("rewardsCheckpoints")]
    fn rewards_checkpoints(&self) -> VecMapper<RewardsCheckpoint<Self::Api>>;

    #[storage_mapper("rootHashKnown")]
    fn root_hash_known(&self, root_hash: &ManagedHash<Self::Api>) -> SingleValueMapper<bool>;

    #[storage_mapper("rewardsClaimed")]
    fn rewards_claimed(&self, user: &ManagedAddress, week: Week) -> SingleValueMapper<bool>;
}
