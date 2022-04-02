use std::{cmp::Ordering, convert::TryFrom};

// TODO should ProgramError be used in this module?
use anchor_lang::prelude::*;
use solana_maths::{Rate, TryDiv, TryMul};
use strum::IntoEnumIterator;

// TODO refactor so we don't need to depend on higher-level modules
use crate::{
    errors::ErrorCode,
    instructions::{ProposedWeightsBps, RateUpdate},
    state::Provider,
};

use super::assets::Asset;

pub trait Strategy {
    fn calculate_allocations(&self, assets: &[impl Asset])
        -> Result<Vec<RateUpdate>, ProgramError>;

    // TODO split this into separate trait?
    /// Fails if the proposed weights don't meet the constraints of the strategy
    /// Default impl is to check that weights add up to 100%
    fn verify(&self, proposed_weights: &ProposedWeightsBps) -> ProgramResult {
        let sum = Provider::iter()
            .map(|p| proposed_weights[p])
            .try_fold(0, |acc: u16, x| acc.checked_add(x))
            .ok_or(ErrorCode::OverflowError)?;

        if sum != 10000 {
            return Err(ErrorCode::InvalidProposedWeights.into());
        }
        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub struct EqualAllocationStrategy;
impl Strategy for EqualAllocationStrategy {
    fn calculate_allocations(
        &self,
        assets: &[impl Asset],
    ) -> Result<Vec<RateUpdate>, ProgramError> {
        // TODO make this error handling more granular and informative
        let num_assets = u8::try_from(assets.len()).map_err(|_| ErrorCode::StrategyError)?;
        let equal_allocation = Rate::one().try_div(Rate::from_percent(num_assets).try_mul(100)?)?;
        Ok(Provider::iter()
            .map(|provider| RateUpdate {
                provider,
                rate: equal_allocation,
            })
            .collect::<Vec<RateUpdate>>())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub struct MaxYieldStrategy;
impl MaxYieldStrategy {
    fn compare(&self, lhs: &impl Asset, rhs: &impl Asset) -> Result<Ordering, ProgramError> {
        Ok(lhs.expected_return()?.cmp(&rhs.expected_return()?))
    }
}

impl Strategy for MaxYieldStrategy {
    fn calculate_allocations(
        &self,
        assets: &[impl Asset],
    ) -> Result<Vec<RateUpdate>, ProgramError> {
        let asset = assets
            .iter()
            .max_by(|x, y| self.compare(*x, *y).unwrap())
            // TODO make this error handling more granular and informative
            .ok_or(ErrorCode::StrategyError)?;

        let ret_vec = Provider::iter()
            .map(|provider| RateUpdate {
                provider,
                rate: if provider == asset.provider() {
                    Rate::one()
                } else {
                    Rate::zero()
                },
            })
            .collect::<Vec<RateUpdate>>();
        Ok(ret_vec)
    }
}
