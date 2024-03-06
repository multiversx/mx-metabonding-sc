use crate::{project::ProjectId, validation::Signature, MAX_PERCENTAGE};

use super::{
    notes_history::NoteData,
    week_timekeeping::{Epoch, Week},
};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy)]
pub enum LockOption {
    None,
    OneWeek,
    TwoWeeks,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy)]
pub enum ClaimType {
    Exemption,
    Rewards(LockOption),
}

impl LockOption {
    pub fn get_lock_period(&self) -> Epoch {
        match *self {
            LockOption::None => NO_LOCK_PERIOD,
            LockOption::OneWeek => ONE_WEEK_LOCK_PERIOD,
            LockOption::TwoWeeks => TWO_WEEKS_LOCK_PERIOD,
        }
    }
}

pub const NONE_PERCENTAGE: u32 = 25 * MAX_PERCENTAGE / 100; // 25%
pub const ONE_WEEK_PERCENTAGE: u32 = 50 * MAX_PERCENTAGE / 100; // 50%
pub const TWO_WEEKS_PERCENTAGE: u32 = 100 * MAX_PERCENTAGE / 100; // 100%

pub const EPOCHS_IN_WEEK: Epoch = 7;
pub const NO_LOCK_PERIOD: Epoch = 0;
pub const ONE_WEEK_LOCK_PERIOD: Epoch = EPOCHS_IN_WEEK;
pub const TWO_WEEKS_LOCK_PERIOD: Epoch = 2 * EPOCHS_IN_WEEK;

pub type OptClaimArgType<M> = OptionalValue<MultiValue2<NoteData<M>, Signature<M>>>;

pub struct CheckSignatureArgs<'a, M: ManagedTypeApi> {
    pub project_id: ProjectId,
    pub user_id: AddressId,
    pub current_week: Week,
    pub caller: &'a ManagedAddress<M>,
    pub opt_note_and_signature: OptClaimArgType<M>,
}

pub struct ClaimExemptionArgs<M: ManagedTypeApi> {
    pub project_id: ProjectId,
    pub user_id: AddressId,
    pub current_week: Week,
    pub rew_advertised: BigUint<M>,
    pub min_rewards: BigUint<M>,
    pub user_original_energy: BigUint<M>,
    pub reward_token_id: TokenIdentifier<M>,
}

pub struct ClaimRewardsArgs<'a, M: ManagedTypeApi> {
    pub project_id: ProjectId,
    pub user_id: AddressId,
    pub current_week: Week,
    pub rew_advertised: BigUint<M>,
    pub min_rewards: BigUint<M>,
    pub user_original_energy: BigUint<M>,
    pub reward_token_id: TokenIdentifier<M>,
    pub lock_option: LockOption,
    pub caller: &'a ManagedAddress<M>,
}
