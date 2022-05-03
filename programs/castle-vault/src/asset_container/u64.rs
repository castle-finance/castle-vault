use anchor_lang::prelude::ProgramError;
use solana_maths::{Decimal, Rate, TryMul};

use super::AssetContainerGeneric;

impl<const N: usize> AssetContainerGeneric<u64, N> {
    /// Calculates $ allocations for a corresponding set of % allocations
    /// and a given total amount
    pub fn try_from_weights(
        rates: &AssetContainerGeneric<Rate, N>,
        total_amount: u64,
    ) -> Result<Self, ProgramError> {
        rates.try_apply(|_provider, rate| {
            rate.try_mul(total_amount)
                .and_then(|product| Decimal::from(product).try_floor_u64())
        })
    }
}
