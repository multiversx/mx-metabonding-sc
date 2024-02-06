use week_timekeeping::Week;

use crate::project::ProjectId;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct RewardsInfo<M: ManagedTypeApi> {
    pub reward_token_id: TokenIdentifier<M>,
    pub undistributed_rewards: BigUint<M>,
    pub start_week: Week,
    pub end_week: Week,
}

#[multiversx_sc::module]
pub trait CommonRewardsModule: week_timekeeping::WeekTimekeepingModule {
    #[endpoint(updateRewards)]
    fn update_rewards_endpoint(
        &self,
        project_id: ProjectId,
        opt_max_nr_weeks: OptionalValue<Week>,
    ) {
        self.rewards_info(project_id).update(|rewards_info| {
            self.update_rewards(project_id, opt_max_nr_weeks, rewards_info);
        });
    }

    fn update_rewards(
        &self,
        project_id: ProjectId,
        opt_max_nr_weeks: OptionalValue<Week>,
        rewards_info: &mut RewardsInfo<Self::Api>,
    ) {
        let current_week = self.get_current_week();
        if rewards_info.start_week >= current_week {
            return;
        }

        if rewards_info.start_week == rewards_info.end_week {
            return;
        }

        let last_week = match opt_max_nr_weeks {
            OptionalValue::Some(max_nr_weeks) => {
                let first_cmp_result =
                    core::cmp::min(rewards_info.start_week + max_nr_weeks, current_week);
                core::cmp::min(first_cmp_result, rewards_info.end_week)
            }
            OptionalValue::None => core::cmp::min(current_week, rewards_info.end_week),
        };

        let mut total_undistributed_rewards = BigUint::zero();
        for week in rewards_info.start_week..last_week {
            let undistributed_rewards = self.rewards_remaining_amount(project_id, week).take();
            total_undistributed_rewards += undistributed_rewards;
        }

        if last_week < rewards_info.end_week - 1 {
            self.rewards_remaining_amount(project_id, last_week)
                .update(|rem_rew| *rem_rew += total_undistributed_rewards);
        } else {
            rewards_info.undistributed_rewards += total_undistributed_rewards;
        }

        rewards_info.start_week = last_week;
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
