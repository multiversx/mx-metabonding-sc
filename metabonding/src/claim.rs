multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc_modules::transfer_role_proxy::PaymentsVec;

use crate::{
    claim_progress::{ClaimFlag, ClaimProgressTracker, ShiftingClaimProgress},
    project::{ProjIdsVec, Project, ProjectId},
    rewards::{RewardsCheckpoint, Week},
    validation::Signature,
};

pub static NO_CLAIM_ARGS_ERR_MSG: &[u8] = b"No claim args";

const MAX_CLAIM_ARG_PAIRS: usize = 5;
const CLAIM_NR_ARGS_PER_PAIR: usize = 4;

pub type ClaimArgPair<M> = MultiValue4<Week, BigUint<M>, BigUint<M>, Signature<M>>;
pub type ClaimArgArray<M> = ArrayVec<ClaimArgsWrapper<M>, MAX_CLAIM_ARG_PAIRS>;
pub type FlagsArray<M> = ArrayVec<ClaimFlag<M>, MAX_CLAIM_ARG_PAIRS>;

#[derive(TypeAbi, TopEncode)]
pub enum ClaimableTokens<M: ManagedTypeApi> {
    All,
    Partial { unclaimed_projects: ProjIdsVec<M> },
}

pub struct ClaimArgsWrapper<M: ManagedTypeApi> {
    pub week: Week,
    pub user_delegation_amount: BigUint<M>,
    pub user_lkmex_staked_amount: BigUint<M>,
    pub checkpoint: RewardsCheckpoint<M>,
    pub signature: Signature<M>,
}

#[multiversx_sc::module]
pub trait ClaimModule:
    multiversx_sc_modules::pause::PauseModule
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
        let mut claim_progress = self.get_claim_progress(&original_caller, current_week);

        let last_checkpoint_week = self.get_last_checkpoint_week();
        let mut args = self.collect_claim_args(raw_claim_args);
        self.sort_claim_args(&mut args);
        self.validate_claim_args(
            &original_caller,
            &args,
            &claim_progress,
            last_checkpoint_week,
        );

        let all_projects = self.get_all_project_ids();
        let rewards = self.claim_all_project_rewards(
            current_week,
            &args,
            &mut claim_progress,
            &all_projects,
            &all_projects,
        );
        self.claim_progress(&original_caller).set(claim_progress);

        if !rewards.is_empty() {
            self.send().direct_multi(&caller, &rewards);
        }

        rewards
    }

    fn sort_claim_args(&self, claim_args: &mut ClaimArgArray<Self::Api>) {
        claim_args.sort_unstable_by(|a, b| a.week.cmp(&b.week));
    }

    fn collect_claim_args(
        &self,
        raw_claim_args: MultiValueEncoded<ClaimArgPair<Self::Api>>,
    ) -> ClaimArgArray<Self::Api> {
        require!(!raw_claim_args.is_empty(), NO_CLAIM_ARGS_ERR_MSG);
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

            let arg = ClaimArgsWrapper {
                week,
                user_delegation_amount,
                user_lkmex_staked_amount,
                checkpoint,
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
        claim_progress: &mut ShiftingClaimProgress<Self::Api>,
        projects_to_claim: &ProjIdsVec<Self::Api>,
        all_projects: &ProjIdsVec<Self::Api>,
    ) -> PaymentsVec<Self::Api> {
        for arg in claim_args {
            let flags_for_week = claim_progress.get_claim_flags_for_week(arg.week);
            if matches!(flags_for_week, ClaimFlag::NotClaimed) {
                claim_progress.set_claimed_for_week(arg.week, all_projects.clone())
            }
        }

        let mut all_rewards = PaymentsVec::new();
        let projects_mapper = self.projects();
        for id in projects_to_claim {
            let opt_project = projects_mapper.get(&id);
            if opt_project.is_none() {
                continue;
            }

            let project = unsafe { opt_project.unwrap_unchecked() };
            let opt_rewards =
                self.claim_for_project(current_week, &id, project, claim_args, claim_progress);
            if let Some(rewards) = opt_rewards {
                all_rewards.push(rewards);
            }
        }

        all_rewards
    }

    fn claim_for_project(
        &self,
        current_week: Week,
        project_id: &ProjectId<Self::Api>,
        project: Project<Self::Api>,
        claim_args: &ClaimArgArray<Self::Api>,
        claim_progress: &mut ShiftingClaimProgress<Self::Api>,
    ) -> Option<EsdtTokenPayment> {
        let mut rewards_for_project = BigUint::zero();
        for arg in claim_args {
            let flags_mut = claim_progress.get_mut_claim_flags_for_week(arg.week);
            let unclaimed_proj_ref = flags_mut.get_mut_unclaimed_proj();
            let opt_index = unclaimed_proj_ref.find(project_id);
            if opt_index.is_none() {
                continue;
            }

            let opt_weekly_reward =
                self.get_weekly_reward_for_project(project_id, &project, current_week, arg);
            if let Some(weekly_reward) = opt_weekly_reward {
                rewards_for_project += weekly_reward;
            }

            let proj_index = unsafe { opt_index.unwrap_unchecked() };
            unclaimed_proj_ref.remove(proj_index);
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
    fn get_user_claimable_weeks(
        &self,
        user: ManagedAddress,
    ) -> MultiValueEncoded<MultiValue2<Week, ClaimableTokens<Self::Api>>> {
        let current_week = self.get_current_week();
        let last_checkpoint_week = self.get_last_checkpoint_week();
        if current_week == 0 || last_checkpoint_week == 0 {
            return MultiValueEncoded::new();
        }

        let claim_progress = self.get_claim_progress(&user, current_week);
        let start_week =
            ShiftingClaimProgress::<Self::Api>::get_first_index_week_for_new_current_week(
                current_week,
            );
        let last_checkpoint_week = self.get_last_checkpoint_week();

        let mut claimable_weeks = MultiValueEncoded::new();
        for week in start_week..=last_checkpoint_week {
            let claim_flags = claim_progress.get_claim_flags_for_week(week);
            match claim_flags {
                ClaimFlag::NotClaimed => claimable_weeks.push((week, ClaimableTokens::All).into()),
                ClaimFlag::Claimed { unclaimed_projects } => {
                    let partial = ClaimableTokens::Partial {
                        unclaimed_projects: unclaimed_projects.clone(),
                    };
                    claimable_weeks.push((week, partial).into());
                }
            };
        }

        claimable_weeks
    }
}
