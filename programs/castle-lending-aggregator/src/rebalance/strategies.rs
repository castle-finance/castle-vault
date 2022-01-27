use std::{cmp::Ordering, convert::TryInto};

use solana_maths::{Rate, TryDiv, TryMul};

use super::assets::Asset;

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

pub struct MaxYieldStrategy;
impl MaxYieldStrategy {
    fn compare(&self, lhs: &dyn Asset, rhs: &dyn Asset) -> Ordering {
        lhs.expected_return()
            .unwrap()
            .cmp(&rhs.expected_return().unwrap())
    }
}

impl Strategy for MaxYieldStrategy {
    fn calculate_allocations(&self, assets: Vec<Box<dyn Asset>>) -> Option<Vec<Rate>> {
        let iter = assets.iter().enumerate();
        let idx = iter.max_by(|x, y| self.compare(&**x.1, &**y.1))?.0;
        let mut ret_vec = vec![Rate::zero(); assets.len()];
        ret_vec[idx] = Rate::one();
        Some(ret_vec)
    }
}
