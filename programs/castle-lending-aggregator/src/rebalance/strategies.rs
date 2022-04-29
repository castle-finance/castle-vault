use std::{cmp::Ordering, convert::TryFrom};

// TODO should ProgramError be used in this module?
use anchor_lang::prelude::*;
use solana_maths::{Rate, TryAdd, TryDiv, TrySub};
use strum::IntoEnumIterator;

// TODO refactor so we don't need to depend on higher-level modules
use crate::{errors::ErrorCode, impl_provider_index};

use super::assets::*;

// TODO rename to PortfolioWeights?
#[derive(Debug, Default, Clone, Copy)]
pub struct StrategyWeights {
    pub solend: Rate,
    pub port: Rate,
    pub jet: Rate,
}
impl_provider_index!(StrategyWeights, Rate);

pub trait Strategy {
    fn calculate_weights(
        &self,
        assets: &Assets,
        _allocation_cap_pct: u8,
    ) -> Result<StrategyWeights, ProgramError>;

    // TODO split this into separate trait?
    /// Fails if the proposed weights don't meet the constraints of the strategy
    /// Default impl is to check that weights add up to 100%
    fn verify_weights(
        &self,
        proposed_weights: &StrategyWeights,
        allocation_cap_pct: u8,
    ) -> ProgramResult {
        let sum = Provider::iter()
            .map(|p| proposed_weights[p])
            .try_fold(Rate::zero(), |acc, x| acc.try_add(x))?;

        if sum != Rate::one() {
            return Err(ErrorCode::InvalidProposedWeights.into());
        }

        let cap = Rate::from_percent(allocation_cap_pct);
        for p in Provider::iter() {
            if proposed_weights[p].gt(&cap) {
                return Err(ErrorCode::InvalidProposedWeights.into());
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EqualAllocationStrategy;
impl Strategy for EqualAllocationStrategy {
    fn calculate_weights(
        &self,
        assets: &Assets,
        _allocation_cap_pct: u8,
    ) -> Result<StrategyWeights, ProgramError> {
        // TODO make this error handling more granular and informative
        let num_assets = u8::try_from(assets.len()).map_err(|_| ErrorCode::StrategyError)?;
        let equal_allocation = Rate::one().try_div(num_assets as u64)?;

        let mut strategy_weights = StrategyWeights::default();
        for p in Provider::iter() {
            strategy_weights[p] = equal_allocation;
        }
        Ok(strategy_weights)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MaxYieldStrategy;
impl MaxYieldStrategy {
    fn compare(
        &self,
        lhs: &impl ReturnCalculator,
        rhs: &impl ReturnCalculator,
    ) -> Result<Ordering, ProgramError> {
        Ok(lhs.calculate_return(0)?.cmp(&rhs.calculate_return(0)?))
    }
}

impl Strategy for MaxYieldStrategy {
    fn calculate_weights(
        &self,
        assets: &Assets,
        allocation_cap_pct: u8,
    ) -> Result<StrategyWeights, ProgramError> {
        let mut sorted_pools: Vec<Provider> = Provider::iter().collect();
        sorted_pools.sort_unstable_by(|x, y| self.compare(&assets[*y], &assets[*x]).unwrap());

        let cap = Rate::from_percent(allocation_cap_pct);
        let mut remaining_weight = Rate::one();
        let mut strategy_weights = StrategyWeights::default();
        for p in sorted_pools {
            let target_weight = remaining_weight.min(cap);
            remaining_weight = remaining_weight.try_sub(target_weight)?;
            strategy_weights[p] = target_weight;
        }

        Ok(strategy_weights)
    }
}
