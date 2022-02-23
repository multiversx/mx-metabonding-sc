#![no_std]

elrond_wasm::imports!();

mod project;
mod rewards;

use elrond_wasm::api::InvalidSliceError;
use rewards::{ManagedHash, RewardsCheckpoint};

const PUBKEY_LEN: usize = 32;
const SIGNATURE_LEN: usize = 64;
const MAX_DATA_LEN: usize = 100;

pub type Signature<M> = ManagedByteArray<M, SIGNATURE_LEN>;

/// Source code for the pause module:
/// https://github.com/ElrondNetwork/elrond-wasm-rs/blob/master/elrond-wasm-modules/src/pause.rs
#[elrond_wasm::contract]
pub trait Metabonding:
    elrond_wasm_modules::pause::PauseModule + project::ProjectModule + rewards::RewardsModule
{
    #[init]
    fn init(&self, signer: ManagedAddress) {
        self.signer().set(&signer);
    }

    #[only_owner]
    #[endpoint(changeSigner)]
    fn change_signer(&self, new_signer: ManagedAddress) {
        self.signer().set(&new_signer);
    }

    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        root_hash: ManagedHash<Self::Api>,
        user_delegation_amount: BigUint,
        signature: Signature<Self::Api>,
    ) {
        require!(self.not_paused(), "May not claim rewards while paused");

        self.verify_signature(&root_hash, &user_delegation_amount, &signature);

        let caller = self.blockchain().get_caller();
        require!(
            !self.rewards_claimed(&caller, &root_hash).get(),
            "Already claimed rewards for this root hash"
        );

        let checkpoint_mapper = self.rewards_checkpoints(&root_hash);
        require!(!checkpoint_mapper.is_empty(), "Invalid root hash");
        let checkpoint: RewardsCheckpoint<Self::Api> = checkpoint_mapper.get();

        let mut user_rewards = ManagedVec::new();
        for (id, project) in self.projects().iter() {
            if !self.rewards_deposited(&id).get() {
                continue;
            }
            if !self.is_in_range(checkpoint.epoch, project.start_epoch, project.end_epoch) {
                continue;
            }

            let reward_amount = self.calculate_reward_amount(
                &project.reward_supply,
                &user_delegation_amount,
                &checkpoint.total_delegation_supply,
            );
            if reward_amount > 0 {
                let user_payment = EsdtTokenPayment {
                    token_type: EsdtTokenType::Fungible,
                    token_identifier: project.reward_token,
                    token_nonce: 0,
                    amount: reward_amount,
                };
                user_rewards.push(user_payment);
            }
        }

        self.rewards_claimed(&caller, &root_hash).set(&true);

        if !user_rewards.is_empty() {
            let _ = Self::Api::send_api_impl().direct_multi_esdt_transfer_execute(
                &caller,
                &user_rewards,
                0,
                &ManagedBuffer::new(),
                &ManagedArgBuffer::new_empty(),
            );
        }
    }

    fn is_in_range(&self, value: u64, min: u64, max: u64) -> bool {
        (min..=max).contains(&value)
    }

    fn verify_signature(
        &self,
        root_hash: &ManagedHash<Self::Api>,
        user_delegation_amount: &BigUint,
        signature: &Signature<Self::Api>,
    ) {
        let mut data = root_hash.as_managed_buffer().clone();
        data.append(&user_delegation_amount.to_bytes_be_buffer());

        let data_len = data.len();
        require!(data_len <= MAX_DATA_LEN, "Signature data too long");

        let mut pubkey_buffer = [0u8; PUBKEY_LEN];
        let mut sig_buffer = [0u8; SIGNATURE_LEN];
        let mut data_buffer = [0u8; MAX_DATA_LEN];

        let signer: ManagedAddress = self.signer().get();
        let mut copy_result = signer.as_managed_buffer().load_slice(0, &mut pubkey_buffer);
        self.require_result_ok(&copy_result);

        copy_result = signature.as_managed_buffer().load_slice(0, &mut sig_buffer);
        self.require_result_ok(&copy_result);

        copy_result = data.load_slice(0, &mut data_buffer);
        self.require_result_ok(&copy_result);

        let valid_signature = self.crypto().verify_ed25519(
            &pubkey_buffer[..],
            &data_buffer[..data_len],
            &sig_buffer[..],
        );
        require!(valid_signature, "Invalid signature");
    }

    fn require_result_ok(&self, result: &Result<(), InvalidSliceError>) {
        require!(result.is_ok(), "Could not copy managed buffer to array");
    }

    #[storage_mapper("signer")]
    fn signer(&self) -> SingleValueMapper<ManagedAddress>;
}
