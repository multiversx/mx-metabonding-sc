elrond_wasm::imports!();

use crate::rewards::Week;

#[elrond_wasm::module]
pub trait MathModule {
    fn calculate_ratio(&self, amount: &BigUint, part: &BigUint, total: &BigUint) -> BigUint {
        if total == &0 {
            return BigUint::zero();
        }

        &(amount * part) / total
    }

    #[inline]
    fn is_in_range(&self, value: Week, min: Week, max: Week) -> bool {
        (min..=max).contains(&value)
    }
}
