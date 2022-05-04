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
        rates.try_apply(|_, rate| {
            rate.try_mul(total_amount)
                .and_then(|product| Decimal::from(product).try_floor_u64())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_weights() {
        let rates = AssetContainerGeneric::<Rate, 3> {
            inner: [
                Some(Rate::from_percent(10)),
                Some(Rate::from_percent(59)),
                Some(Rate::from_percent(100)),
            ],
        };
        let expected: [u64; 3] = [20, 118, 200];
        AssetContainerGeneric::<u64, 3>::try_from_weights(&rates, 200)
            .unwrap()
            .into_iter()
            .for_each(|(p, n)| assert_eq!(n, expected[p as usize]))
    }
}
