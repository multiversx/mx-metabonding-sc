use week_timekeeping::{Epoch, Week, EPOCHS_IN_WEEK};

use crate::{project::ProjectId, validation::Signature, MAX_PERCENTAGE};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy)]
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
    + multiversx_sc_modules::pause::PauseModule
{
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        project_id: ProjectId,
        min_rewards: BigUint,
        signature: Signature<Self::Api>,
        opt_lock_option: OptionalValue<LockOption>,
    ) -> OptionalValue<EsdtTokenPayment> {
        self.require_not_paused();
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

        claimed_user_mapper.insert(user_id);

        let user_original_energy = self.get_energy_amount(&caller);
        let rem_rewards_mapper = self.rewards_remaining_amount(project_id, current_week);
        let remaining_rewards = rem_rewards_mapper.get();
        if remaining_rewards == 0 {
            require!(min_rewards == 0, "Invalid min rewards");

            self.registered_energy_exemption_claimers(project_id, current_week)
                .update(|reg_energy| *reg_energy += user_original_energy);
            let _ = self
                .exempted_participants(project_id, current_week + 1)
                .insert(user_id);

            return OptionalValue::None;
        }

        require!(opt_lock_option.is_some(), "No lock option provided");

        let lock_option = unsafe { opt_lock_option.into_option().unwrap_unchecked() };
        let user_adjusted_energy =
            self.adjust_energy_to_lock_option(user_original_energy.clone(), lock_option);
        self.registered_energy_rewards_claimers(project_id, current_week)
            .update(|reg_energy| *reg_energy += user_original_energy);
        self.interested_energy_rewards_claimers(project_id, current_week)
            .update(|int_energy| *int_energy += &user_adjusted_energy);

        let total_rewards = self.rewards_total_amount(project_id, current_week).get();
        let total_energy = self.get_total_energy_for_current_week(project_id);
        let max_rewards = total_rewards * user_adjusted_energy / total_energy;
        let user_rewards = core::cmp::min(max_rewards, remaining_rewards);
        require!(user_rewards >= min_rewards, "Too few rewards");

        rem_rewards_mapper.update(|rem_rew| *rem_rew -= &user_rewards);

        let lock_period = lock_option.get_lock_period();
        let unlocked_payment =
            EsdtTokenPayment::new(rewards_info.reward_token_id.clone(), 0, user_rewards);
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

    #[view(getExemptedParticipants)]
    fn get_exempted_participants(
        &self,
        project_id: ProjectId,
        week: Week,
    ) -> MultiValueEncoded<ManagedAddress> {
        let id_mapper = self.user_ids();
        let mut results = MultiValueEncoded::new();
        for user_id in self.exempted_participants(project_id, week).iter() {
            let opt_user_address = id_mapper.get_address(user_id);
            let user_address = unsafe { opt_user_address.unwrap_unchecked() };
            results.push(user_address);
        }

        results
    }

    #[view(getUserClaimed)]
    fn get_user_claimed(
        &self,
        user_address: ManagedAddress,
        project_id: ProjectId,
        week: Week,
    ) -> bool {
        let user_id = self.user_ids().get_id_non_zero(&user_address);
        self.user_claimed(project_id, week).contains(&user_id)
    }

    fn adjust_energy_to_lock_option(&self, amount: BigUint, lock_option: LockOption) -> BigUint {
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
