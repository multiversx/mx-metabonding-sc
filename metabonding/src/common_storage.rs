multiversx_sc::imports!();

use crate::project::Epoch;

pub const EPOCHS_IN_WEEK: Epoch = 7;
pub const MAX_PERCENTAGE: u64 = 100;

#[multiversx_sc::module]
pub trait CommonStorageModule {
    #[storage_mapper("signer")]
    fn signer(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("firstWeekStartEpoch")]
    fn first_week_start_epoch(&self) -> SingleValueMapper<Epoch>;
}
