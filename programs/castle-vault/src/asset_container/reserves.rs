use core::{convert::TryFrom, ops::Index};
use std::cmp::Ordering;

use itertools::Itertools;
use solana_maths::{Rate, TryAdd, TryDiv, TryMul, TrySub};

use anchor_lang::prelude::ProgramError;

use crate::{
    errors::ErrorCode,
    reserves::{Provider, Reserves, ReturnCalculator},
    state::StrategyType,
};

use super::AssetContainer;

pub fn compare(
    lhs: &impl ReturnCalculator,
    rhs: &impl ReturnCalculator,
) -> Result<Ordering, ProgramError> {
    Ok(lhs.calculate_return(0)?.cmp(&rhs.calculate_return(0)?))
}

impl AssetContainer<Reserves> {
    fn calculate_weights_max_yield(
        &self,
        allocation_cap_pct: u8,
    ) -> Result<AssetContainer<Rate>, ProgramError> {
        self.into_iter()
            .filter(|(_, r)| !r.is_none())
            .map(|(p, r)| (p, r.unwrap()))
            .sorted_unstable_by(|(_, alloc_y), (_, alloc_x)| {
                // TODO: can we remove the expect() in any way?
                compare(*alloc_x, *alloc_y).expect("Could not successfully compare allocations")
            })
            .try_fold(
                (AssetContainer::<Rate>::default(), Rate::one()),
                |(mut strategy_weights, remaining_weight), (provider, _)| {
                    let target_weight =
                        remaining_weight.min(Rate::from_percent(allocation_cap_pct));
                    strategy_weights[provider] = Some(target_weight);
                    match remaining_weight.try_sub(target_weight) {
                        Ok(r) => Ok((strategy_weights, r)),
                        Err(e) => Err(e),
                    }
                },
            )
            .map(|(r, _)| r)
    }

    fn calculate_weights_equal(&self) -> Result<AssetContainer<Rate>, ProgramError> {
        u8::try_from(self.valid_len())
            .map_err(|_| ErrorCode::StrategyError.into())
            .and_then(|num_assets| Rate::from_percent(num_assets).try_mul(100))
            .and_then(|r| Rate::one().try_div(r))
            .map(|equal_allocation| self.apply(|_, v| {
                match v {
                    Some(_) => Some(equal_allocation),
                    None => None,
                }
            }))
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
        weights: &dyn Index<Provider, Output = Option<Rate>>,
        allocations: &dyn Index<Provider, Output = Option<u64>>,
    ) -> Result<Rate, ProgramError> {
        self.into_iter()
            .filter(|(_, r)| !r.is_none())
            .map(|(p, r)| (p, r.unwrap()))
            .map(|(p, r)| {
                r.calculate_return(allocations[p].unwrap())
                    .and_then(|r| weights[p].unwrap().try_mul(r))
            })
            .try_fold(Rate::zero(), |acc, r| acc.try_add(r?))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::reserves::MockReturnCalculator;
    use solana_maths::Rate;

    // TODO
    #[test]
    fn test_get_apr() {}

    #[test]
    fn test_compare() {
        let mut mock_rc1 = MockReturnCalculator::new();
        mock_rc1
            .expect_calculate_return()
            .return_const(Ok(Rate::from_percent(10)));

        let mut mock_rc2 = MockReturnCalculator::new();
        mock_rc2
            .expect_calculate_return()
            .return_const(Ok(Rate::from_percent(20)));

        assert_eq!(compare(&mock_rc1, &mock_rc2), Ok(Ordering::Less));
    }
}
