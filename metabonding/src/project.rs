multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    common_storage::{EPOCHS_IN_WEEK, MAX_PERCENTAGE},
    rewards::{Week, FIRST_WEEK},
};
use core::convert::TryInto;

pub const PROJECT_EXPIRATION_WEEKS: Week = 4;
const MAX_PROJECT_ID_LEN: usize = 10;
const MIN_GAS_FOR_CLEAR: u64 = 5_000_000;
static INVALID_PROJECT_ID_ERR_MSG: &[u8] = b"Invalid project ID";

pub type ProjectId<M> = ManagedBuffer<M>;
pub type ProjIdsVec<M> = ManagedVec<M, ProjectId<M>>;
pub type ProjectAsMultiResult<M> =
    MultiValue5<TokenIdentifier<M>, BigUint<M>, BigUint<M>, Week, Week>;
pub type Epoch = u64;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct Project<M: ManagedTypeApi> {
    pub reward_token: TokenIdentifier<M>,
    pub delegation_reward_supply: BigUint<M>,
    pub lkmex_reward_supply: BigUint<M>,
    pub start_week: Week,
    pub end_week: Week,
}

impl<M: ManagedTypeApi> Project<M> {
    #[inline]
    pub fn is_expired(&self, current_week: Week) -> bool {
        current_week > self.end_week + PROJECT_EXPIRATION_WEEKS
    }

    #[inline]
    pub fn get_duration_in_weeks(&self) -> Week {
        self.end_week - self.start_week + 1
    }

    pub fn into_multiresult(self) -> ProjectAsMultiResult<M> {
        (
            self.reward_token,
            self.delegation_reward_supply,
            self.lkmex_reward_supply,
            self.start_week,
            self.end_week,
        )
            .into()
    }
}

#[multiversx_sc::module]
pub trait ProjectModule: crate::common_storage::CommonStorageModule {
    /// Adds a new project. Arguments:
    /// - project_id: a unique ID of maximum 10 bytes
    /// - project_owner - the owner of the project. They will receive any unclaimed funds for the projects.
    /// - reward_token - the token ID of the token given as reward
    /// - reward_supply - total supply of the reward token
    /// - start_week - the week from which the project starts producing rewards. Has to be >= 1.
    /// - duration_weeks - the duration in weeks of the project
    /// - lkmex_rewards_percentage - The percentage of the total rewards which will be given to LKMEX stakers.
    ///     Expected value range is [0, 100]
    #[only_owner]
    #[endpoint(addProject)]
    fn add_project(
        &self,
        project_id: ProjectId<Self::Api>,
        project_owner: ManagedAddress,
        reward_token: TokenIdentifier,
        reward_supply: BigUint,
        start_week: Week,
        duration_weeks: Week,
        lkmex_rewards_percentage: u64,
    ) {
        require!(
            reward_token.is_valid_esdt_identifier(),
            "Invalid reward token"
        );
        require!(reward_supply > 0, "Reward supply cannot be 0");
        require!(
            start_week >= FIRST_WEEK && duration_weeks > 0,
            "Invalid duration"
        );

        require!(
            lkmex_rewards_percentage <= MAX_PERCENTAGE,
            "Invalid percentage"
        );

        let id_len = project_id.len();
        require!(
            id_len > 0 && id_len <= MAX_PROJECT_ID_LEN,
            INVALID_PROJECT_ID_ERR_MSG
        );

        self.project_owner(&project_id).set(&project_owner);

        let lkmex_reward_supply = &reward_supply * lkmex_rewards_percentage / MAX_PERCENTAGE;
        let delegation_reward_supply = &reward_supply - &lkmex_reward_supply;

        let project = Project {
            reward_token,
            delegation_reward_supply,
            lkmex_reward_supply,
            start_week,
            end_week: start_week + duration_weeks - 1,
        };
        let insert_result = self.projects().insert(project_id, project);
        require!(insert_result.is_none(), "ID already in use");
    }

    /// Removes a project and gives any leftover funds to the project_owner
    #[only_owner]
    #[endpoint(removeProject)]
    fn remove_project(&self, project_id: ProjectId<Self::Api>) {
        let project = self.get_project_or_panic(&project_id);
        self.clear_and_refund_project(&project_id, &project.reward_token);
    }

    /// Clears all expired projects and sends the leftover funds to the respective project_owner.
    /// A project is considered expired if PROJECT_EXPIRATION_WEEKS weeks
    ///     have passed since its last rewards week
    #[only_owner]
    #[endpoint(clearExpiredProjects)]
    fn clear_expired_projects(&self) -> OperationCompletionStatus {
        let mut prev_token = TokenIdentifier::from_esdt_bytes(&[]);
        let mut prev_id = ProjectId::<Self::Api>::new();
        let mut clear_prev_id = false;
        let current_week = self.get_current_week();

        // can only clear on next step, otherwise we'd lose the map's internal links
        for (id, project) in self.projects().iter() {
            let gas_left = self.blockchain().get_gas_left();
            if gas_left < MIN_GAS_FOR_CLEAR {
                return OperationCompletionStatus::InterruptedBeforeOutOfGas;
            }

            if clear_prev_id {
                self.clear_and_refund_project(&prev_id, &prev_token);
                clear_prev_id = false;
            }

            if project.is_expired(current_week) {
                prev_token = project.reward_token;
                prev_id = id;
                clear_prev_id = true;
            }
        }

        if clear_prev_id {
            self.clear_and_refund_project(&prev_id, &prev_token);
        }

        OperationCompletionStatus::Completed
    }

    fn clear_and_refund_project(
        &self,
        project_id: &ProjectId<Self::Api>,
        token_id: &TokenIdentifier,
    ) {
        let project_owner = self.project_owner(project_id).take();
        let leftover_funds = self.leftover_project_funds(project_id).take();

        let _ = self.projects().remove(project_id);

        if leftover_funds > 0 {
            self.send()
                .direct_esdt(&project_owner, token_id, 0, &leftover_funds);
        }
    }

    #[view(getAllProjectIds)]
    fn get_all_project_ids_view(&self) -> MultiValueEncoded<ProjectId<Self::Api>> {
        self.get_all_project_ids().into()
    }

    fn get_all_project_ids(&self) -> ProjIdsVec<Self::Api> {
        let mut all_ids = ManagedVec::new();
        for id in self.projects().keys() {
            all_ids.push(id);
        }

        all_ids
    }

    /// Returns a project by ID. The results are, in order:
    /// - reward_token
    /// - delegation_reward_supply
    /// - lkmex_reward_supply
    /// - start_week
    /// - end_week
    #[view(getProjectById)]
    fn get_project_by_id(
        &self,
        project_id: ProjectId<Self::Api>,
    ) -> ProjectAsMultiResult<Self::Api> {
        self.get_project_or_panic(&project_id).into_multiresult()
    }

    fn get_project_or_panic(&self, project_id: &ProjectId<Self::Api>) -> Project<Self::Api> {
        self.projects()
            .get(project_id)
            .unwrap_or_else(|| sc_panic!(INVALID_PROJECT_ID_ERR_MSG))
    }

    #[view(getCurrentWeek)]
    fn get_current_week(&self) -> Week {
        let first_week_start_epoch = self.first_week_start_epoch().get();
        let current_epoch = self.blockchain().get_block_epoch();

        // will never overflow usize
        unsafe {
            ((current_epoch - first_week_start_epoch) / EPOCHS_IN_WEEK)
                .try_into()
                .unwrap_unchecked()
        }
    }

    #[storage_mapper("projects")]
    fn projects(&self) -> MapMapper<ProjectId<Self::Api>, Project<Self::Api>>;

    #[storage_mapper("projectOwner")]
    fn project_owner(&self, project_id: &ProjectId<Self::Api>)
        -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("leftoverProjectFunds")]
    fn leftover_project_funds(
        &self,
        project_id: &ProjectId<Self::Api>,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("rewardsDeposited")]
    fn rewards_deposited(&self, project_id: &ProjectId<Self::Api>) -> SingleValueMapper<bool>;
}
