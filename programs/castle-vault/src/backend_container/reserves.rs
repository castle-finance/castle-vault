use core::ops::Index;

use anchor_lang::prelude::ProgramError;
use core::convert::TryFrom;
use solana_maths::{Rate, TryAdd, TryDiv, TryMul};
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
        // TODO add allocation cap

        // let mut sorted_pools: Vec<Provider> = Provider::iter().collect();
        // sorted_pools.sort_unstable_by(|x, y| self.compare(&assets[*y], &assets[*x]).unwrap());

        // let cap = Rate::from_percent(allocation_cap_pct);
        // let mut remaining_weight = Rate::one();
        // let mut strategy_weights = StrategyWeights::default();
        // for p in sorted_pools {
        //     let target_weight = remaining_weight.min(cap);
        //     remaining_weight = remaining_weight.try_sub(target_weight)?;
        //     strategy_weights[p] = target_weight;
        // }

        // Ok(strategy_weights)

        self.into_iter()
            .max_by(|(_, alloc_x), (_, alloc_y)| {
                // TODO: can we remove the unwrap() in any way?
                self.compare(*alloc_x, *alloc_y).unwrap()
            })
            .map(|(max_yielding_provider, _a)| {
                self.apply(|provider, _| {
                    if provider == max_yielding_provider {
                        Rate::one()
                    } else {
                        Rate::zero()
                    }
                })
            })
            // TODO make this error handling more granular and informative
            .ok_or(ErrorCode::StrategyError)
            .map_err(Into::into)
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
