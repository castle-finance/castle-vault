use anchor_lang::prelude::ProgramError;
use boolinator::Boolinator;
use solana_maths::{Rate, TryAdd};

use crate::errors::ErrorCode;

use super::BackendContainer;

impl<const N: usize> BackendContainer<Rate, N> {
    pub fn verify_weights(&self) -> Result<(), ProgramError> {
        let sum = self
            .into_iter()
            .map(|(_, r)| r)
            .try_fold(Rate::zero(), |acc, x| acc.try_add(*x))?;
        (sum != Rate::one()).as_result((), ErrorCode::StrategyError.into())
    }
}

impl<const N: usize> From<BackendContainer<u16, N>> for BackendContainer<Rate, N> {
    fn from(c: BackendContainer<u16, N>) -> Self {
        c.apply(|_provider, v| Rate::from_bips(u64::from(*v)))
    }
}
