use std::{cmp::Ordering, convert::TryInto};

use anchor_lang::prelude::ProgramError;
use solana_maths::{Rate, TryDiv, TryMul};
use strum::IntoEnumIterator;

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
        let num_assets = assets.len();
        // TODO don't suppress errors
        let allocations = Provider::iter()
            .map(|provider| RateUpdate {
                provider,
                rate: Rate::one()
                    .try_div(
                        Rate::from_percent(num_assets.try_into().unwrap())
                            .try_mul(100)
                            .unwrap(),
                    )
                    .unwrap(),
            })
            .collect::<Vec<RateUpdate>>();
        Ok(allocations)
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
