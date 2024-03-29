mod iter;
mod rate;
mod reserves;
mod u64;

pub use self::u64::*;
pub use iter::*;
pub use rate::*;
pub use reserves::*;

use core::ops::{Index, IndexMut};

use strum::{EnumCount, IntoEnumIterator};

use crate::reserves::Provider;

pub type AssetContainer<T> = AssetContainerGeneric<T, { Provider::COUNT }>;

/// Provides an abstraction over supported assets
#[derive(Debug, Clone)]
pub struct AssetContainerGeneric<T, const N: usize> {
    pub(crate) inner: [Option<T>; N],
}

impl<T, const N: usize> AssetContainerGeneric<T, N> {
    pub fn len(&self) -> usize {
        self.into_iter().filter(|(_, o)| o.is_some()).count()
    }

    /// Returns if the container is uninitialized
    pub fn is_empty(&self) -> bool {
        self.inner.iter().all(Option::is_none)
    }
}

impl<T, const N: usize> Index<Provider> for AssetContainerGeneric<T, N> {
    type Output = Option<T>;

    fn index(&self, index: Provider) -> &Self::Output {
        &self.inner[index as usize]
    }
}

impl<T, const N: usize> IndexMut<Provider> for AssetContainerGeneric<T, N> {
    fn index_mut(&mut self, index: Provider) -> &mut Self::Output {
        &mut self.inner[index as usize]
    }
}

impl<T: Default, const N: usize> Default for AssetContainerGeneric<T, N> {
    fn default() -> Self {
        // TODO: Is there a better way to do this...?
        Self {
            inner: [(); N].map(|_| None),
        }
    }
}

impl<'a, T, const N: usize> From<&'a dyn Index<Provider, Output = &'a T>>
    for AssetContainerGeneric<&'a T, N>
where
    &'a T: Default,
{
    fn from(p: &'a dyn Index<Provider, Output = &'a T>) -> Self {
        Provider::iter().fold(AssetContainerGeneric::default(), |mut acc, provider| {
            acc[provider] = Some(p[provider]);
            acc
        })
    }
}

impl<T, const N: usize> AssetContainerGeneric<T, N> {
    pub fn apply_owned<U: Clone + Default, F: Fn(Provider, Option<&T>) -> Option<U>>(
        mut self,
        f: F,
    ) -> AssetContainerGeneric<U, N> {
        Provider::iter()
            .map(|provider| {
                (
                    provider,
                    f(provider, self.inner[provider as usize].take().as_ref()),
                )
            })
            .collect()
    }

    /// Applies `f` to each element of the container individually, yielding a new container
    pub fn apply<U: Default, F: Fn(Provider, Option<&T>) -> Option<U>>(
        &self,
        f: F,
    ) -> AssetContainerGeneric<U, N> {
        // Because we have FromIterator<(Provider, T)>, if we yield a tuple of
        // `(Provider, U)` we can `collect()` this into a `AssetContainerGeneric<U>`
        Provider::iter()
            .map(|provider| (provider, f(provider, self[provider].as_ref())))
            .collect()
    }

    /// Identical to `apply` but returns a `Result<AssetContainerGeneric<..>>`
    pub fn try_apply<U: Default, E, F: Fn(Provider, Option<&T>) -> Result<Option<U>, E>>(
        &self,
        f: F,
    ) -> Result<AssetContainerGeneric<U, N>, E> {
        Provider::iter()
            .map(|provider| f(provider, self[provider].as_ref()).map(|res| (provider, res)))
            // collect() will stop at the first failure
            .collect()
    }
}

// TODO add unit tests
