use core::iter::FromIterator;

use strum::IntoEnumIterator;

use crate::rebalance::assets::{Provider, ProviderIter};

use super::BackendContainer;

impl<'a, T> IntoIterator for &'a BackendContainer<T> {
    type Item = (Provider, &'a T);
    type IntoIter = BackendContainerIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        BackendContainerIterator {
            inner: self,
            inner_iter: Provider::iter(),
        }
    }
}

impl<T> IntoIterator for BackendContainer<T> {
    type Item = (Provider, T);
    type IntoIter = OwnedBackendContainerIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        OwnedBackendContainerIterator {
            inner: self,
            inner_iter: Provider::iter(),
        }
    }
}

pub struct BackendContainerIterator<'inner, T> {
    inner: &'inner BackendContainer<T>,
    inner_iter: ProviderIter,
}

impl<'inner, T> Iterator for BackendContainerIterator<'inner, T> {
    type Item = (Provider, &'inner T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter
            .next()
            .map(|provider| (provider, &self.inner[provider]))
    }
}

pub struct OwnedBackendContainerIterator<T> {
    inner: BackendContainer<T>,
    inner_iter: ProviderIter,
}

impl<T> Iterator for OwnedBackendContainerIterator<T> {
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

// Allows us to create a BackendContainer<T> from an Iterator that yields (Provider, T)
impl<T> FromIterator<(Provider, T)> for BackendContainer<T> {
    fn from_iter<U: IntoIterator<Item = (Provider, T)>>(iter: U) -> Self {
        iter.into_iter()
            .fold(BackendContainer::default(), |mut acc, (provider, v)| {
                acc[provider] = v;
                acc
            })
    }
}
