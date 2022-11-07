use crate::rewards::FIRST_WEEK;

elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait LegacyStorageCleanupModule:
    crate::project::ProjectModule
    + crate::common_storage::CommonStorageModule
    + crate::claim_progress::ClaimProgressModule
{
    #[only_owner]
    #[endpoint(clearOldStorageFlags)]
    fn clear_old_storage_flags(&self, users: MultiValueEncoded<ManagedAddress>) {
        let current_week = self.get_current_week();
        let nr_first_grace_weeks = self.rewards_nr_first_grace_weeks().get();
        for user in users {
            let grace_progress =
                self.get_grace_weeks_progress(&user, nr_first_grace_weeks, current_week);
            let shifting_progress = self.get_shifting_progress(&user, current_week);

            for week in FIRST_WEEK..=current_week {
                self.legacy_rewards_claimed_flag(&user, week).clear();
            }

            self.claim_progress_grace_weeks(&user).set(&grace_progress);
            self.shifting_claim_progress(&user).set(&shifting_progress);
        }
    }
}
