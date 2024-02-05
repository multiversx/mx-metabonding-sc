use week_timekeeping::Week;

use crate::{project::ProjectId, rewards::common_rewards::RewardsInfo};

multiversx_sc::imports!();

pub static INVALID_START_WEEK_ERR_MSG: &[u8] = b"Invalid start week";

#[multiversx_sc::module]
pub trait DepositRewardsModule:
    crate::project::ProjectsModule
    + crate::price_query::PriceQueryModule
    + super::common_rewards::CommonRewardsModule
    + super::energy::EnergyModule
    + week_timekeeping::WeekTimekeepingModule
    + multiversx_sc_modules::pause::PauseModule
{
    #[only_owner]
    #[endpoint(setMinRewardsPeriod)]
    fn set_min_rewards_period(&self, nr_weeks: Week) {
        self.min_rewards_period().set(nr_weeks);
    }

    #[only_owner]
    #[endpoint(setMinWeeklyRewardsValue)]
    fn set_min_weekly_rewards_value(&self, value: BigUint) {
        self.min_weekly_rewards_value().set(value);
    }

    #[payable("*")]
    #[endpoint(depositInitialRewards)]
    fn deposit_initial_rewards(
        &self,
        project_id: ProjectId,
        start_week: Week,
        end_week: Week,
        initial_energy_per_rew_dollar: BigUint,
    ) {
        self.require_not_paused();
        self.require_valid_project_id(project_id);

        let caller = self.blockchain().get_caller();
        self.require_is_project_owner(&caller, project_id);

        let info_mapper = self.rewards_info(project_id);
        require!(info_mapper.is_empty(), "Initial rewards already deposited");
        require!(start_week < end_week, "Invalid week numbers");

        let week_diff = end_week - start_week;
        let min_rewards_period = self.min_rewards_period().get();
        require!(week_diff >= min_rewards_period, "Too few reward weeks");

        let current_week = self.get_current_week();
        require!(start_week > current_week, INVALID_START_WEEK_ERR_MSG);

        let (token_id, amount) = self.call_value().single_fungible_esdt();
        let rewards_per_week = &amount / week_diff as u32;
        let dollar_value = self.get_dollar_value(token_id.clone(), rewards_per_week.clone());
        let min_weekly_rewards_value = self.min_weekly_rewards_value().get();
        require!(dollar_value >= min_weekly_rewards_value, "Too few rewards");

        for week in start_week..end_week {
            self.rewards_total_amount(project_id, week)
                .set(&rewards_per_week);
            self.rewards_remaining_amount(project_id, week)
                .set(&rewards_per_week);
        }

        self.energy_per_reward_dollar_for_week(project_id, start_week)
            .set(initial_energy_per_rew_dollar);

        let surplus_amount = amount - &rewards_per_week * week_diff as u32;
        let surplus_payment = EsdtTokenPayment::new(token_id.clone(), 0, surplus_amount);
        self.send()
            .direct_non_zero_esdt_payment(&caller, &surplus_payment);

        let rewards_info = RewardsInfo {
            reward_token_id: token_id,
            undistributed_rewards: BigUint::zero(),
            start_week,
            end_week,
        };
        info_mapper.set(rewards_info);
    }

    #[payable("*")]
    #[endpoint(depositAdditionalRewards)]
    fn deposit_additional_rewards(&self, project_id: ProjectId, start_week: Week, end_week: Week) {
        self.require_not_paused();
        self.require_valid_project_id(project_id);

        let caller = self.blockchain().get_caller();
        self.require_is_project_owner(&caller, project_id);

        let info_mapper = self.rewards_info(project_id);
        require!(
            !info_mapper.is_empty(),
            "Must deposit initial rewards first"
        );
        require!(start_week < end_week, "Invalid week numbers");

        let (token_id, amount) = self.call_value().single_fungible_esdt();
        let mut rewards_info = info_mapper.get();
        require!(token_id == rewards_info.reward_token_id, "Invalid payment");

        let current_week = self.get_current_week();
        require!(
            rewards_info.end_week > current_week,
            "Project already ended"
        );
        require!(start_week > current_week, INVALID_START_WEEK_ERR_MSG);
        require!(
            rewards_info.end_week >= start_week,
            INVALID_START_WEEK_ERR_MSG
        );
        require!(end_week >= rewards_info.start_week, "Invalid end week");

        self.update_rewards(project_id, OptionalValue::None, &mut rewards_info);

        let week_diff = end_week - start_week;
        let rewards_per_week = &amount / week_diff as u32;
        let dollar_value = self.get_dollar_value(token_id.clone(), rewards_per_week.clone());
        let min_weekly_rewards_value = self.min_weekly_rewards_value().get();
        require!(dollar_value >= min_weekly_rewards_value, "Too few rewards");

        for week in start_week..end_week {
            self.rewards_total_amount(project_id, week)
                .update(|total| *total += &rewards_per_week);
            self.rewards_remaining_amount(project_id, week)
                .update(|remaining| *remaining += &rewards_per_week);
        }

        let surplus_amount = amount - &rewards_per_week * week_diff as u32;
        let surplus_payment = EsdtTokenPayment::new(token_id, 0, surplus_amount);
        self.send()
            .direct_non_zero_esdt_payment(&caller, &surplus_payment);

        rewards_info.start_week = core::cmp::min(rewards_info.start_week, start_week);
        rewards_info.end_week = core::cmp::max(rewards_info.end_week, end_week);

        info_mapper.set(rewards_info);
    }
}
