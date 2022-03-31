use std::{cmp::Ordering, convert::TryFrom};

// TODO should ProgramError be used in this module?
use anchor_lang::prelude::ProgramError;
use solana_maths::{Rate, TryDiv, TryMul};
use strum::IntoEnumIterator;

// TODO refactor so we don't need to depend on higher-level modules
use crate::{errors::ErrorCode, instructions::RateUpdate};

use super::assets::{Asset, Provider};

pub trait Strategy {
    fn calculate_allocations(&self, assets: &[impl Asset])
        -> Result<Vec<RateUpdate>, ProgramError>;
}

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
