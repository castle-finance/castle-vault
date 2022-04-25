use core::ops::Index;

use anchor_lang::prelude::ProgramError;
use solana_maths::{Rate, TryAdd};
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

    pub fn calculate_weights(
        &self,
        stype: StrategyType,
    ) -> Result<BackendContainer<Rate>, ProgramError> {
        match stype {
            StrategyType::MaxYield => self.calculate_weights_max_yield(),
            StrategyType::EqualAllocation => todo!(),
        }
    }

    pub fn get_apr(
        &self,
        weights: &dyn Index<Provider, Output = Rate>,
        allocations: &dyn Index<Provider, Output = Allocation>,
    ) -> Result<Rate, ProgramError> {
        Provider::iter()
            .map(|p| {
                solana_maths::TryMul::try_mul(
                    weights[p],
                    self[p].calculate_return(allocations[p].value)?,
                )
            })
            .collect::<Result<Vec<_>, ProgramError>>()?
            .iter()
            .try_fold(Rate::zero(), |acc, r| acc.try_add(*r))
    }
}
