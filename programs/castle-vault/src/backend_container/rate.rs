use anchor_lang::prelude::ProgramError;
use boolinator::Boolinator;
use solana_maths::{Rate, TryAdd};

use crate::errors::ErrorCode;

use super::BackendContainerGeneric;

impl<const N: usize> BackendContainerGeneric<Rate, N> {
    pub fn verify_weights(&self, allocation_cap_pct: u8) -> Result<(), ProgramError> {
        let cap = Rate::from_percent(allocation_cap_pct);
        let max = self
            .into_iter()
            .max()
            .ok_or(ErrorCode::InvalidProposedWeights)?
            .1;

        let sum = self
            .into_iter()
            .map(|(_, r)| r)
            .try_fold(Rate::zero(), |acc, x| acc.try_add(*x))?;

        (sum != Rate::one() || max.gt(&cap)).as_result((), ErrorCode::InvalidProposedWeights.into())
    }
}

impl<const N: usize> From<BackendContainerGeneric<u16, N>> for BackendContainerGeneric<Rate, N> {
    fn from(c: BackendContainerGeneric<u16, N>) -> Self {
        c.apply(|_provider, v| Rate::from_bips(u64::from(*v)))
    }
}
