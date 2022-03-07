#![no_std]

elrond_wasm::imports!();

mod project;
mod rewards;

use core::borrow::Borrow;

use elrond_wasm::api::ED25519_SIGNATURE_BYTE_LEN;
use rewards::{ManagedHash, RewardsCheckpoint, Week};

const MAX_DATA_LEN: usize = 120; // 32 * 3 bytes, with some extra for high BigUint values

pub type Signature<M> = ManagedByteArray<M, ED25519_SIGNATURE_BYTE_LEN>;

/// Source code for the pause module:
/// https://github.com/ElrondNetwork/elrond-wasm-rs/blob/master/elrond-wasm-modules/src/pause.rs
#[elrond_wasm::contract]
pub trait Metabonding:
    elrond_wasm_modules::pause::PauseModule + project::ProjectModule + rewards::RewardsModule
{
    #[init]
    fn init(&self, signer: ManagedAddress) {
        self.signer().set(&signer);
        self.set_paused(true);

        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set_if_empty(&current_epoch);
    }

    #[only_owner]
    #[endpoint(changeSigner)]
    fn change_signer(&self, new_signer: ManagedAddress) {
        self.signer().set(&new_signer);
    }

    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        week: Week,
        user_delegation_amount: BigUint,
        signature: Signature<Self::Api>,
    ) {
        require!(self.not_paused(), "May not claim rewards while paused");

        let caller = self.blockchain().get_caller();
        require!(
            !self.rewards_claimed(&caller, week).get(),
            "Already claimed rewards for this week"
        );

        let last_checkpoint_week = self.get_last_checkpoint_week();
        require!(week <= last_checkpoint_week, "No checkpoint for week yet");

        let checkpoint: RewardsCheckpoint<Self::Api> = self.rewards_checkpoints().get(week);
        self.verify_signature(
            &caller,
            &checkpoint.root_hash,
            &user_delegation_amount,
            &signature,
        );

        self.rewards_claimed(&caller, week).set(&true);

        let weekly_rewards = self.get_rewards_for_week(
            week,
            &user_delegation_amount,
            &checkpoint.total_delegation_supply,
        );
        if !weekly_rewards.is_empty() {
            for (id, payment) in weekly_rewards.iter() {
                self.leftover_project_funds(id.borrow())
                    .update(|leftover| *leftover -= &payment.amount);
            }

            let _ = Self::Api::send_api_impl().direct_multi_esdt_transfer_execute(
                &caller,
                &weekly_rewards.payments,
                0,
                &ManagedBuffer::new(),
                &ManagedArgBuffer::new_empty(),
            );
        }
    }

    fn verify_signature(
        &self,
        caller: &ManagedAddress,
        root_hash: &ManagedHash<Self::Api>,
        user_delegation_amount: &BigUint,
        signature: &Signature<Self::Api>,
    ) {
        let mut data = caller.as_managed_buffer().clone();
        data.append(root_hash.as_managed_buffer());
        data.append(&user_delegation_amount.to_bytes_be_buffer());

        let signer: ManagedAddress = self.signer().get();
        let valid_signature = self.crypto().verify_ed25519_managed::<MAX_DATA_LEN>(
            signer.as_managed_byte_array(),
            &data,
            signature,
        );
        require!(valid_signature, "Invalid signature");
    }

    #[storage_mapper("signer")]
    fn signer(&self) -> SingleValueMapper<ManagedAddress>;
}
