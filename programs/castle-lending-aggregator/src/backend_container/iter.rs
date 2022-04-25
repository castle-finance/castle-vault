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
