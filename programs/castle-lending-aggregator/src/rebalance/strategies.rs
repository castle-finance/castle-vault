use std::{cmp::Ordering, convert::TryFrom};

// TODO should ProgramError be used in this module?
use anchor_lang::prelude::*;
use solana_maths::{Rate, TryAdd, TryDiv, TryMul};
use strum::IntoEnumIterator;

// TODO refactor so we don't need to depend on higher-level modules
use crate::{errors::ErrorCode, impl_provider_index, state::Provider};

use super::assets::Asset;

#[derive(Debug, Default, Clone, Copy)]
pub struct StrategyWeights {
    pub solend: Rate,
    pub port: Rate,
    pub jet: Rate,
}
impl_provider_index!(StrategyWeights, Rate);

pub trait Strategy {
    fn calculate_weights(&self, assets: &[impl Asset]) -> Result<StrategyWeights, ProgramError>;

    // TODO split this into separate trait?
    /// Fails if the proposed weights don't meet the constraints of the strategy
    /// Default impl is to check that weights add up to 100%
    fn verify(&self, proposed_weights: &StrategyWeights) -> ProgramResult {
        let sum = Provider::iter()
            .map(|p| proposed_weights[p])
            .try_fold(Rate::zero(), |acc, x| acc.try_add(x))?;

        if sum != Rate::one() {
            return Err(ErrorCode::InvalidProposedWeights.into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EqualAllocationStrategy;
impl Strategy for EqualAllocationStrategy {
    fn calculate_weights(&self, assets: &[impl Asset]) -> Result<StrategyWeights, ProgramError> {
        // TODO make this error handling more granular and informative
        let num_assets = u8::try_from(assets.len()).map_err(|_| ErrorCode::StrategyError)?;
        let equal_allocation = Rate::one().try_div(Rate::from_percent(num_assets).try_mul(100)?)?;

        let strategy_weights = &mut StrategyWeights::default();
        for p in Provider::iter() {
            strategy_weights[p] = equal_allocation;
        }
        Ok(*strategy_weights)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MaxYieldStrategy;
impl MaxYieldStrategy {
    fn compare(&self, lhs: &impl Asset, rhs: &impl Asset) -> Result<Ordering, ProgramError> {
        Ok(lhs.expected_return(0)?.cmp(&rhs.expected_return(0)?))
    }
}

impl Strategy for MaxYieldStrategy {
    fn calculate_weights(&self, assets: &[impl Asset]) -> Result<StrategyWeights, ProgramError> {
        let max_yielding_asset = assets
            .iter()
            .max_by(|x, y| self.compare(*x, *y).unwrap())
            // TODO make this error handling more granular and informative
            .ok_or(ErrorCode::StrategyError)?;

        let strategy_weights = &mut StrategyWeights::default();
        for p in Provider::iter() {
            if p == max_yielding_asset.provider() {
                strategy_weights[p] = Rate::one();
            } else {
                strategy_weights[p] = Rate::zero();
            }
        }
        Ok(*strategy_weights)
    }
}
