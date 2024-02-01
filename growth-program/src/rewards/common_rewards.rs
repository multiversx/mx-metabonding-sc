use week_timekeeping::Week;

use crate::project::ProjectId;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct RewardsInfo<M: ManagedTypeApi> {
    pub reward_token_id: TokenIdentifier<M>,
    pub undistributed_rewards: BigUint<M>,
    pub start_week: Week,
    pub end_week: Week,
}

#[multiversx_sc::module]
pub trait CommonRewardsModule {
    #[endpoint(updateRewards)]
    fn update_rewards_endpoint(&self, _project_id: ProjectId, _max_nr_weeks: OptionalValue<Week>) {
        // TODO
    }

    fn update_rewards(
        &self,
        _project_id: ProjectId,
        _max_nr_weeks: OptionalValue<Week>,
        _rewards_info: &mut RewardsInfo<Self::Api>,
    ) {
        // TODO
    }

    #[storage_mapper("minRewardsPeriod")]
    fn min_rewards_period(&self) -> SingleValueMapper<Week>;

    #[storage_mapper("minWeeklyRewardsValue")]
    fn min_weekly_rewards_value(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("rewardsInfo")]
    fn rewards_info(&self, project_id: ProjectId) -> SingleValueMapper<RewardsInfo<Self::Api>>;

    #[storage_mapper("rewardsTotalAmount")]
    fn rewards_total_amount(&self, project_id: ProjectId, week: Week)
        -> SingleValueMapper<BigUint>;

    #[storage_mapper("rewardsRemainingAmount")]
    fn rewards_remaining_amount(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> SingleValueMapper<BigUint>;
}
