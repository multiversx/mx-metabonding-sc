multiversx_sc::imports!();

use crate::rewards::Week;
use multiversx_sc::api::ED25519_SIGNATURE_BYTE_LEN;

// week + caller + user_delegation_amount + user_lkmex_staked_amount
// 4 + 32 + (4 + 32) + (4 + 32) = 108, with some extra for high BigUint values
const MAX_DATA_LEN: usize = 120;

pub type Signature<M> = ManagedByteArray<M, ED25519_SIGNATURE_BYTE_LEN>;

#[multiversx_sc::module]
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
        let valid_signature = self.crypto().verify_ed25519_legacy_managed::<MAX_DATA_LEN>(
            signer.as_managed_byte_array(),
            &data,
            signature,
        );
        require!(valid_signature, "Invalid signature");
    }
}
