use core::ops::Index;

use anchor_lang::prelude::ProgramError;
use core::convert::TryFrom;
use itertools::Itertools;
use solana_maths::{Rate, TryAdd, TryDiv, TryMul, TrySub};
use strum::IntoEnumIterator;

use crate::{
    errors::ErrorCode,
    reserves::{Provider, Reserves, ReturnCalculator},
    state::{Allocation, StrategyType},
};

use super::BackendContainer;

impl BackendContainer<Reserves> {
    fn calculate_weights_max_yield(
        &self,
        allocation_cap_pct: u8,
    ) -> Result<BackendContainer<Rate>, ProgramError> {
        // TODO is there a cleaner way to do this with combinators?
        let sorted: Vec<(Provider, &Reserves)> = self
            .into_iter()
            .sorted_unstable_by(|(_, alloc_x), (_, alloc_y)| {
                // TODO: can we remove the unwrap() in any way?
                self.compare(*alloc_x, *alloc_y).unwrap()
            })
            .collect();

        let cap = Rate::from_percent(allocation_cap_pct);
        let mut remaining_weight = Rate::one();
        let mut strategy_weights = BackendContainer::<Rate>::default();
        for (p, _) in sorted {
            let target_weight = remaining_weight.min(cap);
            remaining_weight = remaining_weight.try_sub(target_weight)?;
            strategy_weights[p] = target_weight;
        }
        Ok(strategy_weights)
    }

    fn calculate_weights_equal(&self) -> Result<BackendContainer<Rate>, ProgramError> {
        u8::try_from(self.len())
            .map_err(|_| ErrorCode::StrategyError.into())
            .and_then(|num_assets| Rate::from_percent(num_assets).try_mul(100))
            .and_then(|r| Rate::one().try_div(r))
            .map(|equal_allocation| self.apply(|_, _| equal_allocation))
    }

    pub fn calculate_weights(
        &self,
        strategy_type: StrategyType,
        allocation_cap_pct: u8,
    ) -> Result<BackendContainer<Rate>, ProgramError> {
        match strategy_type {
            StrategyType::MaxYield => self.calculate_weights_max_yield(allocation_cap_pct),
            StrategyType::EqualAllocation => self.calculate_weights_equal(),
        }
    }

    pub fn get_apr(
        &self,
        weights: &dyn Index<Provider, Output = Rate>,
        allocations: &dyn Index<Provider, Output = Allocation>,
    ) -> Result<Rate, ProgramError> {
        Provider::iter()
            .map(|p| {
                self[p]
                    .calculate_return(allocations[p].value)
                    .and_then(|r| weights[p].try_mul(r))
            })
            .try_fold(Rate::zero(), |acc, r| acc.try_add(r?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_weights_equal() {}

    #[test]
    fn test_calculate_weights_max_yield() {}

    #[test]
    fn test_get_apr() {}
}
