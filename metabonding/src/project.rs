elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub type ProjectId<M> = ManagedBuffer<M>;
pub type Epoch = u64;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct Project<M: ManagedTypeApi> {
    pub reward_token: TokenIdentifier<M>,
    pub reward_supply: BigUint<M>,
    pub start_epoch: Epoch,
    pub end_epoch: Epoch,
}

#[elrond_wasm::module]
pub trait ProjectModule {
    #[only_owner]
    #[endpoint(addProject)]
    fn add_project(
        &self,
        project_id: ProjectId<Self::Api>,
        reward_token: TokenIdentifier,
        reward_supply: BigUint,
        start_epoch: Epoch,
        end_epoch: Epoch,
    ) {
        require!(
            reward_token.is_valid_esdt_identifier(),
            "Invalid reward token"
        );
        require!(reward_supply > 0, "Reward supply cannot be 0");

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            start_epoch >= current_epoch,
            "Start epoch cannot be in the past"
        );
        require!(start_epoch < end_epoch, "Invalid end epoch");

        require!(!project_id.is_empty(), "Invalid project ID");

        let project = Project {
            reward_token,
            reward_supply,
            start_epoch,
            end_epoch,
        };
        let insert_result = self.projects().insert(project_id, project);
        require!(insert_result.is_none(), "ID already in use");
    }

    #[only_owner]
    #[endpoint(removeProject)]
    fn remove_project(&self, project_id: ProjectId<Self::Api>) {
        let remove_result = self.projects().remove(&project_id);
        require!(remove_result.is_some(), "Invalid project ID");
    }

    #[storage_mapper("projects")]
    fn projects(&self) -> MapMapper<ProjectId<Self::Api>, Project<Self::Api>>;
}
