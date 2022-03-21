elrond_wasm::imports!();

use crate::project::Epoch;
use elrond_wasm::elrond_codec::TopEncode;

pub const EPOCHS_IN_WEEK: Epoch = 7;
pub const MAX_PERCENTAGE: u64 = 100;

#[elrond_wasm::module]
pub trait CommonStorageModule {
    fn get_and_clear<T: TopEncode + TopDecode>(&self, mapper: &SingleValueMapper<T>) -> T {
        let value = mapper.get();
        mapper.clear();

        value
    }

    #[storage_mapper("signer")]
    fn signer(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("firstWeekStartEpoch")]
    fn first_week_start_epoch(&self) -> SingleValueMapper<Epoch>;
}
