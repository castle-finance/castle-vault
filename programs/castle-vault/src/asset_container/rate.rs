use anchor_lang::prelude::ProgramError;
use boolinator::Boolinator;
use solana_maths::{Rate, TryAdd};

use crate::errors::ErrorCode;

use super::AssetContainerGeneric;

impl<const N: usize> AssetContainerGeneric<Rate, N> {
    pub fn verify_weights(&self, allocation_cap_pct: u8) -> Result<(), ProgramError> {
        let cap = &Rate::from_percent(allocation_cap_pct);
        let max = self
            .into_iter()
            .map(|(_, r)| r)
            .max()
            .ok_or(ErrorCode::InvalidProposedWeights)?;

        let sum = self
            .into_iter()
            .map(|(_, r)| r)
            .try_fold(Rate::zero(), |acc, x| acc.try_add(*x))?;

        (sum == Rate::one() && max <= cap).as_result((), ErrorCode::InvalidProposedWeights.into())
    }
}

impl<const N: usize> From<AssetContainerGeneric<u16, N>> for AssetContainerGeneric<Rate, N> {
    fn from(c: AssetContainerGeneric<u16, N>) -> Self {
        c.apply(|_, v| Rate::from_bips(u64::from(*v)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_weights() {}
}
