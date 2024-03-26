use super::week_timekeeping::Week;

use crate::{
    events::DepositInitialRewardsEventData,
    project::{ProjectId, PROJECT_UNPAUSED},
    rewards::common_rewards::RewardsInfo,
    DAY_IN_SECONDS,
};

multiversx_sc::imports!();

pub static INVALID_START_WEEK_ERR_MSG: &[u8] = b"Invalid start week";

#[multiversx_sc::module]
pub trait DepositRewardsModule:
    crate::project::ProjectsModule
    + crate::events::EventsModule
    + crate::price_query::PriceQueryModule
    + super::common_rewards::CommonRewardsModule
    + super::energy::EnergyModule
    + super::week_timekeeping::WeekTimekeepingModule
    + crate::validation::ValidationModule
    + multiversx_sc_modules::pause::PauseModule
    + energy_query::EnergyQueryModule
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
        signer: ManagedAddress,
    ) {
        require!(
            self.rewards_info(project_id).is_empty(),
            "Initial rewards already deposited"
        );

        let week_diff = end_week - start_week;
        let min_rewards_period = self.min_rewards_period().get();
        require!(week_diff >= min_rewards_period, "Too few reward weeks");

        let reward_payment = self.deposit_rewards_common(project_id, start_week, end_week);

        let first_week_rdpe = self.get_initial_rdpe();
        require!(
            first_week_rdpe > 0,
            "First week rewards dollar per energy not set"
        );

        self.rewards_dollars_per_energy(project_id, start_week)
            .set(first_week_rdpe);
        self.signer(project_id).set(&signer);
        self.project_active(project_id).set(PROJECT_UNPAUSED);

        self.emit_deposit_initial_rewards_event(
            project_id,
            &DepositInitialRewardsEventData {
                start_week,
                end_week,
                signer,
                reward_payment,
            },
        );
    }

    #[payable("*")]
    #[endpoint(depositAdditionalRewards)]
    fn deposit_additional_rewards(&self, project_id: ProjectId, start_week: Week, end_week: Week) {
        let info_mapper = self.rewards_info(project_id);
        require!(
            !info_mapper.is_empty(),
            "Must deposit initial rewards first"
        );

        let rewards_info = info_mapper.get();
        let current_week = self.get_current_week();
        require!(
            rewards_info.end_week > current_week,
            "Project already ended"
        );
        require!(
            rewards_info.end_week >= start_week,
            INVALID_START_WEEK_ERR_MSG
        );
        require!(
            end_week >= rewards_info.last_update_week,
            "Invalid end week"
        );

        let reward_payment = self.deposit_rewards_common(project_id, start_week, end_week);

        self.emit_deposit_additional_rewards_event(
            project_id,
            start_week,
            end_week,
            reward_payment,
        );
    }

    fn deposit_rewards_common(
        &self,
        project_id: ProjectId,
        start_week: Week,
        end_week: Week,
    ) -> EsdtTokenPayment {
        self.require_not_paused();
        self.require_valid_project_id(project_id);

        let payment = self.call_value().single_esdt();
        require!(payment.token_nonce == 0, "Only fungible tokens accepted");

        let caller = self.blockchain().get_caller();
        self.require_is_project_owner(&caller, project_id);

        require!(start_week < end_week, "Invalid week numbers");

        let current_week = self.get_current_week();
        require!(start_week > current_week, INVALID_START_WEEK_ERR_MSG);

        let week_diff = end_week - start_week;
        let rewards_per_week = &payment.amount / week_diff as u32;
        let dollar_value = self.get_usdc_value(
            payment.token_identifier.clone(),
            rewards_per_week.clone(),
            DAY_IN_SECONDS,
        );
        let min_weekly_rewards_value = self.min_weekly_rewards_value().get();
        require!(dollar_value >= min_weekly_rewards_value, "Too few rewards");

        let info_mapper = self.rewards_info(project_id);
        let mut rewards_info = if !info_mapper.is_empty() {
            let mut rewards_info = info_mapper.get();
            require!(
                payment.token_identifier == rewards_info.reward_token_id,
                "Invalid payment"
            );

            rewards_info.last_update_week =
                core::cmp::min(rewards_info.last_update_week, start_week);
            rewards_info.end_week = core::cmp::max(rewards_info.end_week, end_week);

            if current_week < start_week && start_week < rewards_info.start_week {
                rewards_info.start_week = start_week;
            }

            rewards_info
        } else {
            RewardsInfo::new(payment.token_identifier.clone(), start_week, end_week)
        };

        self.update_rewards(project_id, OptionalValue::None, &mut rewards_info);

        for week in start_week..end_week {
            self.rewards_total_amount(project_id, week)
                .update(|total| *total += &rewards_per_week);
            self.rewards_remaining_amount(project_id, week)
                .update(|remaining| *remaining += &rewards_per_week);
        }

        let total_rewards = &rewards_per_week * week_diff as u32;
        let surplus_amount = payment.amount - &total_rewards;
        let surplus_payment =
            EsdtTokenPayment::new(payment.token_identifier.clone(), 0, surplus_amount);
        self.send()
            .direct_non_zero_esdt_payment(&caller, &surplus_payment);

        EsdtTokenPayment::new(payment.token_identifier, 0, total_rewards)
    }
}
