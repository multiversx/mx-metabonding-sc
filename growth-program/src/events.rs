use crate::{project::ProjectId, rewards::claim_types::ClaimType};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct ClaimRewardsEventData<M: ManagedTypeApi> {
    pub project_id: ProjectId,
    pub amount: BigUint<M>,
    pub claim_type: ClaimType,
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

    #[event("claimRewardsEvent")]
    fn claim_rewards_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        claim_data: &ClaimRewardsEventData<Self::Api>,
    );
}
