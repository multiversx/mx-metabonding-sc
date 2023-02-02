elrond_wasm::imports!();

use crate::{
    claim::{ClaimArgArray, SignedClaimArgArray, SignedClaimArgs},
    claim_progress::{ClaimProgressGraceWeeks, ClaimProgressTracker, ShiftingClaimProgress},
    rewards::{Week, FIRST_WEEK},
};
use elrond_wasm::api::ED25519_SIGNATURE_BYTE_LEN;

// week + caller + user_delegation_amount + user_lkmex_staked_amount
// 4 + 32 + (4 + 32) + (4 + 32) = 108, with some extra for high BigUint values
const MAX_DATA_LEN: usize = 120;

pub type Signature<M> = ManagedByteArray<M, ED25519_SIGNATURE_BYTE_LEN>;

pub static ALREADY_CLAIMED_ERR_MSG: &[u8] = b"Already claimed rewards for this week";
pub static INVALID_WEEK_NR_ERR_MSG: &[u8] = b"Invalid week number";

#[elrond_wasm::module]
pub trait ValidationModule: crate::common_storage::CommonStorageModule {
    fn verify_signature(
        &self,
        caller: &ManagedAddress,
        signed_claim_arg: &SignedClaimArgs<Self::Api>,
    ) {
        let week = signed_claim_arg.args_wrapper.week;
        let user_delegation_amount = &signed_claim_arg.args_wrapper.user_delegation_amount;
        let user_lkmex_staked_amount = &signed_claim_arg.args_wrapper.user_lkmex_staked_amount;

        let mut data = ManagedBuffer::new();
        let _ = week.dep_encode(&mut data);
        data.append(caller.as_managed_buffer());
        let _ = user_delegation_amount.dep_encode(&mut data);
        let _ = user_lkmex_staked_amount.dep_encode(&mut data);

        let signer = self.signer().get();
        let valid_signature = self.crypto().verify_ed25519_legacy_managed::<MAX_DATA_LEN>(
            signer.as_managed_byte_array(),
            &data,
            &signed_claim_arg.signature,
        );
        require!(valid_signature, "Invalid signature");
    }

    fn validate_claim_args(
        &self,
        caller: &ManagedAddress,
        signed_args: SignedClaimArgArray<Self::Api>,
        grace_weeks_progress: &ClaimProgressGraceWeeks<Self::Api>,
        shifting_progress: &ShiftingClaimProgress,
        last_checkpoint_week: Week,
    ) -> ClaimArgArray<Self::Api> {
        let mut validated_args = ArrayVec::new();
        for signed_claim_arg in signed_args {
            self.validate_single_signed_claim_arg(
                caller,
                &signed_claim_arg,
                grace_weeks_progress,
                shifting_progress,
                last_checkpoint_week,
            );

            unsafe {
                validated_args.push_unchecked(signed_claim_arg.args_wrapper);
            }
        }

        validated_args
    }

    fn validate_single_signed_claim_arg(
        &self,
        caller: &ManagedAddress,
        signed_claim_arg: &SignedClaimArgs<Self::Api>,
        grace_weeks_progress: &ClaimProgressGraceWeeks<Self::Api>,
        shifting_progress: &ShiftingClaimProgress,
        last_checkpoint_week: Week,
    ) {
        let claim_week = signed_claim_arg.args_wrapper.week;
        require!(
            claim_week >= FIRST_WEEK && claim_week <= last_checkpoint_week,
            INVALID_WEEK_NR_ERR_MSG
        );

        let is_valid_grace_week = grace_weeks_progress.is_week_valid(claim_week);
        let is_valid_shifting_week = shifting_progress.is_week_valid(claim_week);
        require!(
            is_valid_grace_week || is_valid_shifting_week,
            INVALID_WEEK_NR_ERR_MSG
        );

        let can_claim_grace = grace_weeks_progress.can_claim_for_week(claim_week);
        let can_claim_shifting = shifting_progress.can_claim_for_week(claim_week);
        require!(
            can_claim_grace || can_claim_shifting,
            ALREADY_CLAIMED_ERR_MSG
        );

        self.verify_signature(caller, signed_claim_arg);
    }
}
