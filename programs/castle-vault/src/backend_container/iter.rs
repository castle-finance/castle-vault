use core::iter::FromIterator;

use strum::IntoEnumIterator;

use crate::reserves::{Provider, ProviderIter};

use super::BackendContainerGeneric;

impl<'a, T, const N: usize> IntoIterator for &'a BackendContainerGeneric<T, N> {
    type Item = (Provider, &'a T);
    type IntoIter = BackendContainerIterator<'a, T, N>;

    fn into_iter(self) -> Self::IntoIter {
        BackendContainerIterator {
            inner: self,
            inner_iter: Provider::iter(),
        }
    }
}

impl<T, const N: usize> IntoIterator for BackendContainerGeneric<T, N> {
    type Item = (Provider, T);
    type IntoIter = OwnedBackendContainerIterator<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        OwnedBackendContainerIterator {
            inner: self,
            inner_iter: Provider::iter(),
        }
    }
}

pub struct BackendContainerIterator<'inner, T, const N: usize> {
    inner: &'inner BackendContainerGeneric<T, N>,
    inner_iter: ProviderIter,
}

impl<'inner, T, const N: usize> Iterator for BackendContainerIterator<'inner, T, N> {
    type Item = (Provider, &'inner T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter
            .next()
            .map(|provider| (provider, &self.inner[provider]))
    }
}

pub struct OwnedBackendContainerIterator<T, const N: usize> {
    inner: BackendContainerGeneric<T, N>,
    inner_iter: ProviderIter,
}

impl<T, const N: usize> Iterator for OwnedBackendContainerIterator<T, N> {
    type Item = (Provider, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter.next().map(|provider| {
            (
                provider,
                self.inner.inner[provider as usize]
                    .take()
                    .expect("missing index in OwnedBackendContainerIterator"),
            )
        })
    }
}

// Allows us to create a BackendContainerGeneric<T, N> from an Iterator that yields (Provider, T)
impl<T: Default, const N: usize> FromIterator<(Provider, T)> for BackendContainerGeneric<T, N> {
    fn from_iter<U: IntoIterator<Item = (Provider, T)>>(iter: U) -> Self {
        iter.into_iter().fold(
            BackendContainerGeneric::default(),
            |mut acc, (provider, v)| {
                acc[provider] = v;
                acc
            },
        )
    }
}
