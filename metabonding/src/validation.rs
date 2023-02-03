multiversx_sc::imports!();

use crate::{
    claim::{ClaimArgArray, ClaimArgsWrapper},
    claim_progress::{ClaimProgressTracker, ShiftingClaimProgress},
    rewards::{Week, FIRST_WEEK},
};
use multiversx_sc::api::ED25519_SIGNATURE_BYTE_LEN;

// week + caller + user_delegation_amount + user_lkmex_staked_amount
// 4 + 32 + (4 + 32) + (4 + 32) = 108, with some extra for high BigUint values
const MAX_DATA_LEN: usize = 120;

pub type Signature<M> = ManagedByteArray<M, ED25519_SIGNATURE_BYTE_LEN>;

pub static ALREADY_CLAIMED_ERR_MSG: &[u8] = b"Already claimed rewards for this week";
pub static INVALID_WEEK_NR_ERR_MSG: &[u8] = b"Invalid week number";

#[multiversx_sc::module]
pub trait ValidationModule: crate::common_storage::CommonStorageModule {
    fn verify_signature(&self, caller: &ManagedAddress, claim_arg: &ClaimArgsWrapper<Self::Api>) {
        let mut data = ManagedBuffer::new();
        let _ = claim_arg.week.dep_encode(&mut data);
        data.append(caller.as_managed_buffer());
        let _ = claim_arg.user_delegation_amount.dep_encode(&mut data);
        let _ = claim_arg.user_lkmex_staked_amount.dep_encode(&mut data);

        let signer = self.signer().get();
        let valid_signature = self.crypto().verify_ed25519_legacy_managed::<MAX_DATA_LEN>(
            signer.as_managed_byte_array(),
            &data,
            &claim_arg.signature,
        );
        require!(valid_signature, "Invalid signature");
    }

    fn validate_claim_args(
        &self,
        caller: &ManagedAddress,
        claim_args: &ClaimArgArray<Self::Api>,
        shifting_progress: &ShiftingClaimProgress,
        last_checkpoint_week: Week,
    ) {
        for claim_arg in claim_args {
            self.validate_single_claim_arg(
                caller,
                claim_arg,
                shifting_progress,
                last_checkpoint_week,
            );
        }
    }

    fn validate_single_claim_arg(
        &self,
        caller: &ManagedAddress,
        claim_arg: &ClaimArgsWrapper<Self::Api>,
        claim_progress: &ShiftingClaimProgress,
        last_checkpoint_week: Week,
    ) {
        let claim_week = claim_arg.week;
        require!(
            claim_week >= FIRST_WEEK && claim_week <= last_checkpoint_week,
            INVALID_WEEK_NR_ERR_MSG
        );
        require!(
            claim_progress.is_week_valid(claim_week),
            INVALID_WEEK_NR_ERR_MSG
        );
        require!(
            claim_progress.can_claim_for_week(claim_week),
            ALREADY_CLAIMED_ERR_MSG
        );

        self.verify_signature(caller, claim_arg);
    }
}
