use std::{cmp::Ordering, convert::TryInto};

use solana_maths::{Rate, TryDiv, TryMul};

use crate::instructions::RateUpdate;

use super::assets::{Asset, Provider};

pub trait Strategy {
    fn calculate_allocations(&self, assets: &[impl Asset]) -> Option<Vec<RateUpdate>>;
}

pub struct EqualAllocationStrategy;
impl Strategy for EqualAllocationStrategy {
    // TODO return a Result
    fn calculate_allocations(&self, assets: &[impl Asset]) -> Option<Vec<RateUpdate>> {
        let num_assets = assets.len();
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
        Some(allocations)
    }
}

use strum::IntoEnumIterator;
pub struct MaxYieldStrategy;
impl MaxYieldStrategy {
    // TODO return a Result
    fn compare(&self, lhs: &impl Asset, rhs: &impl Asset) -> Ordering {
        lhs.expected_return()
            .unwrap()
            .cmp(&rhs.expected_return().unwrap())
    }
}

impl Strategy for MaxYieldStrategy {
    // TODO return a Result
    fn calculate_allocations(&self, assets: &[impl Asset]) -> Option<Vec<RateUpdate>> {
        let asset = assets.iter().max_by(|x, y| self.compare(*x, *y))?;

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
        Some(ret_vec)
    }
}
