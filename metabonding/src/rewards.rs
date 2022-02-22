elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::project::{Epoch, Project, ProjectId};

pub type ManagedHash<M> = ManagedByteArray<M, 32>;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct RewardsCheckpoint<M: ManagedTypeApi> {
    pub total_delegation_supply: BigUint<M>,
    pub epoch: Epoch,
}

#[elrond_wasm::module]
pub trait RewardsModule: crate::project::ProjectModule {
    #[only_owner]
    #[endpoint(addRewardsCheckpoint)]
    fn add_rewards_checkpoint(
        &self,
        root_hash: ManagedHash<Self::Api>,
        total_delegation_supply: BigUint,
        epoch: Epoch,
    ) {
        require!(
            self.rewards_checkpoints(&root_hash).is_empty(),
            "Checkpoint already exists"
        );

        let checkpoint = RewardsCheckpoint {
            total_delegation_supply,
            epoch,
        };
        self.rewards_checkpoints(&root_hash).set(&checkpoint);
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
        rewards_supply: &BigUint,
        user_delegation_amount: &BigUint,
        total_delegation_supply: &BigUint,
    ) -> BigUint {
        &(rewards_supply * user_delegation_amount) / total_delegation_supply
    }

    #[storage_mapper("rewardsCheckpoints")]
    fn rewards_checkpoints(
        &self,
        root_hash: &ManagedHash<Self::Api>,
    ) -> SingleValueMapper<RewardsCheckpoint<Self::Api>>;

    #[storage_mapper("rewardsDeposited")]
    fn rewards_deposited(&self, project_id: &ProjectId<Self::Api>) -> SingleValueMapper<bool>;

    #[storage_mapper("rewardsClaimed")]
    fn rewards_claimed(
        &self,
        user: &ManagedAddress,
        root_hash: &ManagedHash<Self::Api>,
    ) -> SingleValueMapper<bool>;
}
