use anchor_lang::prelude::ProgramError;
use solana_maths::{Decimal, Rate, TryMul};

use super::BackendContainerGeneric;

impl<const N: usize> BackendContainerGeneric<u64, N> {
    pub fn try_from_weights(
        rates: &BackendContainerGeneric<Rate, N>,
        vault_value: u64,
    ) -> Result<Self, ProgramError> {
        rates.try_apply(|_provider, rate| {
            rate.try_mul(vault_value)
                .and_then(|product| Decimal::from(product).try_floor_u64())
        })
    }
}
