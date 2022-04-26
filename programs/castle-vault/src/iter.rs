use core::iter::FromIterator;

use strum::IntoEnumIterator;

use crate::rebalance::assets::{Provider, ProviderIter};

use super::BackendContainer;

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

impl<T> FromIterator<(Provider, T)> for BackendContainer<T> {
    fn from_iter<U: IntoIterator<Item = (Provider, T)>>(iter: U) -> Self {
        let mut solend = None;
        let mut port = None;
        let mut jet = None;
        for (provider, backend) in iter {
            match provider {
                Provider::Solend => solend = Some(backend),
                Provider::Port => port = Some(backend),
                Provider::Jet => jet = Some(backend),
            }
        }
        Self {
            solend: solend.expect("missing item in FromIterator for BackendContainer"),
            port: port.expect("missing item in FromIterator for BackendContainer"),
            jet: jet.expect("missing item in FromIterator for BackendContainer"),
        }
    }
}
