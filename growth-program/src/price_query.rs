use pair::safe_price_view::ProxyTrait as _;
use router::factory::ProxyTrait as _;

use crate::WEEK_IN_SECONDS;

multiversx_sc::imports!();

pub enum PairQueryResponse<M: ManagedTypeApi> {
    WegldIntermediary {
        token_to_wegld_pair: ManagedAddress<M>,
        wegld_to_usdc_pair: ManagedAddress<M>,
    },
    TokenToUsdc(ManagedAddress<M>),
}

#[multiversx_sc::module]
pub trait PriceQueryModule {
    fn get_dollar_value(&self, token_id: TokenIdentifier, amount: BigUint) -> BigUint {
        let pair_query_response = self.get_pair_to_query(token_id.clone());
        match pair_query_response {
            PairQueryResponse::WegldIntermediary {
                token_to_wegld_pair,
                wegld_to_usdc_pair,
            } => {
                let wegld_token_id = self.wegld_token_id().get();
                let wegld_price = self.call_get_safe_price(token_to_wegld_pair, token_id, amount);

                self.call_get_safe_price(wegld_to_usdc_pair, wegld_token_id, wegld_price)
            }
            PairQueryResponse::TokenToUsdc(pair_addr) => {
                self.call_get_safe_price(pair_addr, token_id, amount)
            }
        }
    }

    fn get_pair_to_query(&self, token_id: TokenIdentifier) -> PairQueryResponse<Self::Api> {
        let wegld_token_id = self.wegld_token_id().get();
        let usdc_token_id = self.usdc_token_id().get();
        let router_address = self.router_address().get();
        let token_to_wegld_pair = self.call_get_pair(
            router_address.clone(),
            token_id.clone(),
            wegld_token_id.clone(),
        );

        if !token_to_wegld_pair.is_zero() {
            let wegld_to_usdc_pair =
                self.call_get_pair(router_address, wegld_token_id, usdc_token_id);
            require!(
                !wegld_to_usdc_pair.is_zero(),
                "Invalid WEGLD-USDC pair address from router"
            );

            return PairQueryResponse::WegldIntermediary {
                token_to_wegld_pair,
                wegld_to_usdc_pair,
            };
        }

        let token_to_usdc_pair = self.call_get_pair(router_address, token_id, usdc_token_id);
        require!(
            !token_to_usdc_pair.is_zero(),
            "Invalid TOKEN-USDC pair address from router"
        );

        PairQueryResponse::TokenToUsdc(token_to_usdc_pair)
    }

    fn call_get_pair(
        &self,
        router_address: ManagedAddress,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> ManagedAddress {
        self.router_proxy(router_address)
            .get_pair(first_token_id, second_token_id)
            .execute_on_dest_context()
    }

    fn call_get_safe_price(
        &self,
        pair_address: ManagedAddress,
        token_id: TokenIdentifier,
        amount: BigUint,
    ) -> BigUint {
        let input_payment = EsdtTokenPayment::new(token_id, 0, amount);
        let safe_price_pair = self.safe_price_pair().get();
        let price_payment: EsdtTokenPayment = self
            .pair_proxy(safe_price_pair)
            .get_safe_price_by_timestamp_offset(pair_address, WEEK_IN_SECONDS, input_payment)
            .execute_on_dest_context();

        price_payment.amount
    }

    #[proxy]
    fn router_proxy(&self, sc_address: ManagedAddress) -> router::Proxy<Self::Api>;

    #[proxy]
    fn pair_proxy(&self, sc_address: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[storage_mapper("routerAddress")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("safePricePair")]
    fn safe_price_pair(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("usdcTokenId")]
    fn usdc_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("wegldTokenId")]
    fn wegld_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
