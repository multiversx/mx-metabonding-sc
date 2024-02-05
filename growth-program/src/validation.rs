use week_timekeeping::Week;

use crate::project::ProjectId;

multiversx_sc::imports!();

pub type Signature<M> = ManagedByteArray<M, ED25519_SIGNATURE_BYTE_LEN>;
pub const ED25519_SIGNATURE_BYTE_LEN: usize = 64;

#[multiversx_sc::module]
pub trait ValidationModule {
    #[only_owner]
    #[endpoint(changeSigner)]
    fn change_signer(&self, new_signer: ManagedAddress) {
        self.signer().set(&new_signer);
    }

    fn verify_signature(
        &self,
        caller: &ManagedAddress,
        project_id: ProjectId,
        week: Week,
        signature: &Signature<Self::Api>,
    ) {
        let mut data = ManagedBuffer::new();
        let _ = caller.dep_encode(&mut data);
        let _ = project_id.dep_encode(&mut data);
        let _ = week.dep_encode(&mut data);

        let signer = self.signer().get();
        self.crypto().verify_ed25519(
            signer.as_managed_buffer(),
            &data,
            signature.as_managed_buffer(),
        );
    }

    #[storage_mapper("signer")]
    fn signer(&self) -> SingleValueMapper<ManagedAddress>;
}
