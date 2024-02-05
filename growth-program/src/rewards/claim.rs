use week_timekeeping::{Epoch, Week, EPOCHS_IN_WEEK};

use crate::{project::ProjectId, validation::Signature, MAX_PERCENTAGE};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub enum LockOption {
    None,
    OneWeek,
    TwoWeeks,
}

impl LockOption {
    pub fn get_lock_period(&self) -> Epoch {
        match *self {
            LockOption::None => NO_LOCK_PERIOD,
            LockOption::OneWeek => ONE_WEEK_LOCK_PERIOD,
            LockOption::TwoWeeks => TWO_WEEKS_LOCK_PERIOD,
        }
    }
}

pub const NONE_PERCENTAGE: u32 = 25 * MAX_PERCENTAGE / 100; // 25%
pub const ONE_WEEK_PERCENTAGE: u32 = 50 * MAX_PERCENTAGE / 100; // 50%
pub const TWO_WEEKS_PERCENTAGE: u32 = 100 * MAX_PERCENTAGE / 100; // 100%

pub const NO_LOCK_PERIOD: Epoch = 0;
pub const ONE_WEEK_LOCK_PERIOD: Epoch = EPOCHS_IN_WEEK;
pub const TWO_WEEKS_LOCK_PERIOD: Epoch = 2 * EPOCHS_IN_WEEK;

#[multiversx_sc::module]
pub trait ClaimRewardsModule:
    week_timekeeping::WeekTimekeepingModule
    + crate::price_query::PriceQueryModule
    + crate::project::ProjectsModule
    + super::energy::EnergyModule
    + super::common_rewards::CommonRewardsModule
    + crate::validation::ValidationModule
    + energy_query::EnergyQueryModule
{
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        project_id: ProjectId,
        lock_option: LockOption,
        min_rewards: BigUint,
        signature: Signature<Self::Api>,
    ) -> OptionalValue<EsdtTokenPayment> {
        self.require_valid_project_id(project_id);

        let current_week = self.get_current_week();
        let info_mapper = self.rewards_info(project_id);
        let mut rewards_info = info_mapper.get();
        require!(
            current_week < rewards_info.end_week,
            "May not claim rewards for this project anymore"
        );

        let mut claimed_user_mapper = self.user_claimed(project_id, current_week);
        let caller = self.blockchain().get_caller();
        let user_id = self.user_ids().get_id_or_insert(&caller);
        require!(!claimed_user_mapper.contains(&user_id), "Already claimed");

        self.verify_signature(&caller, project_id, current_week, &signature);
        self.update_rewards(project_id, OptionalValue::None, &mut rewards_info);

        let total_energy = self.get_total_energy_for_current_week(project_id);
        let user_energy = self.get_energy_amount(&caller);
        self.interested_energy_for_week(project_id, current_week)
            .update(|interested_energy| *interested_energy += &user_energy);

        claimed_user_mapper.insert(user_id);

        let remaining_energy_mapper = self.remaining_energy_for_week(project_id, current_week);
        let remaining_energy = remaining_energy_mapper.get();
        if remaining_energy == 0 {
            require!(min_rewards == 0, "Invalid min rewards");

            let _ = self
                .exempted_participants(project_id, current_week + 1)
                .insert(user_id);

            return OptionalValue::None;
        }

        let user_remaining_energy = core::cmp::min(remaining_energy.clone(), user_energy);
        let new_remaining_energy = &remaining_energy - &user_remaining_energy;
        remaining_energy_mapper.set(new_remaining_energy);

        let total_rewards = self.rewards_total_amount(project_id, current_week).get();
        let total_user_rewards = total_rewards * user_remaining_energy / total_energy;
        let lock_period = lock_option.get_lock_period();
        let final_user_rewards =
            self.adjust_rewards_to_lock_option(total_user_rewards, lock_option);

        require!(final_user_rewards >= min_rewards, "Too few rewards");

        self.rewards_remaining_amount(project_id, current_week)
            .update(|rem_rew| *rem_rew -= &final_user_rewards);

        let unlocked_payment =
            EsdtTokenPayment::new(rewards_info.reward_token_id.clone(), 0, final_user_rewards);
        let output_payment = if lock_period > 0 {
            self.lock_tokens(unlocked_payment, lock_period, caller)
        } else {
            self.send()
                .direct_non_zero_esdt_payment(&caller, &unlocked_payment);

            unlocked_payment
        };

        info_mapper.set(rewards_info);

        OptionalValue::Some(output_payment)
    }

    fn adjust_rewards_to_lock_option(&self, amount: BigUint, lock_option: LockOption) -> BigUint {
        match lock_option {
            LockOption::None => amount * NONE_PERCENTAGE / MAX_PERCENTAGE,
            LockOption::OneWeek => amount * ONE_WEEK_PERCENTAGE / MAX_PERCENTAGE,
            LockOption::TwoWeeks => amount * TWO_WEEKS_PERCENTAGE / MAX_PERCENTAGE,
        }
    }

    fn lock_tokens(
        &self,
        payment: EsdtTokenPayment,
        lock_epochs: Epoch,
        user_address: ManagedAddress,
    ) -> EsdtTokenPayment {
        if payment.amount == 0 {
            return payment;
        }

        let current_epoch = self.blockchain().get_block_epoch();
        let unlock_epoch = current_epoch + lock_epochs;
        let simple_lock_address = self.simple_lock_address().get();
        let output_payment: EgldOrEsdtTokenPayment = self
            .simple_lock_proxy(simple_lock_address)
            .lock_tokens_endpoint(unlock_epoch, OptionalValue::Some(user_address))
            .with_esdt_transfer(payment)
            .execute_on_dest_context();

        EsdtTokenPayment::new(
            output_payment.token_identifier.unwrap_esdt(),
            output_payment.token_nonce,
            output_payment.amount,
        )
    }

    #[proxy]
    fn simple_lock_proxy(&self, sc_address: ManagedAddress) -> simple_lock::Proxy<Self::Api>;

    #[storage_mapper("simpleLockAddress")]
    fn simple_lock_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("exemptedParticipants")]
    fn exempted_participants(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("userClaimed")]
    fn user_claimed(&self, project_id: ProjectId, week: Week) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("userIds")]
    fn user_ids(&self) -> AddressToIdMapper<Self::Api>;
}
