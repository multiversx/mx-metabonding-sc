elrond_wasm::imports!();

use elrond_wasm_modules::transfer_role_proxy::PaymentsVec;

use crate::{
    claim_progress::{ClaimProgressGraceWeeks, ClaimProgressTracker, ShiftingClaimProgress},
    project::{Project, ProjectId},
    rewards::{RewardsCheckpoint, Week, FIRST_WEEK},
    validation::{Signature, ALREADY_CLAIMED_ERR_MSG},
};

const MAX_CLAIM_ARG_PAIRS: usize = 5;
const CLAIM_NR_ARGS_PER_PAIR: usize = 4;

pub type ClaimArgPair<M> = MultiValue4<Week, BigUint<M>, BigUint<M>, Signature<M>>;
pub type ClaimArgArray<M> = ArrayVec<ClaimArgsWrapper<M>, MAX_CLAIM_ARG_PAIRS>;
pub type SignedClaimArgArray<M> = ArrayVec<SignedClaimArgs<M>, MAX_CLAIM_ARG_PAIRS>;

pub struct ClaimArgsWrapper<M: ManagedTypeApi> {
    pub week: Week,
    pub user_delegation_amount: BigUint<M>,
    pub user_lkmex_staked_amount: BigUint<M>,
    pub checkpoint: RewardsCheckpoint<M>,
}

pub struct SignedClaimArgs<M: ManagedTypeApi> {
    pub args_wrapper: ClaimArgsWrapper<M>,
    pub signature: Signature<M>,
}

#[elrond_wasm::module]
pub trait ClaimModule:
    elrond_wasm_modules::pause::PauseModule
    + crate::project::ProjectModule
    + crate::access_control::AccessControlModule
    + crate::common_storage::CommonStorageModule
    + crate::math::MathModule
    + crate::validation::ValidationModule
    + crate::rewards::RewardsModule
    + crate::claim_progress::ClaimProgressModule
    + sc_whitelist_module::SCWhitelistModule
{
    /// Claims rewards for the given user.
    /// May only be different from caller for whitelisted proxy contracts.
    /// If the user performs their own claim, this address should be their own.
    ///
    /// Claims rewards for the given weeks. Maximum of MAX_CLAIM_ARG_PAIRS weeks can be claimed per call.
    /// Arguments are pairs of:
    /// week: number,
    /// user_delegation_amount: BigUint,
    /// user_lkmex_staked_amount: BigUint,
    /// signature: 120 bytes
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        original_caller: ManagedAddress,
        raw_claim_args: MultiValueEncoded<ClaimArgPair<Self::Api>>,
    ) -> ManagedVec<EsdtTokenPayment> {
        require!(self.not_paused(), "May not claim rewards while paused");

        let caller = self.blockchain().get_caller();
        if caller != original_caller {
            self.require_sc_address_whitelisted(&caller);
        }

        let current_week = self.get_current_week();
        let last_checkpoint_week = self.get_last_checkpoint_week();

        let rewards_nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();
        let mut grace_weeks_progress =
            self.get_grace_weeks_progress(&caller, rewards_nr_first_grace_weeks, current_week);
        let mut shifting_progress = self.get_shifting_progress(&caller, current_week);

        let signed_args = self.collect_claim_args(raw_claim_args);
        let args = self.validate_claim_args(
            &caller,
            signed_args,
            &grace_weeks_progress,
            &shifting_progress,
            last_checkpoint_week,
        );

        let rewards = self.claim_all_project_rewards(
            current_week,
            &args,
            &mut grace_weeks_progress,
            &mut shifting_progress,
        );

        self.claim_progress_grace_weeks(&caller)
            .set(grace_weeks_progress);
        self.shifting_claim_progress(&caller).set(shifting_progress);

        if !rewards.is_empty() {
            self.send().direct_multi(&caller, &rewards);
        }

        rewards
    }

    fn collect_claim_args(
        &self,
        raw_claim_args: MultiValueEncoded<ClaimArgPair<Self::Api>>,
    ) -> SignedClaimArgArray<Self::Api> {
        require!(
            raw_claim_args.raw_len() / CLAIM_NR_ARGS_PER_PAIR <= MAX_CLAIM_ARG_PAIRS,
            "Too many arguments"
        );

        let mut array = ArrayVec::new();
        for raw_arg in raw_claim_args {
            let (week, user_delegation_amount, user_lkmex_staked_amount, signature) =
                raw_arg.into_tuple();
            let checkpoint = self
                .rewards_checkpoints()
                .get_or_else(week, RewardsCheckpoint::default);

            let args_wrapper = ClaimArgsWrapper {
                week,
                user_delegation_amount,
                user_lkmex_staked_amount,
                checkpoint,
            };
            let arg = SignedClaimArgs {
                args_wrapper,
                signature,
            };

            unsafe {
                array.push_unchecked(arg);
            }
        }

        array
    }

    fn claim_all_project_rewards(
        &self,
        current_week: Week,
        claim_args: &ClaimArgArray<Self::Api>,
        grace_weeks_progress: &mut ClaimProgressGraceWeeks<Self::Api>,
        shifting_progress: &mut ShiftingClaimProgress,
    ) -> PaymentsVec<Self::Api> {
        let mut all_rewards = PaymentsVec::new();
        for (id, project) in self.projects().iter() {
            let opt_rewards = self.claim_for_project(current_week, &id, project, claim_args);
            if let Some(rewards) = opt_rewards {
                all_rewards.push(rewards);
            }
        }

        for arg in claim_args {
            // have to check again to prevent duplicate claims
            let can_claim_grace_week = grace_weeks_progress.can_claim_for_week(arg.week);
            let can_claim_shifting_week = shifting_progress.can_claim_for_week(arg.week);
            require!(
                can_claim_grace_week || can_claim_shifting_week,
                ALREADY_CLAIMED_ERR_MSG
            );

            if grace_weeks_progress.is_week_valid(arg.week) {
                grace_weeks_progress.set_claimed_for_week(arg.week);
            }

            shifting_progress.set_claimed_for_week(arg.week);
        }

        all_rewards
    }

    fn claim_for_project(
        &self,
        current_week: Week,
        project_id: &ProjectId<Self::Api>,
        project: Project<Self::Api>,
        claim_args: &ClaimArgArray<Self::Api>,
    ) -> Option<EsdtTokenPayment> {
        let mut rewards_for_project = BigUint::zero();
        for arg in claim_args {
            let opt_weekly_reward =
                self.get_weekly_reward_for_project(project_id, &project, current_week, arg);
            if let Some(weekly_reward) = opt_weekly_reward {
                rewards_for_project += weekly_reward;
            }
        }

        if rewards_for_project == 0 {
            return None;
        }

        self.leftover_project_funds(project_id)
            .update(|leftover| *leftover -= &rewards_for_project);

        let reward_payment = EsdtTokenPayment::new(project.reward_token, 0, rewards_for_project);
        Some(reward_payment)
    }

    #[view(getUserClaimableWeeks)]
    fn get_user_claimable_weeks(&self, user: ManagedAddress) -> MultiValueEncoded<Week> {
        let current_week = self.get_current_week();
        let last_checkpoint_week = self.get_last_checkpoint_week();
        if current_week == 0 || last_checkpoint_week == 0 {
            return MultiValueEncoded::new();
        }

        let rewards_nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();
        let claimable_weeks = if current_week <= rewards_nr_first_grace_weeks {
            self.get_claimable_grace_weeks(&user, rewards_nr_first_grace_weeks, current_week)
        } else {
            self.get_claimable_shifting_weeks(&user, current_week)
        };

        claimable_weeks.into()
    }

    fn get_claimable_grace_weeks(
        &self,
        user: &ManagedAddress,
        grace_weeks: Week,
        current_week: Week,
    ) -> ManagedVec<Week> {
        let grace_weeks_progress = self.get_grace_weeks_progress(user, grace_weeks, current_week);
        let last_checkpoint_week = self.get_last_checkpoint_week();

        self.get_claimable_weeks(&grace_weeks_progress, FIRST_WEEK, last_checkpoint_week)
    }

    fn get_claimable_shifting_weeks(
        &self,
        user: &ManagedAddress,
        current_week: Week,
    ) -> ManagedVec<Week> {
        let shifting_progress = self.get_shifting_progress(user, current_week);
        let start_week =
            ShiftingClaimProgress::get_first_index_week_for_new_current_week(current_week);
        let last_checkpoint_week = self.get_last_checkpoint_week();

        self.get_claimable_weeks(&shifting_progress, start_week, last_checkpoint_week)
    }
}
