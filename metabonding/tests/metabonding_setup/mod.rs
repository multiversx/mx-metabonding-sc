use elrond_wasm::{
    api::ED25519_SIGNATURE_BYTE_LEN,
    elrond_codec::multi_types::OptionalValue,
    types::{Address, MultiValueEncoded},
};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    tx_mock::TxResult,
    DebugApi,
};
use elrond_wasm_modules::pause::PauseModule;
use metabonding::rewards::RewardsModule;
use metabonding::*;
use metabonding::{claim::ClaimModule, project::ProjectModule};
use metabonding::{
    common_storage::{CommonStorageModule, EPOCHS_IN_WEEK},
    rewards::Week,
};

// associated private key - used for generating the signatures (please don't steal my funds)
// 3eb200ef228e593d49a522f92587889fedfc091629d175873b64ca0ab3b4514d52773868c13654355cca16adb389b09201fabf5d9d4b795ebbdae5b361b46f20
pub static SIGNER_ADDRESS: [u8; 32] =
    hex_literal::hex!("52773868c13654355cca16adb389b09201fabf5d9d4b795ebbdae5b361b46f20");
pub static FIRST_PROJ_ID: &[u8] = b"FirstProj";
pub static SECOND_PROJ_ID: &[u8] = b"SecondProj";
pub static FIRST_PROJ_TOKEN: &[u8] = b"PROJ-123456";
pub static SECOND_PROJ_TOKEN: &[u8] = b"COOL-123456";
pub const TOTAL_FIRST_PROJ_TOKENS: u64 = 1_000_000_000;
pub const TOTAL_SECOND_PROJ_TOKENS: u64 = 2_000_000_000;

pub struct MetabondingSetup<MetabondingObjBuilder>
where
    MetabondingObjBuilder: 'static + Copy + Fn() -> metabonding::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_addr: Address,
    pub first_project_owner: Address,
    pub second_project_owner: Address,
    pub first_user_addr: Address,
    pub second_user_addr: Address,
    pub mb_wrapper: ContractObjWrapper<metabonding::ContractObj<DebugApi>, MetabondingObjBuilder>,
    pub current_epoch: u64,
}

impl<MetabondingObjBuilder> MetabondingSetup<MetabondingObjBuilder>
where
    MetabondingObjBuilder: 'static + Copy + Fn() -> metabonding::ContractObj<DebugApi>,
{
    pub fn new(builder: MetabondingObjBuilder) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let first_project_owner = b_mock.create_user_account(&rust_zero);
        let second_project_owner = b_mock.create_user_account(&rust_zero);

        // need to create some fixed addresses to reuse the signatures from mandos
        // address:user1 from mandos
        let first_user_addr = Address::from(hex_literal::hex!(
            "75736572315F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F"
        ));
        b_mock.create_user_account_fixed_address(&first_user_addr, &rust_zero);

        // address:user2 from mandos
        let second_user_addr = Address::from(hex_literal::hex!(
            "75736572325F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F5F"
        ));
        b_mock.create_user_account_fixed_address(&second_user_addr, &rust_zero);

        b_mock.set_esdt_balance(
            &first_project_owner,
            &FIRST_PROJ_TOKEN,
            &rust_biguint!(TOTAL_FIRST_PROJ_TOKENS),
        );
        b_mock.set_esdt_balance(
            &second_project_owner,
            SECOND_PROJ_TOKEN,
            &rust_biguint!((TOTAL_SECOND_PROJ_TOKENS)),
        );

        let current_epoch = 5;
        b_mock.set_block_epoch(current_epoch);

        let mb_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            builder,
            "metabonding wasm path",
        );
        b_mock
            .execute_tx(&owner_addr, &mb_wrapper, &rust_zero, |sc| {
                let signer_addr = managed_address!(&Address::from(&SIGNER_ADDRESS));
                sc.init(
                    signer_addr.clone(),
                    OptionalValue::None,
                    OptionalValue::None,
                );

                assert_eq!(sc.first_week_start_epoch().get(), 5);
                assert_eq!(sc.signer().get(), signer_addr);
                assert_eq!(sc.is_paused(), true);
            })
            .assert_ok();

        Self {
            b_mock,
            owner_addr,
            first_project_owner,
            second_project_owner,
            first_user_addr,
            second_user_addr,
            mb_wrapper,
            current_epoch,
        }
    }

    pub fn add_default_projects(&mut self) {
        let first_proj_owner = self.first_project_owner.clone();
        self.call_add_project(
            FIRST_PROJ_ID,
            &first_proj_owner,
            FIRST_PROJ_TOKEN,
            TOTAL_FIRST_PROJ_TOKENS,
            1,
            3,
            0,
        )
        .assert_ok();

        let second_proj_owner = self.second_project_owner.clone();
        self.call_add_project(
            SECOND_PROJ_ID,
            &second_proj_owner,
            SECOND_PROJ_TOKEN,
            TOTAL_SECOND_PROJ_TOKENS,
            2,
            5,
            0,
        )
        .assert_ok();
    }

    pub fn deposit_rewards_default_projects(&mut self) {
        let first_proj_owner = self.first_project_owner.clone();
        self.call_deposit_rewards(
            &first_proj_owner,
            FIRST_PROJ_ID,
            FIRST_PROJ_TOKEN,
            TOTAL_FIRST_PROJ_TOKENS,
        )
        .assert_ok();

        let second_proj_owner = self.second_project_owner.clone();
        self.call_deposit_rewards(
            &second_proj_owner,
            SECOND_PROJ_ID,
            SECOND_PROJ_TOKEN,
            TOTAL_SECOND_PROJ_TOKENS,
        )
        .assert_ok();
    }

    pub fn add_default_checkpoints(&mut self) {
        self.set_current_epoch(20);

        self.call_add_rewards_checkpoint(1, 100_000, 0).assert_ok();
        self.call_add_rewards_checkpoint(2, 200_000, 0).assert_ok();
    }
}

impl<MetabondingObjBuilder> MetabondingSetup<MetabondingObjBuilder>
where
    MetabondingObjBuilder: 'static + Copy + Fn() -> metabonding::ContractObj<DebugApi>,
{
    pub fn set_current_epoch(&mut self, epoch: u64) {
        self.current_epoch = epoch;
        self.b_mock.set_block_epoch(epoch);
    }

    pub fn advance_one_week(&mut self) {
        self.current_epoch += EPOCHS_IN_WEEK;
        self.b_mock.set_block_epoch(self.current_epoch);
    }

    pub fn get_current_week(&mut self) -> Week {
        let mut week = 0;
        self.b_mock
            .execute_query(&self.mb_wrapper, |sc| {
                week = sc.get_current_week();
            })
            .assert_ok();

        week
    }

    pub fn call_unpause(&mut self) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_addr,
            &self.mb_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.unpause_endpoint();
            },
        )
    }

    pub fn call_add_project(
        &mut self,
        project_id: &[u8],
        project_owner: &Address,
        reward_token: &[u8],
        reward_supply: u64,
        start_week: Week,
        duration_weeks: Week,
        lkmex_rewards_percentage: u64,
    ) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_addr,
            &self.mb_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.add_project(
                    managed_buffer!(project_id),
                    managed_address!(project_owner),
                    managed_token_id!(reward_token),
                    managed_biguint!(reward_supply),
                    start_week,
                    duration_weeks,
                    lkmex_rewards_percentage,
                );
            },
        )
    }

    pub fn call_remove_project(&mut self, project_id: &[u8]) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_addr,
            &self.mb_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.remove_project(managed_buffer!(project_id));
            },
        )
    }

    pub fn call_clear_expired_projects(&mut self) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_addr,
            &self.mb_wrapper,
            &rust_biguint!(0),
            |sc| {
                let _ = sc.clear_expired_projects();
            },
        )
    }

    pub fn get_all_project_ids(&mut self) -> Vec<Vec<u8>> {
        let mut all_ids = Vec::new();

        self.b_mock
            .execute_query(&self.mb_wrapper, |sc| {
                let result = sc.get_all_project_ids();

                for id in &result.to_vec() {
                    all_ids.push(id.to_boxed_bytes().as_slice().to_vec());
                }
            })
            .assert_ok();

        all_ids
    }

    pub fn get_project_by_id(&mut self, proj_id: &[u8]) -> (Vec<u8>, u64, u64, Week, Week) {
        let mut token = Vec::new();
        let mut reward_amount = 0;
        let mut lkmex_rewards_supply = 0;
        let mut start_week = 0;
        let mut duration = 0;

        self.b_mock
            .execute_query(&self.mb_wrapper, |sc| {
                let result = sc.get_project_by_id(managed_buffer!(proj_id));
                let (first, second, third, fourth, fifth) = result.into_tuple();
                token = first.to_boxed_bytes().as_slice().to_vec();
                reward_amount = second.to_u64().unwrap();
                lkmex_rewards_supply = third.to_u64().unwrap();
                start_week = fourth;
                duration = fifth;
            })
            .assert_ok();

        (
            token,
            reward_amount,
            lkmex_rewards_supply,
            start_week,
            duration,
        )
    }

    pub fn call_add_rewards_checkpoint(
        &mut self,
        week: Week,
        total_delegation_supply: u64,
        total_lkmex_staked: u64,
    ) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_addr,
            &self.mb_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.add_rewards_checkpoint(
                    week,
                    managed_biguint!(total_delegation_supply),
                    managed_biguint!(total_lkmex_staked),
                );
            },
        )
    }

    pub fn call_deposit_rewards(
        &mut self,
        caller: &Address,
        project_id: &[u8],
        token_id: &[u8],
        amount: u64,
    ) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            caller,
            &self.mb_wrapper,
            token_id,
            0,
            &rust_biguint!(amount),
            |sc| {
                sc.deposit_rewards(managed_buffer!(project_id));
            },
        )
    }

    pub fn call_claim_rewards(
        &mut self,
        caller: &Address,
        week: Week,
        user_delegation_supply: u64,
        user_lkmex_staked: u64,
        signature: &[u8; ED25519_SIGNATURE_BYTE_LEN],
    ) -> TxResult {
        self.b_mock
            .execute_tx(caller, &self.mb_wrapper, &rust_biguint!(0), |sc| {
                let mut args = MultiValueEncoded::new();
                args.push(
                    (
                        week,
                        managed_biguint!(user_delegation_supply),
                        managed_biguint!(user_lkmex_staked),
                        signature.into(),
                    )
                        .into(),
                );

                sc.claim_rewards(args);
            })
    }

    pub fn call_claim_rewards_multiple(
        &mut self,
        caller: &Address,
        args: &[(Week, u64, u64, &[u8; ED25519_SIGNATURE_BYTE_LEN])],
    ) -> TxResult {
        self.b_mock
            .execute_tx(caller, &self.mb_wrapper, &rust_biguint!(0), |sc| {
                let mut encoded_args = MultiValueEncoded::new();
                for arg in args {
                    let (week, user_delegation_supply, user_lkmex_staked, signature) = *arg;

                    encoded_args.push(
                        (
                            week,
                            managed_biguint!(user_delegation_supply),
                            managed_biguint!(user_lkmex_staked),
                            signature.into(),
                        )
                            .into(),
                    );
                }

                sc.claim_rewards(encoded_args);
            })
    }

    pub fn get_user_claimable_weeks(&mut self, user_addr: &Address) -> Vec<Week> {
        let mut weeks = Vec::new();

        self.b_mock
            .execute_query(&self.mb_wrapper, |sc| {
                let result = sc.get_user_claimable_weeks(managed_address!(user_addr));

                for week in &result.to_vec() {
                    weeks.push(week);
                }
            })
            .assert_ok();

        weeks
    }

    pub fn get_pretty_rewards(
        &mut self,
        week: Week,
        user_delegation_amount: u64,
        user_lkmex_staked_amount: u64,
    ) -> Vec<(Vec<u8>, Vec<u8>, u64)> {
        let mut rewards = Vec::new();

        self.b_mock
            .execute_query(&self.mb_wrapper, |sc| {
                let result = sc.get_rewards_for_week_pretty(
                    week,
                    managed_biguint!(user_delegation_amount),
                    managed_biguint!(user_lkmex_staked_amount),
                );

                for rew in result {
                    let (proj_id, token, amount) = rew.into_tuple();
                    let raw_id = proj_id.to_boxed_bytes().as_slice().to_vec();
                    let raw_token = token.to_boxed_bytes().as_slice().to_vec();
                    let raw_amount = amount.to_u64().unwrap();

                    rewards.push((raw_id, raw_token, raw_amount));
                }
            })
            .assert_ok();

        rewards
    }
}
