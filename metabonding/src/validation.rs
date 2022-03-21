elrond_wasm::imports!();

use crate::rewards::Week;
use elrond_wasm::api::ED25519_SIGNATURE_BYTE_LEN;

const MAX_DATA_LEN: usize = 80; // 4 + 32 + 32, with some extra for high BigUint values

pub type Signature<M> = ManagedByteArray<M, ED25519_SIGNATURE_BYTE_LEN>;

#[elrond_wasm::module]
pub trait ValidationModule: crate::common_storage::CommonStorageModule {
    fn verify_signature(
        &self,
        week: Week,
        caller: &ManagedAddress,
        user_delegation_amount: &BigUint,
        user_lkmex_staked_amount: &BigUint,
        signature: &Signature<Self::Api>,
    ) {
        let mut data = ManagedBuffer::new();
        let _ = week.dep_encode(&mut data);
        data.append(caller.as_managed_buffer());
        let _ = user_delegation_amount.dep_encode(&mut data);
        let _ = user_lkmex_staked_amount.dep_encode(&mut data);

        let signer: ManagedAddress = self.signer().get();
        let valid_signature = self.crypto().verify_ed25519_managed::<MAX_DATA_LEN>(
            signer.as_managed_byte_array(),
            &data,
            signature,
        );
        require!(valid_signature, "Invalid signature");
    }
}
