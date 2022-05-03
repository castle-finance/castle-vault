use core::ops::Index;

use anchor_lang::prelude::ProgramError;
use core::convert::TryFrom;
use itertools::Itertools;
use solana_maths::{Rate, TryAdd, TryDiv, TryMul, TrySub};
use strum::IntoEnumIterator;

use crate::{
    errors::ErrorCode,
    reserves::{Provider, Reserves, ReturnCalculator},
    state::StrategyType,
};

use super::AssetContainer;

impl AssetContainer<Reserves> {
    fn calculate_weights_max_yield(
        &self,
        allocation_cap_pct: u8,
    ) -> Result<AssetContainer<Rate>, ProgramError> {
        self.into_iter()
            .sorted_unstable_by(|(_, alloc_y), (_, alloc_x)| {
                // TODO: can we remove the expect() in any way?
                self.compare(*alloc_x, *alloc_y)
                    .expect("Could not successfully compare allocations")
            })
            .try_fold(
                (AssetContainer::<Rate>::default(), Rate::one()),
                |(mut strategy_weights, remaining_weight), (provider, _)| {
                    let target_weight =
                        remaining_weight.min(Rate::from_percent(allocation_cap_pct));
                    strategy_weights[provider] = target_weight;
                    match remaining_weight.try_sub(target_weight) {
                        Ok(r) => Ok((strategy_weights, r)),
                        Err(e) => Err(e),
                    }
                },
            )
            .map(|(r, _)| r)
    }

    fn calculate_weights_equal(&self) -> Result<AssetContainer<Rate>, ProgramError> {
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
    ) -> Result<AssetContainer<Rate>, ProgramError> {
        match strategy_type {
            StrategyType::MaxYield => self.calculate_weights_max_yield(allocation_cap_pct),
            StrategyType::EqualAllocation => self.calculate_weights_equal(),
        }
    }

    pub fn get_apr(
        &self,
        weights: &dyn Index<Provider, Output = Rate>,
        allocations: &dyn Index<Provider, Output = u64>,
    ) -> Result<Rate, ProgramError> {
        Provider::iter()
            .map(|p| {
                self[p]
                    .calculate_return(allocations[p])
                    .and_then(|r| weights[p].try_mul(r))
            })
            .try_fold(Rate::zero(), |acc, r| acc.try_add(r?))
    }
}

// TODO add tests
