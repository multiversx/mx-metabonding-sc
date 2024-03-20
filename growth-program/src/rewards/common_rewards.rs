use super::week_timekeeping::Week;

use crate::project::ProjectId;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct RewardsInfo<M: ManagedTypeApi> {
    pub reward_token_id: TokenIdentifier<M>,
    pub undistributed_rewards: BigUint<M>,
    pub start_week: Week,
    pub last_update_week: Week,
    pub end_week: Week,
}

impl<M: ManagedTypeApi> RewardsInfo<M> {
    pub fn new(reward_token_id: TokenIdentifier<M>, start_week: Week, end_week: Week) -> Self {
        RewardsInfo {
            reward_token_id,
            undistributed_rewards: BigUint::zero(),
            start_week,
            last_update_week: start_week,
            end_week,
        }
    }
}

#[multiversx_sc::module]
pub trait CommonRewardsModule: super::week_timekeeping::WeekTimekeepingModule {
    #[endpoint(updateRewards)]
    fn update_rewards_endpoint(
        &self,
        project_id: ProjectId,
        opt_max_nr_weeks: OptionalValue<Week>,
    ) {
        let mut rewards_info = self.rewards_info(project_id).get();
        self.update_rewards(project_id, opt_max_nr_weeks, &mut rewards_info);
    }

    fn update_rewards(
        &self,
        project_id: ProjectId,
        opt_max_nr_weeks: OptionalValue<Week>,
        rewards_info: &mut RewardsInfo<Self::Api>,
    ) {
        let current_week = self.get_current_week();
        if rewards_info.start_week > current_week {
            self.rewards_info(project_id).set(rewards_info);
            return;
        }

        if rewards_info.last_update_week >= current_week {
            self.rewards_info(project_id).set(rewards_info);
            return;
        }

        if rewards_info.last_update_week == rewards_info.end_week {
            self.rewards_info(project_id).set(rewards_info);
            return;
        }

        let last_week = match opt_max_nr_weeks {
            OptionalValue::Some(max_nr_weeks) => {
                let first_cmp_result =
                    core::cmp::min(rewards_info.last_update_week + max_nr_weeks, current_week);
                core::cmp::min(first_cmp_result, rewards_info.end_week)
            }
            OptionalValue::None => core::cmp::min(current_week, rewards_info.end_week),
        };

        let mut total_undistributed_rewards = BigUint::zero();
        for week in rewards_info.last_update_week..last_week {
            let undistributed_rewards = self.rewards_remaining_amount(project_id, week).take();
            total_undistributed_rewards += undistributed_rewards;
        }

        if last_week < rewards_info.end_week {
            self.rewards_remaining_amount(project_id, last_week)
                .update(|rem_rew| *rem_rew += &total_undistributed_rewards);
            self.rewards_total_amount(project_id, last_week)
                .update(|rew_total| *rew_total += total_undistributed_rewards);
        } else {
            rewards_info.undistributed_rewards += total_undistributed_rewards;
        }

        rewards_info.last_update_week = last_week;

        self.rewards_info(project_id).set(rewards_info);
    }

    #[storage_mapper("minRewardsPeriod")]
    fn min_rewards_period(&self) -> SingleValueMapper<Week>;

    #[storage_mapper("minWeeklyRewardsValue")]
    fn min_weekly_rewards_value(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardsInfo)]
    #[storage_mapper("rewardsInfo")]
    fn rewards_info(&self, project_id: ProjectId) -> SingleValueMapper<RewardsInfo<Self::Api>>;

    #[view(getRewardsTotalAmount)]
    #[storage_mapper("rewardsTotalAmount")]
    fn rewards_total_amount(&self, project_id: ProjectId, week: Week)
        -> SingleValueMapper<BigUint>;

    #[view(getRewardsRemainingAmount)]
    #[storage_mapper("rewardsRemainingAmount")]
    fn rewards_remaining_amount(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> SingleValueMapper<BigUint>;
}
