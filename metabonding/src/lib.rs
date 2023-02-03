#![no_std]

multiversx_sc::imports!();

pub mod access_control;
pub mod claim;
pub mod claim_progress;
pub mod common_storage;
pub mod legacy_storage_cleanup;
pub mod math;
pub mod project;
pub mod rewards;
pub mod validation;

#[multiversx_sc::contract]
pub trait Metabonding:
    multiversx_sc_modules::pause::PauseModule
    + project::ProjectModule
    + rewards::RewardsModule
    + claim::ClaimModule
    + claim_progress::ClaimProgressModule
    + access_control::AccessControlModule
    + common_storage::CommonStorageModule
    + math::MathModule
    + validation::ValidationModule
    + legacy_storage_cleanup::LegacyStorageCleanupModule
    + sc_whitelist_module::SCWhitelistModule
{
    /// Arguments:
    /// - signer - public key that will be used for checking the claim signatures
    /// - opt_first_week_start_epoch - The epoch which signals the start of week 0.
    ///     Can also be an epoch from the past.
    ///     By default, the current epoch on deploy will be used
    #[init]
    fn init(&self, signer: ManagedAddress, opt_first_week_start_epoch: OptionalValue<u64>) {
        self.signer().set(&signer);
        self.set_paused(true);

        let first_week_start_epoch = match opt_first_week_start_epoch {
            OptionalValue::Some(epoch) => epoch,
            OptionalValue::None => self.blockchain().get_block_epoch(),
        };
        self.first_week_start_epoch()
            .set_if_empty(first_week_start_epoch);
    }

    #[only_owner]
    #[endpoint(changeSigner)]
    fn change_signer(&self, new_signer: ManagedAddress) {
        self.signer().set(&new_signer);
    }
}
