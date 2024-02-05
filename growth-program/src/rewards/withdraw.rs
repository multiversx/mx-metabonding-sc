use week_timekeeping::Week;

use crate::{project::ProjectId, rewards::deposit::INVALID_START_WEEK_ERR_MSG};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait WithdrawRewardsModule:
    super::common_rewards::CommonRewardsModule
    + crate::project::ProjectsModule
    + week_timekeeping::WeekTimekeepingModule
{
    #[only_owner]
    #[endpoint(ownerWithdrawRewards)]
    fn owner_withdraw_rewards(&self, project_id: ProjectId, start_week: Week) {
        self.require_valid_project_id(project_id);

        let info_mapper = self.rewards_info(project_id);
        let mut rewards_info = info_mapper.get();
        let current_week = self.get_current_week();
        require!(
            current_week < rewards_info.end_week,
            "Cannot withdraw anymore"
        );
        require!(
            start_week > rewards_info.start_week,
            INVALID_START_WEEK_ERR_MSG
        );
        require!(
            start_week < rewards_info.end_week,
            INVALID_START_WEEK_ERR_MSG
        );
        require!(start_week > current_week, INVALID_START_WEEK_ERR_MSG);

        self.update_rewards(project_id, OptionalValue::None, &mut rewards_info);

        let mut total_amount = BigUint::zero();
        for week in start_week..=rewards_info.end_week {
            let remaining_rewards = self.rewards_remaining_amount(project_id, week).take();
            total_amount += remaining_rewards;

            self.rewards_total_amount(project_id, week).clear();
        }

        let caller = self.blockchain().get_caller();
        let payment = EsdtTokenPayment::new(rewards_info.reward_token_id.clone(), 0, total_amount);
        self.send().direct_non_zero_esdt_payment(&caller, &payment);

        if start_week == rewards_info.start_week {
            info_mapper.clear();
        } else {
            rewards_info.end_week = start_week;
            info_mapper.set(rewards_info);
        }
    }

    #[endpoint(finishProgram)]
    fn finish_program(&self, project_id: ProjectId) {
        let caller = self.blockchain().get_caller();
        self.require_is_project_owner(&caller, project_id);

        let mut rewards_info = self.rewards_info(project_id).take();
        let current_week = self.get_current_week();
        require!(
            current_week >= rewards_info.end_week,
            "End week not reached"
        );

        self.update_rewards(project_id, OptionalValue::None, &mut rewards_info);

        let remaining_rewards = EsdtTokenPayment::new(
            rewards_info.reward_token_id,
            0,
            rewards_info.undistributed_rewards,
        );
        self.send()
            .direct_non_zero_esdt_payment(&caller, &remaining_rewards);
    }
}
