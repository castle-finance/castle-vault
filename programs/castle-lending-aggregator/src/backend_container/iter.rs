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
            let v = self
                .inner
                .take(provider)
                .expect("missing index in OwnedBackendContainerIterator");
            (provider, v)
        })
    }
}

// Allows us to create a BackendContainer<T> from an Iterator that yields (Provider, T)
// TODO: this would need to be macro'd
impl<T> FromIterator<(Provider, T)> for BackendContainer<T> {
    fn from_iter<U: IntoIterator<Item = (Provider, T)>>(iter: U) -> Self {
        let mut solend: Option<T> = None;
        let mut port: Option<T> = None;
        let mut jet: Option<T> = None;

        iter.into_iter().for_each(|(provider, v)| match provider {
            Provider::Solend => solend = Some(v),
            Provider::Port => port = Some(v),
            Provider::Jet => jet = Some(v),
        });

        // TODO: check that all are Some() or push it off until an attempted Index?
        BackendContainer { solend, port, jet }
    }
}
