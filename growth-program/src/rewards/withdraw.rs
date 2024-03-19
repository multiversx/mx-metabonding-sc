use super::week_timekeeping::Week;

use crate::{project::ProjectId, rewards::deposit::INVALID_START_WEEK_ERR_MSG};

multiversx_sc::imports!();

mod fees_collector_proxy {
    #[multiversx_sc::proxy]
    pub trait FeesCollectorProxy {
        #[payable("*")]
        #[endpoint(depositSwapFees)]
        fn deposit_swap_fees(&self);
    }
}

#[multiversx_sc::module]
pub trait WithdrawRewardsModule:
    super::common_rewards::CommonRewardsModule
    + crate::project::ProjectsModule
    + super::week_timekeeping::WeekTimekeepingModule
    + multiversx_sc_modules::pause::PauseModule
{
    #[only_owner]
    #[endpoint(setFeesCollectorAddress)]
    fn set_fees_collector_address(&self, fees_collector_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&fees_collector_address),
            "Invalid fees collector address"
        );

        self.fees_collector_address().set(fees_collector_address);
    }

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
            start_week >= rewards_info.last_update_week,
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

        let sc_owner = self.blockchain().get_owner_address();
        let payment = EsdtTokenPayment::new(rewards_info.reward_token_id.clone(), 0, total_amount);
        self.send()
            .direct_non_zero_esdt_payment(&sc_owner, &payment);

        if start_week == rewards_info.last_update_week {
            info_mapper.clear();
        } else {
            rewards_info.end_week = start_week;
            info_mapper.set(rewards_info);
        }
    }

    #[endpoint(finishProgram)]
    fn finish_program(&self, project_id: ProjectId) {
        self.require_not_paused();

        let rewards_mapper = self.rewards_info(project_id);
        let mut rewards_info = rewards_mapper.get();
        let current_week = self.get_current_week();
        require!(
            current_week >= rewards_info.end_week,
            "End week not reached"
        );

        self.update_rewards(project_id, OptionalValue::None, &mut rewards_info);
        rewards_mapper.clear();

        if rewards_info.undistributed_rewards == 0 {
            return;
        }

        let remaining_rewards = EsdtTokenPayment::new(
            rewards_info.reward_token_id,
            0,
            rewards_info.undistributed_rewards,
        );

        // comment this to run the "start_program_again_after_end" test
        let fees_collector_address = self.fees_collector_address().get();
        let _: IgnoreValue = self
            .fees_collector_proxy(fees_collector_address)
            .deposit_swap_fees()
            .with_esdt_transfer(remaining_rewards)
            .execute_on_dest_context();
    }

    #[proxy]
    fn fees_collector_proxy(
        &self,
        sc_address: ManagedAddress,
    ) -> fees_collector_proxy::Proxy<Self::Api>;

    #[storage_mapper("feesCollectorAddress")]
    fn fees_collector_address(&self) -> SingleValueMapper<ManagedAddress>;
}
