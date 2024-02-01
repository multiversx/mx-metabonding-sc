multiversx_sc::imports!();

pub type ProjectId = u64;

#[multiversx_sc::module]
pub trait ProjectsModule {
    #[only_owner]
    #[endpoint(addProject)]
    fn add_project(&self, project_owner: ManagedAddress) -> ProjectId {
        let project_id = self.get_and_save_new_project_id();
        self.project_owner(project_id).set(project_owner);

        project_id
    }

    #[only_owner]
    #[endpoint(setProjectOwner)]
    fn set_project_owner(&self, project_id: ProjectId, new_owner: ManagedAddress) {
        self.project_owner(project_id).set(new_owner);
    }

    fn get_and_save_new_project_id(&self) -> ProjectId {
        let new_project_id = self.last_project_id().get() + 1;
        self.last_project_id().set(new_project_id);

        new_project_id
    }

    #[storage_mapper("lastProjectId")]
    fn last_project_id(&self) -> SingleValueMapper<ProjectId>;

    #[storage_mapper("projectOwner")]
    fn project_owner(&self, project_id: ProjectId) -> SingleValueMapper<ManagedAddress>;
}
