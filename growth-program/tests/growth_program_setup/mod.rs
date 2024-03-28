#![allow(deprecated)]

use energy_factory::SimpleLockEnergy;
use growth_program::{
    project::{ProjectId, ProjectsModule},
    rewards::{
        claim::ClaimRewardsModule,
        claim_types::{ClaimType, LockOption},
        deposit::DepositRewardsModule,
        energy::EnergyModule,
        week_timekeeping::{Epoch, MONDAY_19_02_2024_GMT_TIMESTAMP},
    },
    GrowthProgram, Timestamp, DEFAULT_MIN_REWARDS_PERIOD, MAX_PERCENTAGE, PRECISION,
    WEEK_IN_SECONDS,
};
use multiversx_sc::{
    api::ManagedTypeApi,
    codec::multi_types::OptionalValue,
    hex_literal,
    storage::mappers::StorageTokenWrapper,
    types::{Address, EsdtLocalRole, ManagedByteArray, MultiValueEncoded},
};
use multiversx_sc_modules::pause::PauseModule;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper, TxResult},
    DebugApi,
};
use pair_mock::PairMock;
use router_mock::RouterMock;
use simple_lock::{locked_token::LockedTokenModule, SimpleLock};

// associated private key - used for generating the signatures (please don't steal my funds)
// 3eb200ef228e593d49a522f92587889fedfc091629d175873b64ca0ab3b4514d52773868c13654355cca16adb389b09201fabf5d9d4b795ebbdae5b361b46f20
pub static SIGNER_ADDRESS: [u8; 32] =
    hex_literal::hex!("52773868c13654355cca16adb389b09201fabf5d9d4b795ebbdae5b361b46f20");

pub static FIRST_PROJ_TOKEN: &[u8] = b"PROJ-123456";
pub static SECOND_PROJ_TOKEN: &[u8] = b"COOL-123456";
pub const TOTAL_FIRST_PROJ_TOKENS: u64 = 1_000_000_000;
pub const TOTAL_SECOND_PROJ_TOKENS: u64 = 2_000_000_000;
pub const DEFAULT_DECIMALS: u32 = 18;

pub const EPOCHS_IN_YEAR: Epoch = 360;
pub const EPOCHS_IN_WEEK: Epoch = 7;

pub static LOCK_OPTIONS: &[u64] = &[EPOCHS_IN_YEAR, 2 * EPOCHS_IN_YEAR, 4 * EPOCHS_IN_YEAR]; // 1, 2 or 4 years
pub static PENALTY_PERCENTAGES: &[u64] = &[4_000, 6_000, 8_000];

pub static USDC_TOKEN_ID: &[u8] = b"USDC-123456";
pub static LOCKED_TOKEN_ID: &[u8] = b"LOCKED-123456";
pub static BASE_ASSET_TOKEN_ID: &[u8] = b"MEX-123456";
pub static LEGACY_LOCKED_TOKEN_ID: &[u8] = b"LEGACY-123456";
pub static ENERGY_TOKEN_ID: &[u8] = b"ENERGY-123456";
pub static WEGLD_TOKEN_ID: &[u8] = b"WEGLD-123456";

pub const DEFAULT_ENERGY_PER_DOLLAR: u64 = 5;

pub const FIRST_USER_LOCKED_TOKENS: u64 = 1_000;
pub const SECOND_USER_LOCKED_TOKENS: u64 = 2_000;

pub struct GrowthProgramSetup<
    GrowthProgramBuilder,
    PairMockBuilder,
    RouterMockBuilder,
    SimpleLockBuilder,
    EnergyFactoryBuilder,
> where
    GrowthProgramBuilder: 'static + Copy + Fn() -> growth_program::ContractObj<DebugApi>,
    PairMockBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
    RouterMockBuilder: 'static + Copy + Fn() -> router_mock::ContractObj<DebugApi>,
    SimpleLockBuilder: 'static + Copy + Fn() -> simple_lock::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_addr: Address,
    pub first_project_owner: Address,
    pub second_project_owner: Address,
    pub first_user_addr: Address,
    pub second_user_addr: Address,
    pub gp_wrapper: ContractObjWrapper<growth_program::ContractObj<DebugApi>, GrowthProgramBuilder>,
    pub pair_wrapper: ContractObjWrapper<pair_mock::ContractObj<DebugApi>, PairMockBuilder>,
    pub router_wrapper: ContractObjWrapper<router_mock::ContractObj<DebugApi>, RouterMockBuilder>,
    pub simple_lock_wrapper:
        ContractObjWrapper<simple_lock::ContractObj<DebugApi>, SimpleLockBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryBuilder>,
    pub current_timestamp: Timestamp,
    pub current_epoch: Epoch,
}

impl<
        GrowthProgramBuilder,
        PairMockBuilder,
        RouterMockBuilder,
        SimpleLockBuilder,
        EnergyFactoryBuilder,
    >
    GrowthProgramSetup<
        GrowthProgramBuilder,
        PairMockBuilder,
        RouterMockBuilder,
        SimpleLockBuilder,
        EnergyFactoryBuilder,
    >
where
    GrowthProgramBuilder: 'static + Copy + Fn() -> growth_program::ContractObj<DebugApi>,
    PairMockBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
    RouterMockBuilder: 'static + Copy + Fn() -> router_mock::ContractObj<DebugApi>,
    SimpleLockBuilder: 'static + Copy + Fn() -> simple_lock::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    pub fn new(
        gp_builder: GrowthProgramBuilder,
        pair_builder: PairMockBuilder,
        router_builder: RouterMockBuilder,
        simple_lock_builder: SimpleLockBuilder,
        energy_factory_builder: EnergyFactoryBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let first_project_owner = b_mock.create_user_account(&rust_zero);
        let second_project_owner = b_mock.create_user_account(&rust_zero);

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
            FIRST_PROJ_TOKEN,
            &StaticMethods::get_first_token_full_amount(),
        );
        b_mock.set_esdt_balance(
            &second_project_owner,
            SECOND_PROJ_TOKEN,
            &StaticMethods::get_second_token_full_amount(),
        );

        let current_epoch = 5;
        b_mock.set_block_epoch(current_epoch);

        // Pair init

        let pair_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            pair_builder,
            "pair wasm path",
        );
        b_mock
            .execute_tx(&owner_addr, &pair_wrapper, &rust_zero, |sc| {
                sc.init(managed_token_id!(USDC_TOKEN_ID));
            })
            .assert_ok();

        // Router init

        let router_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            router_builder,
            "router wasm path",
        );
        b_mock
            .execute_tx(&owner_addr, &router_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_address!(pair_wrapper.address_ref()),
                    managed_token_id!(USDC_TOKEN_ID),
                );
            })
            .assert_ok();

        // simple lock init

        let simple_lock_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            simple_lock_builder,
            "simple lock wasm path",
        );
        b_mock
            .execute_tx(&owner_addr, &simple_lock_wrapper, &rust_zero, |sc| {
                sc.init();
                sc.locked_token()
                    .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            })
            .assert_ok();

        b_mock.set_esdt_local_roles(
            simple_lock_wrapper.address_ref(),
            LOCKED_TOKEN_ID,
            &[
                EsdtLocalRole::NftCreate,
                EsdtLocalRole::NftAddQuantity,
                EsdtLocalRole::NftBurn,
            ],
        );

        // energy factory init

        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            energy_factory_builder,
            "energy factory wasm path",
        );
        b_mock
            .execute_tx(&owner_addr, &energy_factory_wrapper, &rust_zero, |sc| {
                let mut lock_options = MultiValueEncoded::new();
                for (option, penalty) in LOCK_OPTIONS.iter().zip(PENALTY_PERCENTAGES.iter()) {
                    lock_options.push((*option, *penalty).into());
                }

                // we don't test migration here
                sc.init(
                    managed_token_id!(BASE_ASSET_TOKEN_ID),
                    managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                    managed_address!(pair_wrapper.address_ref()),
                    0,
                    lock_options,
                );

                sc.locked_token()
                    .set_token_id(managed_token_id!(ENERGY_TOKEN_ID));
                sc.set_paused(false);
            })
            .assert_ok();

        // set energy factory roles
        b_mock.set_esdt_local_roles(
            energy_factory_wrapper.address_ref(),
            BASE_ASSET_TOKEN_ID,
            &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
        );
        b_mock.set_esdt_local_roles(
            energy_factory_wrapper.address_ref(),
            ENERGY_TOKEN_ID,
            &[
                EsdtLocalRole::NftCreate,
                EsdtLocalRole::NftAddQuantity,
                EsdtLocalRole::NftBurn,
                EsdtLocalRole::Transfer,
            ],
        );
        b_mock.set_esdt_local_roles(
            energy_factory_wrapper.address_ref(),
            LEGACY_LOCKED_TOKEN_ID,
            &[EsdtLocalRole::NftBurn],
        );

        // users lock tokens
        b_mock.set_esdt_balance(
            &first_user_addr,
            BASE_ASSET_TOKEN_ID,
            &rust_biguint!(FIRST_USER_LOCKED_TOKENS),
        );
        b_mock.set_esdt_balance(
            &second_user_addr,
            BASE_ASSET_TOKEN_ID,
            &rust_biguint!(SECOND_USER_LOCKED_TOKENS),
        );

        b_mock
            .execute_esdt_transfer(
                &first_user_addr,
                &energy_factory_wrapper,
                BASE_ASSET_TOKEN_ID,
                0,
                &rust_biguint!(FIRST_USER_LOCKED_TOKENS),
                |sc| {
                    sc.lock_tokens_endpoint(LOCK_OPTIONS[0], OptionalValue::None);
                },
            )
            .assert_ok();

        b_mock
            .execute_esdt_transfer(
                &second_user_addr,
                &energy_factory_wrapper,
                BASE_ASSET_TOKEN_ID,
                0,
                &rust_biguint!(SECOND_USER_LOCKED_TOKENS),
                |sc| {
                    sc.lock_tokens_endpoint(LOCK_OPTIONS[0], OptionalValue::None);
                },
            )
            .assert_ok();

        b_mock.set_block_timestamp(MONDAY_19_02_2024_GMT_TIMESTAMP);

        // Growth Program SC init

        let gp_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            gp_builder,
            "growth program wasm path",
        );
        b_mock
            .execute_tx(&owner_addr, &gp_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_biguint!(10) * PRECISION,
                    managed_biguint!(25) * MAX_PERCENTAGE / 100u32, // 25%
                    managed_biguint!(25) * MAX_PERCENTAGE / 100u32, // 25%
                    managed_address!(router_wrapper.address_ref()),
                    managed_address!(pair_wrapper.address_ref()),
                    managed_address!(energy_factory_wrapper.address_ref()),
                    managed_address!(simple_lock_wrapper.address_ref()),
                    managed_address!(simple_lock_wrapper.address_ref()), // unused
                    managed_token_id!(USDC_TOKEN_ID),
                    managed_token_id!(WEGLD_TOKEN_ID),
                );

                sc.first_week_reward_dollars_per_energy()
                    .set(managed_biguint!(DEFAULT_ENERGY_PER_DOLLAR) * PRECISION * PRECISION);

                sc.set_paused(false);
            })
            .assert_ok();

        GrowthProgramSetup {
            b_mock,
            owner_addr,
            first_project_owner,
            second_project_owner,
            first_user_addr,
            second_user_addr,
            gp_wrapper,
            pair_wrapper,
            router_wrapper,
            simple_lock_wrapper,
            energy_factory_wrapper,
            current_timestamp: MONDAY_19_02_2024_GMT_TIMESTAMP,
            current_epoch,
        }
    }

    pub fn add_projects(&mut self) {
        let first_proj_owner = self.first_project_owner.clone();
        let second_proj_owner = self.second_project_owner.clone();

        self.b_mock
            .execute_tx(
                &self.owner_addr,
                &self.gp_wrapper,
                &rust_biguint!(0),
                |sc| {
                    let first_proj_id = sc.add_project(managed_address!(&first_proj_owner));
                    let second_proj_id = sc.add_project(managed_address!(&second_proj_owner));
                    let last_project_id = sc.last_project_id().get();

                    assert_eq!(first_proj_id, 1);
                    assert_eq!(second_proj_id, 2);
                    assert_eq!(last_project_id, 2);
                },
            )
            .assert_ok();
    }

    pub fn set_first_week_apr(&mut self, apr: u64) {
        self.b_mock
            .execute_tx(
                &self.owner_addr,
                &self.gp_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.set_first_week_apr(apr.into());
                },
            )
            .assert_ok();
    }

    pub fn deposit_rewards(&mut self) {
        let first_proj_owner = self.first_project_owner.clone();
        let second_proj_owner = self.second_project_owner.clone();

        self.b_mock
            .execute_esdt_transfer(
                &first_proj_owner,
                &self.gp_wrapper,
                FIRST_PROJ_TOKEN,
                0,
                &StaticMethods::get_first_token_full_amount(),
                |sc| {
                    let signer_addr = managed_address!(&Address::from(&SIGNER_ADDRESS));

                    sc.deposit_initial_rewards(1, 2, 2 + DEFAULT_MIN_REWARDS_PERIOD, signer_addr);
                },
            )
            .assert_ok();

        self.b_mock
            .execute_esdt_transfer(
                &second_proj_owner,
                &self.gp_wrapper,
                SECOND_PROJ_TOKEN,
                0,
                &StaticMethods::get_second_token_full_amount(),
                |sc| {
                    let signer_addr = managed_address!(&Address::from(&SIGNER_ADDRESS));

                    sc.deposit_initial_rewards(2, 2, 2 + DEFAULT_MIN_REWARDS_PERIOD, signer_addr);
                },
            )
            .assert_ok();
    }

    pub fn advance_week(&mut self) {
        self.current_timestamp += WEEK_IN_SECONDS;
        self.b_mock.set_block_timestamp(self.current_timestamp);
        self.current_epoch += 7;
        self.b_mock.set_block_epoch(self.current_epoch);
    }

    pub fn claim(
        &mut self,
        user: &Address,
        project_id: ProjectId,
        lock_option: LockOption,
        min_rewards: u64,
        signature: &[u8; 64],
    ) -> TxResult {
        self.b_mock
            .execute_tx(user, &self.gp_wrapper, &rust_biguint!(0), |sc| {
                let multi_value_arg = (
                    managed_buffer!(b"lala"),
                    ManagedByteArray::new_from_bytes(signature),
                )
                    .into();
                let _ = sc.claim_rewards(
                    project_id,
                    managed_biguint!(min_rewards),
                    ClaimType::Rewards(lock_option),
                    OptionalValue::Some(multi_value_arg),
                );
            })
    }
}

pub struct StaticMethods {}

impl StaticMethods {
    pub fn get_first_token_full_amount() -> num_bigint::BigUint {
        rust_biguint!(TOTAL_FIRST_PROJ_TOKENS) * rust_biguint!(10).pow(DEFAULT_DECIMALS)
    }

    pub fn get_second_token_full_amount() -> num_bigint::BigUint {
        rust_biguint!(TOTAL_SECOND_PROJ_TOKENS) * rust_biguint!(10).pow(DEFAULT_DECIMALS)
    }

    pub fn get_first_token_full_amount_managed<M: ManagedTypeApi>(
    ) -> multiversx_sc::types::BigUint<M> {
        managed_biguint!(TOTAL_FIRST_PROJ_TOKENS) * managed_biguint!(10).pow(DEFAULT_DECIMALS)
    }

    pub fn get_second_token_full_amount_managed<M: ManagedTypeApi>(
    ) -> multiversx_sc::types::BigUint<M> {
        managed_biguint!(TOTAL_SECOND_PROJ_TOKENS) * managed_biguint!(10).pow(DEFAULT_DECIMALS)
    }
}
