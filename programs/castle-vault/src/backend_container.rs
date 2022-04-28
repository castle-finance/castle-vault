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
use crate::MAX_NUM_PROVIDERS;
use anchor_lang::prelude::ProgramError;
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use core::cmp::Ordering;
use core::ops::{Index, IndexMut};

pub type BackendContainer<T> = BackendContainerGeneric<T, MAX_NUM_PROVIDERS>;

/// Provides an abstraction over supported backends
#[derive(PartialEq, AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct BackendContainerGeneric<T, const N: usize> {
    pub(crate) inner: [Option<T>; N],
}

impl<T> BackendContainer<T> {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    // TODO: Should this just always return `true`, or do we want it to mean "uninitialized"?
    pub fn is_empty(&self) -> bool {
        self.inner.iter().all(Option::is_none)
    }
}

impl<T> Index<Provider> for BackendContainer<T> {
    type Output = T;

    fn index(&self, index: Provider) -> &Self::Output {
        self.inner[index as usize]
            .as_ref()
            .expect("missing index in BackendContainer")
    }
}

impl<T> IndexMut<Provider> for BackendContainer<T> {
    fn index_mut(&mut self, index: Provider) -> &mut Self::Output {
        self.inner[index as usize]
            .as_mut()
            .expect("missing index in BackendContainer")
    }
}

impl<T> Default for BackendContainer<T> {
    fn default() -> Self {
        // TODO: Is there a better way to do this...?
        Self {
            inner: [(); MAX_NUM_PROVIDERS].map(|_| None),
        }
    }
}

impl<'a, T> From<&'a dyn Index<Provider, Output = &'a T>> for BackendContainer<&'a T> {
    fn from(p: &'a dyn Index<Provider, Output = &'a T>) -> Self {
        Provider::iter().fold(BackendContainer::default(), |mut acc, provider| {
            acc[provider] = p[provider];
            acc
        })
    }
}

impl<T> BackendContainer<T> {
    pub fn apply_owned<U: Clone, F: Fn(Provider, T) -> U>(mut self, f: F) -> BackendContainer<U> {
        Provider::iter()
            .map(|provider| {
                (
                    provider,
                    f(
                        provider,
                        self.inner[provider as usize]
                            .take()
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
