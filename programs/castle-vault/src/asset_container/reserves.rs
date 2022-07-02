use core::{convert::TryFrom, ops::Index};
use std::cmp::Ordering;

use itertools::Itertools;
use solana_maths::{Rate, TryAdd, TryDiv, TryMul, TrySub};

use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    reserves::{Provider, Reserves, ReturnCalculator},
    state::StrategyType,
};

use super::AssetContainer;

pub fn compare(
    lhs: &impl ReturnCalculator,
    rhs: &impl ReturnCalculator,
) -> Result<Ordering> {
    Ok(lhs
        .calculate_return(0, 0)?
        .cmp(&rhs.calculate_return(0, 0)?))
}

impl AssetContainer<Reserves> {
    fn calculate_weights_max_yield(
        &self,
        allocation_cap_pct: u8,
    ) -> Result<AssetContainer<Rate>> {
        self.into_iter()
            .flat_map(|(p, r)| r.map(|v| (p, v)))
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
            .map(|(r, _)| r).map_err(|e| e.into())
    }

    fn calculate_weights_equal(&self) -> Result<AssetContainer<Rate>> {
        u8::try_from(self.len())
            .map_err(|_| ErrorCode::StrategyError)
            .and_then(|num_assets| Rate::from_percent(num_assets).try_mul(100).map_err(|_| ErrorCode::StrategyError))
            .and_then(|r| Rate::one().try_div(r).map_err(|_| ErrorCode::StrategyError))
            .map(|equal_allocation| self.apply(|_, v| v.map(|_| equal_allocation))).map_err(
                |e| e.into(),
            )
    }

    pub fn calculate_weights(
        &self,
        strategy_type: StrategyType,
        allocation_cap_pct: u8,
    ) -> Result<AssetContainer<Rate>> {
        match strategy_type {
            StrategyType::MaxYield => self.calculate_weights_max_yield(allocation_cap_pct),
            StrategyType::EqualAllocation => self.calculate_weights_equal(),
        }
    }

    pub fn get_apr(
        &self,
        weights: &dyn Index<Provider, Output = Option<Rate>>,
        new_allocations: &dyn Index<Provider, Output = Option<u64>>,
        actual_allocations: &dyn Index<Provider, Output = Option<u64>>,
    ) -> Result<Rate> {
        self.into_iter()
            .map(|(p, r)| (r, new_allocations[p], actual_allocations[p], weights[p]))
            .flat_map(|v| match v {
                (Some(r), Some(a1), Some(a0), Some(w)) => Some((r, a1, a0, w)),
                _ => None,
            })
            .map(|(r, a1, a0, w)| r.calculate_return(a1, a0).and_then(|ret| w.try_mul(ret).map_err(|e| e.into())))
            .try_fold(Rate::zero(), |acc, r| acc.try_add(r?)).map_err(|e| e.into())
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
            .return_once(move |_, _| Ok(Rate::from_percent(10)));

        let mut mock_rc2 = MockReturnCalculator::new();
        mock_rc2
            .expect_calculate_return()
            .return_once(move |_, _| Ok(Rate::from_percent(20)));

        assert_eq!(compare(&mock_rc1, &mock_rc2).unwrap(), Ordering::Less);
    }
}
