use core::ops::Index;

use anchor_lang::prelude::ProgramError;
use core::convert::TryFrom;
use solana_maths::{Rate, TryAdd, TryDiv, TryMul};
use strum::IntoEnumIterator;

use crate::{
    errors::ErrorCode,
    instructions::Reserves,
    rebalance::assets::{Provider, ReturnCalculator},
    state::{Allocation, StrategyType},
};

use super::BackendContainer;

impl BackendContainer<Reserves> {
    fn calculate_weights_max_yield(&self) -> Result<BackendContainer<Rate>, ProgramError> {
        self.into_iter()
            .max_by(|(_prov_x, alloc_x), (_prov_y, alloc_y)| {
                // TODO: can we remove the unwrap() in any way?
                self.compare(*alloc_x, *alloc_y).unwrap()
            })
            .map(|(max_yielding_provider, _a)| {
                self.apply(|provider, _v| {
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
            // TODO: error code?
            .map_err(|_| ProgramError::Custom(0))
            .and_then(|num_assets| Rate::from_percent(num_assets).try_mul(100))
            .and_then(|r| Rate::one().try_div(r))
            .map(|equal_allocation| self.apply(|_, _| equal_allocation))
    }

    pub fn calculate_weights(
        &self,
        stype: StrategyType,
    ) -> Result<BackendContainer<Rate>, ProgramError> {
        match stype {
            StrategyType::MaxYield => self.calculate_weights_max_yield(),
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
