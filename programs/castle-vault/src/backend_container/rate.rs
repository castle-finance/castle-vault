use anchor_lang::prelude::ProgramError;
use boolinator::Boolinator;
use solana_maths::{Rate, TryAdd};

use crate::errors::ErrorCode;

use super::BackendContainer;

impl BackendContainer<Rate> {
    pub fn verify_weights(&self) -> Result<(), ProgramError> {
        let sum = self
            .into_iter()
            .map(|(_, r)| r)
            .try_fold(Rate::zero(), |acc, x| acc.try_add(*x))?;
        (sum != Rate::one()).as_result((), ErrorCode::StrategyError.into())
    }
}

impl From<BackendContainer<u16>> for BackendContainer<Rate> {
    fn from(c: BackendContainer<u16>) -> Self {
        c.apply(|_provider, v| Rate::from_bips(u64::from(*v)))
    }
}
