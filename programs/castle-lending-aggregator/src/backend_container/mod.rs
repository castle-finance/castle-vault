mod allocation;
mod iter;
mod rate;
mod reserves;

pub use allocation::*;
pub use iter::*;
pub use rate::*;
pub use reserves::*;
use strum::IntoEnumIterator;

use crate::rebalance::assets::{Provider, ReturnCalculator};
// use crate::{BorshDeserialize, BorshSerialize};
use anchor_lang::prelude::{ProgramError, Pubkey};
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use std::cmp::Ordering;
use std::ops::Index;

#[derive(PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct BackendContainer<T> {
    pub solend: T,
    pub port: T,
    pub jet: T,
}

impl<T> Index<Provider> for BackendContainer<T> {
    type Output = T;

    fn index(&self, provider: Provider) -> &Self::Output {
        match provider {
            Provider::Solend => &self.solend,
            Provider::Port => &self.port,
            Provider::Jet => &self.jet,
        }
    }
}

impl<T> BackendContainer<T> {
    pub fn len(&self) -> usize {
        3
    }

    pub fn is_empty(&self) -> bool {
        false
    }
}

impl<T> BackendContainer<T> {
    /// Applies `f` to each element of the container individually, yielding a new container
    pub fn apply<U, F: Fn(Provider, &T) -> U>(&self, f: F) -> BackendContainer<U> {
        // Because we have FromIterator<(Provider, T)>, if we yield a tuple of
        // `(Provider, U)` we can `collect()` this into a `BackendContainer<U>`
        self.into_iter()
            .map(|(provider, value)| (provider, f(provider, value)))
            .collect()
    }

    /// Identical to `apply` but returns a `Result<BackendContainer<..>>`
    pub fn try_apply<U, E, F: Fn(Provider, &T) -> Result<U, E>>(
        &self,
        f: F,
    ) -> Result<BackendContainer<U>, E> {
        self.into_iter()
            .map(|(provider, value)| f(provider, value).map(|res| (provider, res)))
            // collect() will stop at the first failure
            .collect()
    }
}

impl<T> std::fmt::Debug for BackendContainer<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackendContainer")
            .field("solend", &self.solend)
            .field("port", &self.port)
            .field("jet", &self.jet)
            .finish()
    }
}

impl<T> Clone for BackendContainer<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            solend: self.solend.clone(),
            port: self.port.clone(),
            jet: self.jet.clone(),
        }
    }
}

impl<T> BackendContainer<T>
where
    T: ReturnCalculator,
{
    pub fn compare(&self, lhs: &T, rhs: &T) -> Result<Ordering, ProgramError> {
        Ok(lhs.calculate_return(0)?.cmp(&rhs.calculate_return(0)?))
    }
}

impl<T> anchor_lang::Owner for BackendContainer<T> {
    fn owner() -> Pubkey {
        todo!()
    }
}

impl<T> anchor_lang::AccountDeserialize for BackendContainer<T>
where
    T: anchor_lang::AccountDeserialize,
{
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        let mut solend = None;
        let mut port = None;
        let mut jet = None;
        for provider in Provider::iter() {
            match provider {
                Provider::Solend => solend = Some(T::try_deserialize(buf)?),
                Provider::Port => port = Some(T::try_deserialize(buf)?),
                Provider::Jet => jet = Some(T::try_deserialize(buf)?),
            }
        }
        Ok(BackendContainer {
            solend: solend.ok_or(ProgramError::InvalidAccountData)?,
            port: port.ok_or(ProgramError::InvalidAccountData)?,
            jet: jet.ok_or(ProgramError::InvalidAccountData)?,
        })
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        todo!()
    }
}

impl<T> anchor_lang::AccountSerialize for BackendContainer<T>
where
    T: anchor_lang::AccountSerialize,
{
    // TODO: is this right?
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<(), ProgramError> {
        Provider::iter().try_for_each(|provider| self[provider].try_serialize(writer))
    }
}
