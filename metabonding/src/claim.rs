elrond_wasm::imports!();

use elrond_wasm_modules::transfer_role_proxy::PaymentsVec;

use crate::{
    claim_progress::ClaimProgressTracker,
    project::PROJECT_EXPIRATION_WEEKS,
    rewards::{RewardsCheckpoint, Week, FIRST_WEEK},
    validation::Signature,
};

const MAX_CLAIM_ARG_PAIRS: usize = 5;
const CLAIM_NR_ARGS_PER_PAIR: usize = 4;
static ALREADY_CLAIMED_ERR_MSG: &[u8] = b"Already claimed rewards for this week";
static INVALID_WEEK_NR_ERR_MSG: &[u8] = b"Invalid week number";

pub type ClaimArgPair<M> = MultiValue4<Week, BigUint<M>, BigUint<M>, Signature<M>>;
pub type ClaimArgArray<M> = ArrayVec<ClaimArgsWrapper<M>, MAX_CLAIM_ARG_PAIRS>;

pub struct ClaimArgsWrapper<M: ManagedTypeApi> {
    pub week: Week,
    pub user_delegation_amount: BigUint<M>,
    pub user_lkmex_staked_amount: BigUint<M>,
    pub checkpoint: RewardsCheckpoint<M>,
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
{
    /// Claims rewards for the given weeks. Maximum of MAX_CLAIM_ARG_PAIRS weeks can be claimed per call.
    /// Arguments are pairs of:
    /// week: number,
    /// user_delegation_amount: BigUint,
    /// user_lkmex_staked_amount: BigUint,
    /// signature: 120 bytes
    #[endpoint(claimRewards)]
    fn claim_rewards(&self, raw_claim_args: MultiValueEncoded<ClaimArgPair<Self::Api>>) {
        require!(self.not_paused(), "May not claim rewards while paused");
        require!(
            raw_claim_args.raw_len() / CLAIM_NR_ARGS_PER_PAIR <= MAX_CLAIM_ARG_PAIRS,
            "Too many arguments"
        );

        let caller = self.blockchain().get_caller();
        let current_week = self.get_current_week();
        let args = self.validate_and_collect_claim_args(&caller, current_week, raw_claim_args);
        let rewards = self.claim_all_from_args(current_week, args);
        if !rewards.is_empty() {
            self.send().direct_multi(&caller, &rewards);
        }
    }

    fn validate_and_collect_claim_args(
        &self,
        caller: &ManagedAddress,
        current_week: Week,
        raw_args: MultiValueEncoded<ClaimArgPair<Self::Api>>,
    ) -> ClaimArgArray<Self::Api> {
        let last_checkpoint_week = self.get_last_checkpoint_week();
        let nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();
        let is_grace_period = current_week <= nr_first_grace_weeks;

        let mut grace_weeks_progress = self.get_grace_weeks_progress(caller);
        let mut shifting_progress = self.get_shifting_progress(caller, current_week);
        let mut args = ClaimArgArray::new();
        for raw_arg in raw_args {
            let (week, user_delegation_amount, user_lkmex_staked_amount, signature) =
                raw_arg.into_tuple();

            require!(
                week >= FIRST_WEEK && week <= last_checkpoint_week,
                INVALID_WEEK_NR_ERR_MSG
            );

            if is_grace_period && grace_weeks_progress.is_week_valid(week) {
                require!(
                    grace_weeks_progress.can_claim_for_week(week),
                    ALREADY_CLAIMED_ERR_MSG
                );

                grace_weeks_progress.set_claimed_for_week(week);
            } else if shifting_progress.is_week_valid(week) {
                require!(
                    shifting_progress.can_claim_for_week(week),
                    ALREADY_CLAIMED_ERR_MSG
                );

                shifting_progress.set_claimed_for_week(week);
            } else {
                sc_panic!(INVALID_WEEK_NR_ERR_MSG);
            }

            let checkpoint = self.rewards_checkpoints().get(week);
            self.verify_signature(
                week,
                caller,
                &user_delegation_amount,
                &user_lkmex_staked_amount,
                &signature,
            );

            args.push(ClaimArgsWrapper {
                week,
                user_delegation_amount,
                user_lkmex_staked_amount,
                checkpoint,
            });
        }

        self.claim_progress_grace_weeks(caller)
            .set(&grace_weeks_progress);
        self.shifting_claim_progress(caller).set(&shifting_progress);

        args
    }

    fn claim_all_from_args(
        &self,
        current_week: Week,
        claim_args: ClaimArgArray<Self::Api>,
    ) -> PaymentsVec<Self::Api> {
        let mut all_rewards = PaymentsVec::new();

        let projects_mapper = self.projects();
        for (id, project) in projects_mapper.iter() {
            let mut rewards_for_project = BigUint::zero();
            for arg in &claim_args {
                let opt_weekly_reward = self.get_weekly_reward_for_project(
                    &id,
                    &project,
                    current_week,
                    arg.week,
                    &arg.user_delegation_amount,
                    &arg.user_lkmex_staked_amount,
                    &arg.checkpoint.total_delegation_supply,
                    &arg.checkpoint.total_lkmex_staked,
                );
                if let Some(weekly_reward) = opt_weekly_reward {
                    rewards_for_project += weekly_reward;
                }
            }

            if rewards_for_project > 0 {
                self.leftover_project_funds(&id)
                    .update(|leftover| *leftover -= &rewards_for_project);

                all_rewards.push(EsdtTokenPayment::new(
                    project.reward_token,
                    0,
                    rewards_for_project,
                ));
            }
        }

        all_rewards
    }

    #[view(getUserClaimableWeeks)]
    fn get_user_claimable_weeks(&self, user: ManagedAddress) -> MultiValueEncoded<Week> {
        let mut weeks_list = MultiValueEncoded::new();
        let current_week = self.get_current_week();
        let last_checkpoint_week = self.get_last_checkpoint_week();
        if current_week == 0 || last_checkpoint_week == 0 {
            return weeks_list;
        }

        let rewards_nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();
        if current_week <= rewards_nr_first_grace_weeks {
            let grace_weeks_progress = self.get_grace_weeks_progress(&user);
            for week in FIRST_WEEK..=last_checkpoint_week {
                if grace_weeks_progress.can_claim_for_week(week) {
                    weeks_list.push(week);
                }
            }
        } else {
            let shifting_progress = self.get_shifting_progress(&user, current_week);
            let start_week = if current_week > PROJECT_EXPIRATION_WEEKS {
                current_week - PROJECT_EXPIRATION_WEEKS
            } else {
                FIRST_WEEK
            };
            for week in start_week..=last_checkpoint_week {
                if shifting_progress.can_claim_for_week(week) {
                    weeks_list.push(week);
                }
            }
        }

        weeks_list
    }
}
