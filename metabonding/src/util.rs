elrond_wasm::imports!();

use elrond_wasm::api::ED25519_SIGNATURE_BYTE_LEN;

use crate::rewards::Week;

const MAX_DATA_LEN: usize = 80; // 4 + 32 + 32, with some extra for high BigUint values

pub type Signature<M> = ManagedByteArray<M, ED25519_SIGNATURE_BYTE_LEN>;

#[elrond_wasm::module]
pub trait UtilModule {
    fn require_caller_owner_or_signer(&self) {
        let caller = self.blockchain().get_caller();
        let owner = self.blockchain().get_owner_address();
        let signer = self.signer().get();
        require!(
            caller == owner || caller == signer,
            "Only owner or signer may call this function"
        );
    }

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

    #[inline]
    fn is_in_range(&self, value: Week, min: Week, max: Week) -> bool {
        (min..=max).contains(&value)
    }

    fn calculate_ratio(&self, amount: &BigUint, part: &BigUint, total: &BigUint) -> BigUint {
        if total == &0 {
            return BigUint::zero();
        }

        &(amount * part) / total
    }

    #[storage_mapper("signer")]
    fn signer(&self) -> SingleValueMapper<ManagedAddress>;
}
