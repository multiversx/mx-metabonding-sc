multiversx_sc::imports!();

pub type ProjectId = u32;

pub const PROJECT_PAUSED: bool = false;
pub const PROJECT_UNPAUSED: bool = true;

#[multiversx_sc::module]
pub trait ProjectsModule: crate::events::EventsModule {
    #[only_owner]
    #[endpoint(addProject)]
    fn add_project(&self, project_owner: ManagedAddress) -> ProjectId {
        let project_id = self.get_and_save_new_project_id();
        self.project_owner(project_id).set(&project_owner);

        self.emit_add_project_event(&project_owner, project_id);

        project_id
    }

    #[only_owner]
    #[endpoint(setProjectOwner)]
    fn set_project_owner(&self, project_id: ProjectId, new_owner: ManagedAddress) {
        self.require_valid_project_id(project_id);
        self.project_owner(project_id).set(new_owner);
    }

    #[endpoint(pauseProject)]
    fn pause_project(&self, project_id: ProjectId) {
        self.pause_common(project_id, PROJECT_PAUSED);

        self.emit_pause_project_event(project_id);
    }

    #[endpoint(unpauseProject)]
    fn unpause_project(&self, project_id: ProjectId) {
        self.pause_common(project_id, PROJECT_UNPAUSED);

        self.emit_unpause_project_event(project_id);
    }

    fn pause_common(&self, project_id: ProjectId, pause_status: bool) {
        self.require_valid_project_id(project_id);

        let caller = self.blockchain().get_caller();
        self.require_sc_owner_or_project_owner(&caller, project_id);

        self.project_active(project_id).set(pause_status);
    }

    fn get_and_save_new_project_id(&self) -> ProjectId {
        let new_project_id = self.last_project_id().get() + 1;
        self.last_project_id().set(new_project_id);

        new_project_id
    }

    fn require_is_project_owner(&self, address: &ManagedAddress, project_id: ProjectId) {
        let project_owner = self.project_owner(project_id).get();
        require!(
            address == &project_owner,
            "Only project owner may call this endpoint"
        );
    }

    fn require_sc_owner_or_project_owner(&self, address: &ManagedAddress, project_id: ProjectId) {
        let sc_owner = self.blockchain().get_owner_address();
        let project_owner = self.project_owner(project_id).get();
        require!(
            address == &project_owner || address == &sc_owner,
            "Only sc owner or project owner may call this endpoint"
        );
    }

    fn require_valid_project_id(&self, project_id: ProjectId) {
        let last_project_id = self.last_project_id().get();
        require!(project_id <= last_project_id, "Invalid project ID");
    }

    fn require_project_active(&self, project_id: ProjectId) {
        let project_status = self.project_active(project_id).get();
        require!(project_status == PROJECT_UNPAUSED, "Project is paused");
    }

    #[view(isProjectActive)]
    #[storage_mapper("projectActive")]
    fn project_active(&self, project_id: ProjectId) -> SingleValueMapper<bool>;

    #[storage_mapper("lastProjectId")]
    fn last_project_id(&self) -> SingleValueMapper<ProjectId>;

    #[storage_mapper("projectOwner")]
    fn project_owner(&self, project_id: ProjectId) -> SingleValueMapper<ManagedAddress>;
}
