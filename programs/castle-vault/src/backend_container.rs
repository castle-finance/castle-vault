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
use std::ops::{Index, IndexMut};

/// Provides an abstraction over supported backends
#[derive(PartialEq, AnchorSerialize, AnchorDeserialize, Debug, Default, Clone)]
pub struct BackendContainer<T> {
    pub solend: Option<T>,
    pub port: Option<T>,
    pub jet: Option<T>,
    // pub m: BTreeMap<Provider, T>,
}

impl<T> Index<Provider> for BackendContainer<T> {
    type Output = T;

    fn index(&self, provider: Provider) -> &Self::Output {
        match provider {
            Provider::Solend => self
                .solend
                .as_ref()
                .expect("missing Solend in BackendContainer"),
            Provider::Port => self
                .port
                .as_ref()
                .expect("missing Port in BackendContainer"),
            Provider::Jet => self.jet.as_ref().expect("missing Jet in BackendContainer"),
        }
    }
}

impl<T> IndexMut<Provider> for BackendContainer<T> {
    fn index_mut(&mut self, provider: Provider) -> &mut T {
        match provider {
            Provider::Solend => self
                .solend
                .as_mut()
                .expect("missing Solend in BackendContainer"),
            Provider::Port => self
                .port
                .as_mut()
                .expect("missing Port in BackendContainer"),
            Provider::Jet => self.jet.as_mut().expect("missing Jet in BackendContainer"),
        }
    }
}

impl<'a, T> From<&'a dyn Index<Provider, Output = &'a T>> for BackendContainer<&'a T> {
    fn from(p: &'a dyn Index<Provider, Output = &'a T>) -> Self {
        Self {
            solend: Some(p[Provider::Solend]),
            port: Some(p[Provider::Port]),
            jet: Some(p[Provider::Jet]),
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
    fn take(&mut self, provider: Provider) -> Option<T> {
        match provider {
            Provider::Solend => self.solend.take(),
            Provider::Port => self.port.take(),
            Provider::Jet => self.jet.take(),
        }
    }
}

impl<T> BackendContainer<T> {
    pub fn apply_owned<U, F: Fn(Provider, T) -> U>(mut self, f: F) -> BackendContainer<U> {
        Provider::iter()
            .map(|provider| {
                (
                    provider,
                    f(
                        provider,
                        self.take(provider)
                            .expect("unable to take() in apply_owned()"),
                    ),
                )
            })
            .collect()
    }

    /// Applies `f` to each element of the container individually, yielding a new container
    pub fn apply<U, F: Fn(Provider, &T) -> U>(&self, f: F) -> BackendContainer<U> {
        // Because we have FromIterator<(Provider, T)>, if we yield a tuple of
        // `(Provider, U)` we can `collect()` this into a `BackendContainer<U>`
        Provider::iter()
            .map(|provider| (provider, f(provider, &self[provider])))
            .collect()
    }

    /// Identical to `apply` but returns a `Result<BackendContainer<..>>`
    pub fn try_apply<U, E, F: Fn(Provider, &T) -> Result<U, E>>(
        &self,
        f: F,
    ) -> Result<BackendContainer<U>, E> {
        Provider::iter()
            .map(|provider| f(provider, &self[provider]).map(|res| (provider, res)))
            // collect() will stop at the first failure
            .collect()
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
        Provider::iter()
            .map(|provider| {
                let val = T::try_deserialize(buf)?;
                Ok((provider, val))
            })
            .collect()
    }

    fn try_deserialize_unchecked(_buf: &mut &[u8]) -> Result<Self, ProgramError> {
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
