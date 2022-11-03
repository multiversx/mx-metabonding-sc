elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{
    project::PROJECT_EXPIRATION_WEEKS,
    rewards::{Week, FIRST_WEEK},
};

type ClaimFlag = bool;
const CLAIMED: ClaimFlag = true;
const NOT_CLAIMED: ClaimFlag = false;

pub trait ClaimProgressTracker {
    fn is_week_valid(&self, week: Week) -> bool;

    fn can_claim_for_week(&self, week: Week) -> bool;

    fn set_claimed_for_week(&mut self, week: Week);
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct ClaimProgressGraceWeeks<M: ManagedTypeApi> {
    claim_flags: ManagedVec<M, ClaimFlag>,
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

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct ShiftingClaimProgress {
    claim_flags: [ClaimFlag; PROJECT_EXPIRATION_WEEKS],
    first_index_week: Week,
}

impl ShiftingClaimProgress {
    fn new(claim_flags: [ClaimFlag; PROJECT_EXPIRATION_WEEKS], current_week: Week) -> Self {
        let first_index_week = if current_week > PROJECT_EXPIRATION_WEEKS {
            current_week - PROJECT_EXPIRATION_WEEKS
        } else {
            FIRST_WEEK
        };

        Self {
            claim_flags,
            first_index_week,
        }
    }

    fn get_index_for_week(&self, week: Week) -> usize {
        week - self.first_index_week
    }

    fn shift_if_needed(&mut self, current_week: Week) {
        if current_week <= PROJECT_EXPIRATION_WEEKS {
            return;
        }

        let new_first_week = current_week - PROJECT_EXPIRATION_WEEKS;
        if self.first_index_week == new_first_week {
            return;
        }

        let nr_shifts = new_first_week - self.first_index_week;
        if nr_shifts < PROJECT_EXPIRATION_WEEKS {
            self.claim_flags
                .copy_within(nr_shifts..PROJECT_EXPIRATION_WEEKS, 0);

            let new_pos_first_index = PROJECT_EXPIRATION_WEEKS - nr_shifts;
            for i in new_pos_first_index..PROJECT_EXPIRATION_WEEKS {
                self.claim_flags[i] = NOT_CLAIMED;
            }
        } else {
            self.claim_flags = [NOT_CLAIMED; PROJECT_EXPIRATION_WEEKS];
        }

        self.first_index_week = new_first_week;
    }
}

impl ClaimProgressTracker for ShiftingClaimProgress {
    fn is_week_valid(&self, week: Week) -> bool {
        let last_index = self.first_index_week + PROJECT_EXPIRATION_WEEKS - 1;
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
    #[only_owner]
    #[endpoint(clearOldStorageFlags)]
    fn clear_old_storage_flags(&self, _users: MultiValueEncoded<ManagedAddress>) {}

    fn get_grace_weeks_progress(
        &self,
        user: &ManagedAddress,
    ) -> ClaimProgressGraceWeeks<Self::Api> {
        let mapper = self.claim_progress_grace_weeks(user);
        if !mapper.is_empty() {
            return mapper.get();
        }

        // index 0 is unused
        let mut claim_flags = ManagedVec::from_single_item(NOT_CLAIMED);
        let nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();
        for week in FIRST_WEEK..=nr_first_grace_weeks {
            let old_flag = self.legacy_rewards_claimed_flag(user, week).get();
            claim_flags.push(old_flag);
        }

        ClaimProgressGraceWeeks { claim_flags }
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

        let mut claim_flags = [NOT_CLAIMED; PROJECT_EXPIRATION_WEEKS];
        let nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();
        if current_week <= nr_first_grace_weeks {
            return ShiftingClaimProgress::new(claim_flags, current_week);
        }

        let first_accepted_week = if current_week > PROJECT_EXPIRATION_WEEKS {
            current_week - PROJECT_EXPIRATION_WEEKS + 1
        } else {
            FIRST_WEEK
        };
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
