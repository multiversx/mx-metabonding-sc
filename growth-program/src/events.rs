use crate::{
    project::ProjectId,
    rewards::{claim_types::ClaimType, week_timekeeping::Week},
};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct ClaimRewardsEventData<M: ManagedTypeApi> {
    pub project_id: ProjectId,
    pub amount: BigUint<M>,
    pub claim_type: ClaimType,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct DepositInitialRewardsEventData<M: ManagedTypeApi> {
    pub start_week: Week,
    pub end_week: Week,
    pub signer: ManagedAddress<M>,
    pub total_reward_amount: BigUint<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct DepositAdditionalRewardsEventData<M: ManagedTypeApi> {
    pub start_week: Week,
    pub end_week: Week,
    pub total_reward_amount: BigUint<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct OwnerWithdrawEventData<M: ManagedTypeApi> {
    pub start_week: Week,
    pub total_withdraw_amount: BigUint<M>,
}

#[multiversx_sc::module]
pub trait EventsModule {
    fn emit_claim_rewards_event(
        &self,
        caller: &ManagedAddress,
        project_id: ProjectId,
        rewards_amount: BigUint,
        claim_type: ClaimType,
    ) {
        let claim_data = ClaimRewardsEventData {
            amount: rewards_amount,
            project_id,
            claim_type,
        };
        self.claim_rewards_event(caller, &claim_data);
    }

    #[inline]
    fn emit_add_project_event(&self, project_owner: &ManagedAddress, project_id: ProjectId) {
        self.add_project_event(project_owner, project_id);
    }

    #[inline]
    fn emit_deposit_initial_rewards_event(
        &self,
        project_id: ProjectId,
        deposit_data: &DepositInitialRewardsEventData<Self::Api>,
    ) {
        self.deposit_initial_rewards_event(project_id, deposit_data);
    }

    fn emit_deposit_additional_rewards_event(
        &self,
        project_id: ProjectId,
        start_week: Week,
        end_week: Week,
        total_reward_amount: BigUint,
    ) {
        let deposit_data = DepositAdditionalRewardsEventData {
            start_week,
            end_week,
            total_reward_amount,
        };
        self.deposit_additional_rewards_event(project_id, &deposit_data);
    }

    fn emit_owner_withdraw_event(
        &self,
        project_id: ProjectId,
        start_week: Week,
        total_withdraw_amount: BigUint,
    ) {
        let withdraw_data = OwnerWithdrawEventData {
            start_week,
            total_withdraw_amount,
        };
        self.owner_withdraw_event(project_id, &withdraw_data);
    }

    #[inline]
    fn emit_change_signer_event(&self, project_id: ProjectId, new_signer: &ManagedAddress) {
        self.change_signer_event(project_id, new_signer);
    }

    #[inline]
    fn emit_pause_project_event(&self, project_id: ProjectId) {
        self.pause_project_event(project_id);
    }

    #[inline]
    fn emit_unpause_project_event(&self, project_id: ProjectId) {
        self.unpause_project_event(project_id);
    }

    #[event("claimRewardsEvent")]
    fn claim_rewards_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        claim_data: &ClaimRewardsEventData<Self::Api>,
    );

    #[event("addProjectEvent")]
    fn add_project_event(
        &self,
        #[indexed] project_owner: &ManagedAddress,
        #[indexed] project_id: ProjectId,
    );

    #[event("depositInitialRewardsEvent")]
    fn deposit_initial_rewards_event(
        &self,
        #[indexed] project_id: ProjectId,
        deposit_data: &DepositInitialRewardsEventData<Self::Api>,
    );

    #[event("depositAdditionalRewardsEvent")]
    fn deposit_additional_rewards_event(
        &self,
        #[indexed] project_id: ProjectId,
        deposit_data: &DepositAdditionalRewardsEventData<Self::Api>,
    );

    #[event("ownerWithdrawEvent")]
    fn owner_withdraw_event(
        &self,
        #[indexed] project_id: ProjectId,
        withdraw_data: &OwnerWithdrawEventData<Self::Api>,
    );

    #[event("changeSignerEvent")]
    fn change_signer_event(&self, #[indexed] project_id: ProjectId, new_signer: &ManagedAddress);

    #[event("pauseProjectEvent")]
    fn pause_project_event(&self, #[indexed] project_id: ProjectId);

    #[event("unpauseProjectEvent")]
    fn unpause_project_event(&self, #[indexed] project_id: ProjectId);
}
