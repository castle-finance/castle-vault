mod iter;
mod rate;
mod reserves;
mod u64;

pub use self::u64::*;
pub use iter::*;
pub use rate::*;
pub use reserves::*;

use core::cmp::Ordering;
use core::ops::{Index, IndexMut};

use strum::{EnumCount, IntoEnumIterator};

use anchor_lang::prelude::*;

use crate::reserves::{Provider, ReturnCalculator};

// TODO is there a better name for this?
pub type BackendContainer<T> = BackendContainerGeneric<T, { Provider::COUNT }>;

/// Provides an abstraction over supported backends
#[derive(Debug, Clone)]
pub struct BackendContainerGeneric<T, const N: usize> {
    pub(crate) inner: [Option<T>; N],
}

impl<T, const N: usize> BackendContainerGeneric<T, N> {
    pub fn len(&self) -> usize {
        N
    }

    pub fn is_empty(&self) -> bool {
        // TODO: Should this just always return `true`, or do we want it to mean "uninitialized"?
        self.inner.iter().all(Option::is_none)
    }
}

impl<T, const N: usize> Index<Provider> for BackendContainerGeneric<T, N> {
    type Output = T;

    fn index(&self, index: Provider) -> &Self::Output {
        self.inner[index as usize].as_ref().expect(&format!(
            "missing index {:?} / {:?} in BackendContainerGeneric",
            index, index as usize
        ))
    }
}

impl<T, const N: usize> IndexMut<Provider> for BackendContainerGeneric<T, N> {
    fn index_mut(&mut self, index: Provider) -> &mut Self::Output {
        self.inner[index as usize].as_mut().expect(&format!(
            "missing index {:?} / {:?} in BackendContainerGeneric",
            index, index as usize
        ))
    }
}

impl<T: Default, const N: usize> Default for BackendContainerGeneric<T, N> {
    fn default() -> Self {
        // TODO: Is there a better way to do this...?
        Self {
            inner: [(); N].map(|_| Some(T::default())),
        }
    }
}

impl<'a, T, const N: usize> From<&'a dyn Index<Provider, Output = &'a T>>
    for BackendContainerGeneric<&'a T, N>
where
    &'a T: Default,
{
    fn from(p: &'a dyn Index<Provider, Output = &'a T>) -> Self {
        Provider::iter().fold(BackendContainerGeneric::default(), |mut acc, provider| {
            acc[provider] = p[provider];
            acc
        })
    }
}

impl<T, const N: usize> BackendContainerGeneric<T, N> {
    pub fn apply_owned<U: Clone + Default, F: Fn(Provider, T) -> U>(
        mut self,
        f: F,
    ) -> BackendContainerGeneric<U, N> {
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
    pub fn apply<U: Default, F: Fn(Provider, &T) -> U>(
        &self,
        f: F,
    ) -> BackendContainerGeneric<U, N> {
        // Because we have FromIterator<(Provider, T)>, if we yield a tuple of
        // `(Provider, U)` we can `collect()` this into a `BackendContainerGeneric<U>`
        Provider::iter()
            .map(|provider| (provider, f(provider, &self[provider])))
            .collect()
    }

    /// Identical to `apply` but returns a `Result<BackendContainerGeneric<..>>`
    pub fn try_apply<U: Default, E, F: Fn(Provider, &T) -> Result<U, E>>(
        &self,
        f: F,
    ) -> Result<BackendContainerGeneric<U, N>, E> {
        Provider::iter()
            .map(|provider| f(provider, &self[provider]).map(|res| (provider, res)))
            // collect() will stop at the first failure
            .collect()
    }
}

impl<T, const N: usize> BackendContainerGeneric<T, N>
where
    T: ReturnCalculator,
{
    pub fn compare(&self, lhs: &T, rhs: &T) -> Result<Ordering, ProgramError> {
        Ok(lhs.calculate_return(0)?.cmp(&rhs.calculate_return(0)?))
    }
}
