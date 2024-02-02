use week_timekeeping::{Week, FIRST_WEEK};

use crate::{project::ProjectId, MAX_PERCENTAGE, PRECISION};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait EnergyModule:
    super::common_rewards::CommonRewardsModule
    + crate::price_query::PriceQueryModule
    + crate::project::ProjectsModule
    + week_timekeeping::WeekTimekeepingModule
{
    #[only_owner]
    #[endpoint(setMinEnergyPerRewardDollar)]
    fn set_min_energy_per_reward_dollar(&self, min_value: BigUint) {
        self.min_energy_per_reward_dollar().set(min_value);
    }

    #[only_owner]
    #[endpoint(setEnergyPerRewardDollarForWeek)]
    fn set_energy_per_reward_dollar_for_week(
        &self,
        project_id: ProjectId,
        week: Week,
        new_min: BigUint,
    ) {
        self.require_valid_project_id(project_id);

        require!(week >= FIRST_WEEK, "Invalid week");

        self.energy_per_reward_dollar_for_week(project_id, week)
            .set(new_min);
    }

    #[only_owner]
    #[endpoint(setAlpha)]
    fn set_alpha(&self, alpha: BigUint) {
        self.alpha().set(alpha);
    }

    #[only_owner]
    #[endpoint(setTotalEnergyForWeek)]
    fn set_total_energy_for_week(
        &self,
        project_id: ProjectId,
        week: Week,
        total_energy_for_week: BigUint,
    ) {
        self.require_valid_project_id(project_id);

        let current_week = self.get_current_week();
        require!(week > current_week, "Invalid week");

        self.total_energy_for_week(project_id, week)
            .set(&total_energy_for_week);
        self.remaining_energy_for_week(project_id, week)
            .set(total_energy_for_week);
    }

    fn get_total_energy_for_current_week(&self, project_id: ProjectId) -> BigUint {
        let current_week = self.get_current_week();
        let mapper = self.total_energy_for_week(project_id, current_week);
        if !mapper.is_empty() {
            return mapper.get();
        }

        let rewards_info = self.rewards_info(project_id).get();
        let total_rewards = self.rewards_total_amount(project_id, current_week).get();
        let rewards_value = self.get_dollar_value(rewards_info.reward_token_id, total_rewards);
        let energy_per_rew_dollar = self.get_energy_per_rew_dollar(project_id);
        let total_energy = rewards_value * energy_per_rew_dollar / PRECISION;

        mapper.set(&total_energy);
        self.remaining_energy_for_week(project_id, current_week)
            .set(&total_energy);

        total_energy
    }

    fn get_energy_per_rew_dollar(&self, project_id: ProjectId) -> BigUint {
        let current_week = self.get_current_week();
        let mapper = self.energy_per_reward_dollar_for_week(project_id, current_week);
        if !mapper.is_empty() {
            return mapper.get();
        }

        let min_energy_per_reward_dollar = self.min_energy_per_reward_dollar().get();
        if current_week == FIRST_WEEK {
            mapper.set(&min_energy_per_reward_dollar);

            return min_energy_per_reward_dollar;
        }

        let previous_week = current_week - 1;
        let interested_energy = self
            .interested_energy_for_week(project_id, previous_week)
            .get();
        let rewards_info = self.rewards_info(project_id).get();
        let total_rewards = self.rewards_total_amount(project_id, previous_week).get();
        let rewards_value = self.get_dollar_value(rewards_info.reward_token_id, total_rewards);

        let alpha = self.alpha().get();
        let calculated_value =
            alpha * PRECISION * interested_energy / (rewards_value * MAX_PERCENTAGE);

        core::cmp::max(calculated_value, min_energy_per_reward_dollar)
    }

    #[storage_mapper("totalEnergyForWeek")]
    fn total_energy_for_week(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("remainingEnergyForWeek")]
    fn remaining_energy_for_week(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("interestedEnergyForWeek")]
    fn interested_energy_for_week(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("energyPerRDForWeek")]
    fn energy_per_reward_dollar_for_week(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("minEnergyPerRD")]
    fn min_energy_per_reward_dollar(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("alpha")]
    fn alpha(&self) -> SingleValueMapper<BigUint>;
}
