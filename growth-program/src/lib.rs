#![no_std]

use week_timekeeping::Week;

multiversx_sc::imports!();

pub mod price_query;
pub mod project;
pub mod rewards;
pub mod validation;

pub const MAX_PERCENTAGE: u32 = 100_000;
pub const WEEK_IN_SECONDS: u64 = 7 * 24 * 60 * 60;
pub const PRECISION: u64 = 1_000_000_000_000_000_000;

pub const DEFAULT_MIN_REWARDS_PERIOD: Week = 26;
pub const DEFAULT_MIN_WEEKLY_REWARDS_DOLLARS_VALUE: u64 = 1_000;
pub const USDC_DECIMALS: u32 = 6;

#[multiversx_sc::contract]
pub trait GrowthProgram:
    project::ProjectsModule
    + rewards::deposit::DepositRewardsModule
    + rewards::common_rewards::CommonRewardsModule
    + price_query::PriceQueryModule
    + validation::ValidationModule
    + week_timekeeping::WeekTimekeepingModule
    + utils::UtilsModule
{
    /// Arguments:
    /// min_energy_per_reward_dollar: Scaled to PRECISION const.
    /// alpha: Percentage, scaled to MAX_PERCENTAGE const.
    /// signer: Public key of the signer, used to verify user claims
    #[init]
    fn init(
        &self,
        _min_energy_per_reward_dollar: BigUint,
        _alpha: BigUint,
        signer: ManagedAddress,
        router_address: ManagedAddress,
        safe_price_pair: ManagedAddress,
        usdc_token_id: TokenIdentifier,
        wegld_token_id: TokenIdentifier,
    ) {
        self.require_sc_address(&router_address);
        self.require_sc_address(&safe_price_pair);
        self.require_valid_token_id(&usdc_token_id);
        self.require_valid_token_id(&wegld_token_id);

        self.router_address().set(router_address);
        self.safe_price_pair().set(safe_price_pair);
        self.usdc_token_id().set(usdc_token_id);
        self.wegld_token_id().set(wegld_token_id);

        self.change_signer(signer);

        self.min_rewards_period().set(DEFAULT_MIN_REWARDS_PERIOD);

        let default_min_weekly_rewards_value =
            BigUint::from(DEFAULT_MIN_WEEKLY_REWARDS_DOLLARS_VALUE)
                * BigUint::from(10u32).pow(USDC_DECIMALS);
        self.min_weekly_rewards_value()
            .set(default_min_weekly_rewards_value);

        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set(current_epoch);
    }

    #[endpoint]
    fn upgrade(&self) {}
}
