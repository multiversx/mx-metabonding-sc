use crate::rewards::claim::ClaimType;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct ClaimRewardsEventData<M: ManagedTypeApi> {
    pub amount: BigUint<M>,
    pub claim_type: ClaimType,
}

#[multiversx_sc::module]
pub trait EventsModule {
    fn emit_claim_rewards_event(
        &self,
        caller: &ManagedAddress,
        rewards_amount: BigUint,
        claim_type: ClaimType,
    ) {
        let claim_data = ClaimRewardsEventData {
            amount: rewards_amount,
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
