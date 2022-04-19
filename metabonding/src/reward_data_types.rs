elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{project::ProjectId, validation::Signature};

pub type Week = usize;
pub type PaymentsVec<M> = ManagedVec<M, EsdtTokenPayment<M>>;
pub type PrettyRewards<M> =
    MultiValueEncoded<M, MultiValue3<ProjectId<M>, TokenIdentifier<M>, BigUint<M>>>;
pub type ClaimArgPair<M> = MultiValue4<Week, BigUint<M>, BigUint<M>, Signature<M>>;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct RewardsCheckpoint<M: ManagedTypeApi> {
    pub total_delegation_supply: BigUint<M>,
    pub total_lkmex_staked: BigUint<M>,
}

pub struct WeeklyRewards<M: ManagedTypeApi> {
    payments: PaymentsVec<M>,
}

impl<M: ManagedTypeApi> WeeklyRewards<M> {
    #[inline]
    pub fn new(payments: PaymentsVec<M>) -> Self {
        Self { payments }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.payments.is_empty()
    }

    pub fn add_rewards(&mut self, other: WeeklyRewards<M>) {
        let payments_len = self.payments.len();
        for i in 0..payments_len {
            let mut payment = self.payments.get_mut(i);
            payment.amount += other.payments.get(i).amount;
        }
    }

    pub fn get_trimmed_payments(
        self,
        project_ids: &mut ManagedVec<M, ProjectId<M>>,
    ) -> ManagedVec<M, EsdtTokenPayment<M>> {
        let mut proj_index = 0;
        let mut trimmed_payments = ManagedVec::new();
        for p in &self.payments {
            if p.amount > 0 {
                trimmed_payments.push(p);
                proj_index += 1;
            } else {
                project_ids.remove(proj_index);
            }
        }

        trimmed_payments
    }
}
