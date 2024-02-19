use super::week_timekeeping::Week;

use crate::{project::ProjectId, DAY_IN_SECONDS, MAX_PERCENTAGE, PRECISION, WEEK_IN_SECONDS};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait EnergyModule:
    super::common_rewards::CommonRewardsModule
    + crate::price_query::PriceQueryModule
    + crate::project::ProjectsModule
    + super::week_timekeeping::WeekTimekeepingModule
{
    #[only_owner]
    #[endpoint(setMinEnergyPerRewardDollar)]
    fn set_min_energy_per_reward_dollar(&self, min_value: BigUint) {
        self.min_energy_per_reward_dollar().set(min_value);
    }

    #[only_owner]
    #[endpoint(setEnergyPerRewardDollarForWeek)]
    fn set_energy_per_reward_dollar_for_week(&self, project_id: ProjectId, new_min: BigUint) {
        self.require_valid_project_id(project_id);

        let week = self.get_current_week() + 1;
        self.energy_per_reward_dollar_for_week(project_id, week)
            .set(new_min);
    }

    #[only_owner]
    #[endpoint(setAlpha)]
    fn set_alpha(&self, alpha: BigUint) {
        self.alpha().set(alpha);
    }

    #[only_owner]
    #[endpoint(setBeta)]
    fn set_beta(&self, beta: BigUint) {
        self.beta().set(beta);
    }

    fn get_total_energy_for_current_week(&self, project_id: ProjectId) -> BigUint {
        let current_week = self.get_current_week();
        let mapper = self.total_energy_for_week(project_id, current_week);
        if !mapper.is_empty() {
            return mapper.get();
        }

        let rewards_info = self.rewards_info(project_id).get();
        let total_rewards = self.rewards_total_amount(project_id, current_week).get();
        let rewards_value =
            self.get_dollar_value(rewards_info.reward_token_id, total_rewards, DAY_IN_SECONDS);
        let energy_per_rew_dollar = self.get_energy_per_rew_dollar(project_id);
        let total_energy = rewards_value * energy_per_rew_dollar / PRECISION;
        mapper.set(&total_energy);

        total_energy
    }

    fn get_energy_per_rew_dollar(&self, project_id: ProjectId) -> BigUint {
        let current_week = self.get_current_week();
        let mapper = self.energy_per_reward_dollar_for_week(project_id, current_week);
        if !mapper.is_empty() {
            return mapper.get();
        }

        let previous_week = current_week - 1;
        let rew_prev_week = self.rewards_total_amount(project_id, previous_week).get();
        let rew_current_week = self.rewards_total_amount(project_id, current_week).get();
        let min_energy_per_reward_dollar = self.min_energy_per_reward_dollar().get();
        if rew_prev_week == 0 || rew_current_week == 0 {
            return min_energy_per_reward_dollar;
        }

        let rewards_info = self.rewards_info(project_id).get();
        let total_rewards_prev_week = self.rewards_total_amount(project_id, previous_week).get();
        let rewards_value_prev_week = self.get_dollar_value(
            rewards_info.reward_token_id.clone(),
            total_rewards_prev_week,
            WEEK_IN_SECONDS,
        );

        let total_rewards_current_week = self.rewards_total_amount(project_id, current_week).get();
        let rewards_value_current_week = self.get_dollar_value(
            rewards_info.reward_token_id,
            total_rewards_current_week,
            DAY_IN_SECONDS,
        );

        let total_energy_prev_week = self.total_energy_for_week(project_id, previous_week).get();
        let interested_energy = self.get_interested_energy(project_id, previous_week);
        let num = (total_energy_prev_week * interested_energy).sqrt();
        let den = (rewards_value_prev_week * rewards_value_current_week).sqrt();
        let alpha = self.alpha().get();

        let calculated_value = alpha * PRECISION * num / (den * MAX_PERCENTAGE);
        let eprd_for_week = core::cmp::max(calculated_value, min_energy_per_reward_dollar);
        mapper.set(&eprd_for_week);

        eprd_for_week
    }

    fn get_interested_energy(&self, project_id: ProjectId, previous_week: Week) -> BigUint {
        let interested_energy_rewards = self
            .interested_energy_rewards_claimers(project_id, previous_week)
            .get();
        let registered_energy_rewards_claimers = self
            .registered_energy_rewards_claimers(project_id, previous_week)
            .get();
        if registered_energy_rewards_claimers == 0 {
            return interested_energy_rewards;
        }

        let registered_energy_exemption_claimers = self
            .registered_energy_exemption_claimers(project_id, previous_week)
            .get();
        let interested_energy_exemption = registered_energy_exemption_claimers
            * &interested_energy_rewards
            / registered_energy_rewards_claimers;
        let beta = self.beta().get();

        interested_energy_rewards + beta * interested_energy_exemption / MAX_PERCENTAGE
    }

    #[storage_mapper("totalEnergyForWeek")]
    fn total_energy_for_week(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("intEnergyForRewClaimers")]
    fn interested_energy_rewards_claimers(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("regEnergyRewClaimers")]
    fn registered_energy_rewards_claimers(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("regEnergyExempClaimers")]
    fn registered_energy_exemption_claimers(
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

    #[storage_mapper("beta")]
    fn beta(&self) -> SingleValueMapper<BigUint>;
}
