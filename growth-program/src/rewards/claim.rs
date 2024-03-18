use super::{
    claim_types::{
        ClaimExemptionArgs, ClaimRewardsArgs, ClaimType, LockOption, OptClaimArgType,
        NONE_PERCENTAGE, ONE_WEEK_PERCENTAGE, TWO_WEEKS_PERCENTAGE,
    },
    week_timekeeping::{Epoch, Week},
};

use crate::{
    project::ProjectId,
    rewards::{claim_types::CheckSignatureArgs, notes_history::Note},
    validation::SignatureData,
    HOUR_IN_SECONDS, MAX_PERCENTAGE,
};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ClaimRewardsModule:
    super::week_timekeeping::WeekTimekeepingModule
    + crate::price_query::PriceQueryModule
    + crate::project::ProjectsModule
    + super::energy::EnergyModule
    + super::common_rewards::CommonRewardsModule
    + super::notes_history::NotesHistoryModule
    + crate::validation::ValidationModule
    + crate::events::EventsModule
    + energy_query::EnergyQueryModule
    + multiversx_sc_modules::pause::PauseModule
{
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        project_id: ProjectId,
        min_rewards: BigUint,
        claim_type: ClaimType,
        opt_note_and_signature: OptClaimArgType<Self::Api>,
    ) -> OptionalValue<EsdtTokenPayment> {
        self.require_not_paused();
        self.require_valid_project_id(project_id);

        let current_week = self.get_current_week();
        let info_mapper = self.rewards_info(project_id);
        let mut rewards_info = info_mapper.get();
        require!(
            current_week >= rewards_info.start_week,
            "Project not started yet"
        );
        require!(
            current_week < rewards_info.end_week,
            "May not claim rewards for this project anymore"
        );

        let mut claimed_user_mapper = self.user_claimed(project_id, current_week);
        let caller = self.blockchain().get_caller();
        let user_id = self.user_ids().get_id_or_insert(&caller);
        require!(!claimed_user_mapper.contains(&user_id), "Already claimed");

        let check_sig_args = CheckSignatureArgs {
            project_id,
            user_id,
            current_week,
            caller: &caller,
            opt_note_and_signature,
        };
        self.check_signature(check_sig_args);

        self.update_rewards(project_id, OptionalValue::None, &mut rewards_info);

        let _ = claimed_user_mapper.insert(user_id);

        let total_energy = self.get_total_energy_for_current_week(project_id);
        let total_rewards = self.rewards_total_amount(project_id, current_week).get();
        let user_original_energy = self.get_energy_amount(&caller);

        let rew_advertised = if total_energy != 0 {
            total_rewards * &user_original_energy / total_energy
        } else {
            BigUint::zero()
        };
        let opt_rewards = match claim_type {
            ClaimType::Exemption => {
                let claim_exemption_args = ClaimExemptionArgs {
                    project_id,
                    user_id,
                    current_week,
                    rew_advertised,
                    min_rewards,
                    user_original_energy,
                    reward_token_id: rewards_info.reward_token_id.clone(),
                };
                self.claim_exemption(claim_exemption_args);

                OptionalValue::None
            }
            ClaimType::Rewards(lock_option) => {
                let claim_normal_args = ClaimRewardsArgs {
                    project_id,
                    user_id,
                    current_week,
                    rew_advertised,
                    min_rewards,
                    user_original_energy,
                    reward_token_id: rewards_info.reward_token_id.clone(),
                    lock_option,
                    caller: &caller,
                };
                let output_payment = self.claim_rewards_normal(claim_normal_args);

                OptionalValue::Some(output_payment)
            }
        };

        let total_rewards = match &opt_rewards {
            OptionalValue::Some(payment) => payment.amount.clone(),
            OptionalValue::None => BigUint::zero(),
        };

        self.emit_claim_rewards_event(&caller, project_id, total_rewards, claim_type);

        opt_rewards
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
        let user_id = self.user_ids().get_id(&user_address);
        if user_id == NULL_ID {
            return false;
        }

        self.user_claimed(project_id, week).contains(&user_id)
    }

    fn check_signature(&self, args: CheckSignatureArgs<Self::Api>) {
        if self
            .exempted_participants(args.project_id, args.current_week)
            .contains(&args.user_id)
        {
            return;
        }

        self.require_project_active(args.project_id);
        require!(
            args.opt_note_and_signature.is_some(),
            "Must provide note and signature"
        );

        let (note, signature) = unsafe {
            args.opt_note_and_signature
                .into_option()
                .unwrap_unchecked()
                .into_tuple()
        };
        let signature_data = SignatureData {
            caller: args.caller,
            project_id: args.project_id,
            week: args.current_week,
            note: &note,
        };

        self.verify_signature(signature_data, &signature);

        let full_note = Note {
            note_data: note,
            week: args.current_week,
        };
        self.insert_note(args.project_id, args.user_id, &full_note);
    }

    fn claim_exemption(&self, args: ClaimExemptionArgs<Self::Api>) {
        let beta = self.beta().get();
        let remaining_rewards = self
            .rewards_remaining_amount(args.project_id, args.current_week)
            .get();

        require!(
            remaining_rewards < args.rew_advertised,
            "Can claim full rewards"
        );
        require!(args.min_rewards == 0, "Invalid min rewards");

        self.registered_energy_exemption_claimers(args.project_id, args.current_week)
            .update(|reg_energy| *reg_energy += args.user_original_energy);

        let rew_available = beta * args.rew_advertised / MAX_PERCENTAGE;
        let rew_available_dollar_value =
            self.get_usdc_value(args.reward_token_id, rew_available, HOUR_IN_SECONDS);
        self.registered_rewards_dollars(args.project_id, args.current_week)
            .update(|reg_rew_dollars| *reg_rew_dollars += rew_available_dollar_value);

        let _ = self
            .exempted_participants(args.project_id, args.current_week + 1)
            .insert(args.user_id);
    }

    fn claim_rewards_normal(&self, args: ClaimRewardsArgs<Self::Api>) -> EsdtTokenPayment {
        let rem_rewards_mapper = self.rewards_remaining_amount(args.project_id, args.current_week);
        let remaining_rewards = rem_rewards_mapper.get();
        require!(remaining_rewards > 0, "Not enough rewards");

        let rew_available = core::cmp::min(args.rew_advertised, remaining_rewards);
        let user_rewards =
            self.adjust_amount_to_lock_option(rew_available.clone(), args.lock_option);
        require!(user_rewards >= args.min_rewards, "Too few rewards");

        rem_rewards_mapper.update(|rem_rew| *rem_rew -= &user_rewards);

        self.registered_energy_rewards_claimers(args.project_id, args.current_week)
            .update(|reg_energy| *reg_energy += &args.user_original_energy);

        let user_adjusted_energy =
            self.adjust_amount_to_lock_option(args.user_original_energy, args.lock_option);
        self.interested_energy_rewards_claimers(args.project_id, args.current_week)
            .update(|int_energy| *int_energy += &user_adjusted_energy);

        let rew_available_dollar_value =
            self.get_usdc_value(args.reward_token_id.clone(), rew_available, HOUR_IN_SECONDS);
        self.registered_rewards_dollars(args.project_id, args.current_week)
            .update(|reg_rew_dollars| *reg_rew_dollars += rew_available_dollar_value);

        let lock_period = args.lock_option.get_lock_period();
        let unlocked_payment = EsdtTokenPayment::new(args.reward_token_id, 0, user_rewards);
        if lock_period > 0 {
            self.lock_tokens(unlocked_payment, lock_period, args.caller.clone())
        } else {
            self.send()
                .direct_non_zero_esdt_payment(args.caller, &unlocked_payment);

            unlocked_payment
        }
    }

    fn adjust_amount_to_lock_option(&self, amount: BigUint, lock_option: LockOption) -> BigUint {
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
}
