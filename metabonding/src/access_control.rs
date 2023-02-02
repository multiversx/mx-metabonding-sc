multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait AccessControlModule: crate::common_storage::CommonStorageModule {
    fn require_caller_owner_or_signer(&self) {
        let caller = self.blockchain().get_caller();
        let owner = self.blockchain().get_owner_address();
        let signer = self.signer().get();
        require!(
            caller == owner || caller == signer,
            "Only owner or signer may call this function"
        );
    }
}
