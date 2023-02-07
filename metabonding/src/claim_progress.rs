multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    project::{ProjIdsVec, PROJECT_EXPIRATION_WEEKS},
    rewards::{Week, FIRST_WEEK},
    validation::INVALID_WEEK_NR_ERR_MSG,
};

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, PartialEq, Debug)]
pub enum ClaimFlag<M: ManagedTypeApi> {
    NotClaimed,
    Claimed { unclaimed_projects: ProjIdsVec<M> },
}

impl<M: ManagedTypeApi> ClaimFlag<M> {
    pub fn from_old_flag(old_flag: bool) -> Self {
        if old_flag {
            ClaimFlag::Claimed {
                unclaimed_projects: ManagedVec::new(),
            }
        } else {
            ClaimFlag::NotClaimed
        }
    }

    pub fn get_mut_unclaimed_proj(&mut self) -> &mut ProjIdsVec<M> {
        match self {
            ClaimFlag::NotClaimed => M::error_api_impl().signal_error(b"Invalid flags state"),
            ClaimFlag::Claimed { unclaimed_projects } => unclaimed_projects,
        }
    }
}

const CLAIM_FLAGS_LEN: usize = PROJECT_EXPIRATION_WEEKS + 1;
type ClaimFlagsArray<M> = ArrayVec<ClaimFlag<M>, CLAIM_FLAGS_LEN>;

fn default_claim_flags<M: ManagedTypeApi>() -> ClaimFlagsArray<M> {
    let mut array = ArrayVec::new();
    array.fill(ClaimFlag::NotClaimed);

    array
}

pub trait ClaimProgressTracker<M: ManagedTypeApi> {
    fn is_week_valid(&self, week: Week) -> bool;

    fn get_claim_flags_for_week(&self, week: Week) -> &ClaimFlag<M>;

    fn get_mut_claim_flags_for_week(&mut self, week: Week) -> &mut ClaimFlag<M>;

    fn set_claimed_for_week(&mut self, week: Week, unclaimed_projects: ProjIdsVec<M>);
}

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct ShiftingClaimProgress<M: ManagedTypeApi> {
    claim_flags: ClaimFlagsArray<M>,
    first_index_week: Week,
}

impl<M: ManagedTypeApi> ShiftingClaimProgress<M> {
    pub fn new(claim_flags: ClaimFlagsArray<M>, current_week: Week) -> Self {
        Self {
            claim_flags,
            first_index_week: Self::get_first_index_week_for_new_current_week(current_week),
        }
    }

    pub fn get_first_index_week_for_new_current_week(current_week: Week) -> Week {
        if current_week > CLAIM_FLAGS_LEN {
            current_week - CLAIM_FLAGS_LEN + 1
        } else {
            FIRST_WEEK
        }
    }

    #[inline]
    fn get_index_for_week(&self, week: Week) -> usize {
        week - self.first_index_week
    }

    fn shift_if_needed(&mut self, current_week: Week) {
        if current_week <= CLAIM_FLAGS_LEN {
            return;
        }

        let new_first_week = Self::get_first_index_week_for_new_current_week(current_week);
        if self.first_index_week == new_first_week {
            return;
        }

        let nr_shifts = new_first_week - self.first_index_week;
        if nr_shifts < CLAIM_FLAGS_LEN {
            // shift to the left by nr_shifts
            let _ = self.claim_flags.drain(0..nr_shifts);

            let new_pos_first_index = CLAIM_FLAGS_LEN - nr_shifts;
            for i in new_pos_first_index..CLAIM_FLAGS_LEN {
                self.claim_flags[i] = ClaimFlag::NotClaimed;
            }
        } else {
            self.claim_flags = default_claim_flags();
        }

        self.first_index_week = new_first_week;
    }
}

impl<M: ManagedTypeApi> ClaimProgressTracker<M> for ShiftingClaimProgress<M> {
    fn is_week_valid(&self, week: Week) -> bool {
        let last_index = self.first_index_week + CLAIM_FLAGS_LEN - 1;
        week >= self.first_index_week && week <= last_index
    }

    fn get_claim_flags_for_week(&self, week: Week) -> &ClaimFlag<M> {
        if !self.is_week_valid(week) {
            M::error_api_impl().signal_error(INVALID_WEEK_NR_ERR_MSG);
        }

        let index = self.get_index_for_week(week);
        &self.claim_flags[index]
    }

    fn get_mut_claim_flags_for_week(&mut self, week: Week) -> &mut ClaimFlag<M> {
        if !self.is_week_valid(week) {
            M::error_api_impl().signal_error(INVALID_WEEK_NR_ERR_MSG);
        }

        let index = self.get_index_for_week(week);
        &mut self.claim_flags[index]
    }

    fn set_claimed_for_week(&mut self, week: Week, unclaimed_projects: ProjIdsVec<M>) {
        if !self.is_week_valid(week) {
            return;
        }

        let index = self.get_index_for_week(week);
        self.claim_flags[index] = ClaimFlag::Claimed { unclaimed_projects };
    }
}

#[multiversx_sc::module]
pub trait ClaimProgressModule {
    fn get_claim_progress(
        &self,
        user: &ManagedAddress,
        current_week: Week,
    ) -> ShiftingClaimProgress<Self::Api> {
        let mapper = self.claim_progress(user);
        if !mapper.is_empty() {
            let mut existing_progress = mapper.get();
            existing_progress.shift_if_needed(current_week);

            return existing_progress;
        }

        let mut claim_flags = default_claim_flags();
        let first_accepted_week =
            ShiftingClaimProgress::<Self::Api>::get_first_index_week_for_new_current_week(
                current_week,
            );
        for (i, week) in (first_accepted_week..=current_week).enumerate() {
            let old_flag = self.legacy_rewards_claimed_flag(user, week).get();
            claim_flags[i] = ClaimFlag::from_old_flag(old_flag);
        }

        ShiftingClaimProgress::new(claim_flags, current_week)
    }

    #[storage_mapper("claimProgress")]
    fn claim_progress(
        &self,
        user: &ManagedAddress,
    ) -> SingleValueMapper<ShiftingClaimProgress<Self::Api>>;

    #[storage_mapper("rewardsClaimed")]
    fn legacy_rewards_claimed_flag(
        &self,
        user: &ManagedAddress,
        week: Week,
    ) -> SingleValueMapper<bool>;
}

#[cfg(test)]
mod claim_progress_tests {
    use super::*;

    #[test]
    fn claim_progress_shift_test() {
        // let mut progress = ShiftingClaimProgress {
        //     claim_flags: [true, false, true, true, false],
        //     first_index_week: FIRST_WEEK,
        // };

        // // no shift needed
        // for i in FIRST_WEEK..=CLAIM_FLAGS_LEN {
        //     progress.shift_if_needed(i);
        //     assert!(
        //         progress.claim_flags == [true, false, true, true, false]
        //             && progress.first_index_week == FIRST_WEEK
        //     );
        // }

        // // shift by 1
        // let mut expected_first_index_week = FIRST_WEEK + 1;
        // let mut current_week = CLAIM_FLAGS_LEN + 1;
        // progress.shift_if_needed(current_week);
        // assert!(
        //     progress.claim_flags == [false, true, true, false, false]
        //         && progress.first_index_week == expected_first_index_week
        // );

        // // shift by 2
        // expected_first_index_week += 2;
        // current_week += 2;
        // progress.shift_if_needed(current_week);
        // assert!(
        //     progress.claim_flags == [true, false, false, false, false]
        //         && progress.first_index_week == expected_first_index_week
        // );

        // // test full shift
        // progress.claim_flags = [true; CLAIM_FLAGS_LEN];
        // expected_first_index_week += CLAIM_FLAGS_LEN;
        // current_week += CLAIM_FLAGS_LEN;
        // progress.shift_if_needed(current_week);
        // assert!(
        //     progress.claim_flags == [false; CLAIM_FLAGS_LEN]
        //         && progress.first_index_week == expected_first_index_week
        // );

        // // shift all flags but 1
        // progress.claim_flags = [true; CLAIM_FLAGS_LEN];
        // expected_first_index_week += CLAIM_FLAGS_LEN - 1;
        // current_week += CLAIM_FLAGS_LEN - 1;
        // progress.shift_if_needed(current_week);
        // assert!(
        //     progress.claim_flags == [true, false, false, false, false]
        //         && progress.first_index_week == expected_first_index_week
        // );
    }
}
