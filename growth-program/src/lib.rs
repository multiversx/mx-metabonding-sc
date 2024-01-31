#![no_std]

multiversx_sc::imports!();

#[multiversx_sc::contract]
pub trait GrowthProgram {
    #[init]
    fn init(&self) {}
}
