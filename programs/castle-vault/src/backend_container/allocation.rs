use anchor_lang::prelude::ProgramError;
use solana_maths::{Decimal, Rate, TryMul};

use crate::state::{Allocation, LastUpdate};

use super::BackendContainer;

impl BackendContainer<Allocation> {
    pub fn try_from_weights(
        rates: &BackendContainer<Rate>,
        vault_value: u64,
        slot: u64,
    ) -> Result<Self, ProgramError> {
        rates.try_apply(|_provider, rate| {
            rate.try_mul(vault_value).and_then(|product| {
                Decimal::from(product)
                    .try_floor_u64()
                    .map(|value| Allocation {
                        value,
                        last_update: LastUpdate::new(slot),
                    })
            })
        })
    }
}