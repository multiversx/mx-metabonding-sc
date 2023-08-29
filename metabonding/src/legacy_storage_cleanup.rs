use crate::rewards::{Week, FIRST_WEEK};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait LegacyStorageCleanupModule:
    crate::project::ProjectModule
    + crate::common_storage::CommonStorageModule
    + crate::claim_progress::ClaimProgressModule
{
    #[only_owner]
    #[endpoint(clearOldStorageFlags)]
    fn clear_old_storage_flags(&self, users: MultiValueEncoded<ManagedAddress>) {
        let current_week = self.get_current_week();
        for user in users {
            let claim_progress = self.get_claim_progress(&user, current_week);
            self.claim_progress(&user).set(claim_progress);

            self.clear_legacy_flags(&user, current_week);
        }
    }

    fn clear_legacy_flags(&self, user: &ManagedAddress, current_week: Week) {
        for week in FIRST_WEEK..=current_week {
            self.legacy_rewards_claimed_flag(user, week).clear();
        }
    }
}
