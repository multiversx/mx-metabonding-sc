use crate::{project::ProjectId, rewards::week_timekeeping::Week, GROWTH_SIGNATURE_PREFIX};

multiversx_sc::imports!();

pub type Signature<M> = ManagedByteArray<M, ED25519_SIGNATURE_BYTE_LEN>;
pub const ED25519_SIGNATURE_BYTE_LEN: usize = 64;

pub struct SignatureData<'a, M: ManagedTypeApi> {
    pub caller: &'a ManagedAddress<M>,
    pub project_id: ProjectId,
    pub week: Week,
    pub note: &'a ManagedBuffer<M>,
}

#[multiversx_sc::module]
pub trait ValidationModule: crate::project::ProjectsModule + crate::events::EventsModule {
    #[endpoint(changeSigner)]
    fn change_signer(&self, project_id: ProjectId, new_signer: ManagedAddress) {
        let caller = self.blockchain().get_caller();
        self.require_sc_owner_or_project_owner(&caller, project_id);

        self.signer(project_id).set(&new_signer);

        self.emit_change_signer_event(project_id, &new_signer);
    }

    fn verify_signature(
        &self,
        signature_data: SignatureData<Self::Api>,
        signature: &Signature<Self::Api>,
    ) {
        let mut data = ManagedBuffer::new();
        let _ = GROWTH_SIGNATURE_PREFIX.dep_encode(&mut data);
        let _ = signature_data.project_id.dep_encode(&mut data);
        let _ = signature_data.week.dep_encode(&mut data);
        let _ = signature_data.caller.dep_encode(&mut data);
        let _ = signature_data.note.dep_encode(&mut data);

        let signer = self.signer(signature_data.project_id).get();
        self.crypto().verify_ed25519(
            signer.as_managed_buffer(),
            &data,
            signature.as_managed_buffer(),
        );
    }

    #[storage_mapper("signer")]
    fn signer(&self, project_id: ProjectId) -> SingleValueMapper<ManagedAddress>;
}
