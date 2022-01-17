use std::convert::TryInto;

use solana_maths::{Rate, TryDiv, TryMul};

use super::assets::Asset;

pub struct StrategyConfig {}

pub trait Strategy {
    fn calculate_allocations(&self, assets: Vec<Box<dyn Asset>>) -> Option<Vec<Rate>>;
}

pub struct EqualAllocationStrategy;

impl Strategy for EqualAllocationStrategy {
    fn calculate_allocations(&self, assets: Vec<Box<dyn Asset>>) -> Option<Vec<Rate>> {
        let num_assets = assets.len();
        let equal_allocation = Rate::one()
            .try_div(
                Rate::from_percent(num_assets.try_into().unwrap())
                    .try_mul(100)
                    .ok()?,
            )
            .ok()?;
        Some(vec![equal_allocation; num_assets])
    }
}
