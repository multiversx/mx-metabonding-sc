elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{
    project::PROJECT_EXPIRATION_WEEKS,
    rewards::{Week, FIRST_WEEK},
};

type ClaimFlag = bool;
const CLAIMED: ClaimFlag = true;
const NOT_CLAIMED: ClaimFlag = false;

const CLAIM_FLAGS_LEN: usize = PROJECT_EXPIRATION_WEEKS + 1;
type ClaimFlagsArray = [ClaimFlag; CLAIM_FLAGS_LEN];
const DEFAULT_CLAIM_FLAGS: ClaimFlagsArray = [NOT_CLAIMED; CLAIM_FLAGS_LEN];

pub trait ClaimProgressTracker {
    fn is_week_valid(&self, week: Week) -> bool;

    fn can_claim_for_week(&self, week: Week) -> bool;

    fn set_claimed_for_week(&mut self, week: Week);
}

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct ClaimProgressGraceWeeks<M: ManagedTypeApi> {
    claim_flags: ManagedVec<M, ClaimFlag>,
}

impl<M: ManagedTypeApi> ClaimProgressGraceWeeks<M> {
    #[inline]
    pub fn new(claim_flags: ManagedVec<M, ClaimFlag>) -> Self {
        Self { claim_flags }
    }
}

impl<M: ManagedTypeApi> ClaimProgressTracker for ClaimProgressGraceWeeks<M> {
    fn is_week_valid(&self, week: Week) -> bool {
        week < self.claim_flags.len()
    }

    fn can_claim_for_week(&self, week: Week) -> bool {
        if !self.is_week_valid(week) {
            return false;
        }

        !self.claim_flags.get(week)
    }

    fn set_claimed_for_week(&mut self, week: Week) {
        if !self.is_week_valid(week) {
            return;
        }

        let _ = self.claim_flags.set(week, &CLAIMED);
    }
}

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct ShiftingClaimProgress {
    claim_flags: ClaimFlagsArray,
    first_index_week: Week,
}

impl ShiftingClaimProgress {
    pub fn new(claim_flags: ClaimFlagsArray, current_week: Week) -> Self {
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
            self.claim_flags.copy_within(nr_shifts..CLAIM_FLAGS_LEN, 0);

            let new_pos_first_index = CLAIM_FLAGS_LEN - nr_shifts;
            for i in new_pos_first_index..CLAIM_FLAGS_LEN {
                self.claim_flags[i] = NOT_CLAIMED;
            }
        } else {
            self.claim_flags = DEFAULT_CLAIM_FLAGS;
        }

        self.first_index_week = new_first_week;
    }
}

impl ClaimProgressTracker for ShiftingClaimProgress {
    fn is_week_valid(&self, week: Week) -> bool {
        let last_index = self.first_index_week + CLAIM_FLAGS_LEN - 1;
        week >= self.first_index_week && week <= last_index
    }

    fn can_claim_for_week(&self, week: Week) -> bool {
        if !self.is_week_valid(week) {
            return false;
        }

        let index = self.get_index_for_week(week);
        !self.claim_flags[index]
    }

    fn set_claimed_for_week(&mut self, week: Week) {
        if !self.is_week_valid(week) {
            return;
        }

        let index = self.get_index_for_week(week);
        self.claim_flags[index] = CLAIMED;
    }
}

#[elrond_wasm::module]
pub trait ClaimProgressModule {
    fn get_grace_weeks_progress(
        &self,
        user: &ManagedAddress,
        nr_first_grace_weeks: Week,
        current_week: Week,
    ) -> ClaimProgressGraceWeeks<Self::Api> {
        if current_week > nr_first_grace_weeks {
            return ClaimProgressGraceWeeks::new(ManagedVec::new());
        }

        let mapper = self.claim_progress_grace_weeks(user);
        if !mapper.is_empty() {
            return mapper.get();
        }

        // index 0 is unused
        let mut claim_flags = ManagedVec::from_single_item(NOT_CLAIMED);
        for week in FIRST_WEEK..=nr_first_grace_weeks {
            let old_flag = self.legacy_rewards_claimed_flag(user, week).get();
            claim_flags.push(old_flag);
        }

        ClaimProgressGraceWeeks::new(claim_flags)
    }

    fn get_shifting_progress(
        &self,
        user: &ManagedAddress,
        current_week: Week,
    ) -> ShiftingClaimProgress {
        let mapper = self.shifting_claim_progress(user);
        if !mapper.is_empty() {
            let mut existing_progress = mapper.get();
            existing_progress.shift_if_needed(current_week);

            return existing_progress;
        }

        let mut claim_flags = DEFAULT_CLAIM_FLAGS;
        let first_accepted_week =
            ShiftingClaimProgress::get_first_index_week_for_new_current_week(current_week);
        for (i, week) in (first_accepted_week..=current_week).enumerate() {
            let old_flag = self.legacy_rewards_claimed_flag(user, week).get();
            claim_flags[i] = old_flag;
        }

        ShiftingClaimProgress::new(claim_flags, current_week)
    }

    #[storage_mapper("rewardsNrFirstGraceWeeks")]
    fn rewards_nr_first_grace_weeks(&self) -> SingleValueMapper<Week>;

    #[storage_mapper("claimProgressGraceWeeks")]
    fn claim_progress_grace_weeks(
        &self,
        user: &ManagedAddress,
    ) -> SingleValueMapper<ClaimProgressGraceWeeks<Self::Api>>;

    #[storage_mapper("shiftingClaimProgress")]
    fn shifting_claim_progress(
        &self,
        user: &ManagedAddress,
    ) -> SingleValueMapper<ShiftingClaimProgress>;

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
        let mut progress = ShiftingClaimProgress {
            claim_flags: [true, false, true, true, false],
            first_index_week: FIRST_WEEK,
        };

        // no shift needed
        for i in FIRST_WEEK..=CLAIM_FLAGS_LEN {
            progress.shift_if_needed(i);
            assert!(
                progress.claim_flags == [true, false, true, true, false]
                    && progress.first_index_week == FIRST_WEEK
            );
        }

        // shift by 1
        let mut expected_first_index_week = FIRST_WEEK + 1;
        let mut current_week = CLAIM_FLAGS_LEN + 1;
        progress.shift_if_needed(current_week);
        assert!(
            progress.claim_flags == [false, true, true, false, false]
                && progress.first_index_week == expected_first_index_week
        );

        // shift by 2
        expected_first_index_week += 2;
        current_week += 2;
        progress.shift_if_needed(current_week);
        assert!(
            progress.claim_flags == [true, false, false, false, false]
                && progress.first_index_week == expected_first_index_week
        );

        // test full shift
        progress.claim_flags = [true; CLAIM_FLAGS_LEN];
        expected_first_index_week += CLAIM_FLAGS_LEN;
        current_week += CLAIM_FLAGS_LEN;
        progress.shift_if_needed(current_week);
        assert!(
            progress.claim_flags == [false; CLAIM_FLAGS_LEN]
                && progress.first_index_week == expected_first_index_week
        );

        // shift all flags but 1
        progress.claim_flags = [true; CLAIM_FLAGS_LEN];
        expected_first_index_week += CLAIM_FLAGS_LEN - 1;
        current_week += CLAIM_FLAGS_LEN - 1;
        progress.shift_if_needed(current_week);
        assert!(
            progress.claim_flags == [true, false, false, false, false]
                && progress.first_index_week == expected_first_index_week
        );
    }
}
