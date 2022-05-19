use core::iter::FromIterator;

use strum::IntoEnumIterator;

use crate::reserves::{Provider, ProviderIter};

use super::AssetContainerGeneric;

impl<'a, T, const N: usize> IntoIterator for &'a AssetContainerGeneric<T, N> {
    type Item = (Provider, Option<&'a T>);
    type IntoIter = AssetContainerIterator<'a, T, N>;

    fn into_iter(self) -> Self::IntoIter {
        AssetContainerIterator {
            inner: self,
            inner_iter: Provider::iter(),
        }
    }
}

impl<T, const N: usize> IntoIterator for AssetContainerGeneric<T, N> {
    type Item = (Provider, Option<T>);
    type IntoIter = OwnedAssetContainerIterator<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        OwnedAssetContainerIterator {
            inner: self,
            inner_iter: Provider::iter(),
        }
    }
}

pub struct AssetContainerIterator<'inner, T, const N: usize> {
    inner: &'inner AssetContainerGeneric<T, N>,
    inner_iter: ProviderIter,
}

impl<'inner, T, const N: usize> Iterator for AssetContainerIterator<'inner, T, N> {
    type Item = (Provider, Option<&'inner T>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter
            .next()
            .map(|provider| (provider, self.inner[provider].as_ref()))
    }
}

pub struct OwnedAssetContainerIterator<T, const N: usize> {
    inner: AssetContainerGeneric<T, N>,
    inner_iter: ProviderIter,
}

impl<T, const N: usize> Iterator for OwnedAssetContainerIterator<T, N> {
    type Item = (Provider, Option<T>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter
            .next()
            .map(|provider| (provider, self.inner.inner[provider as usize].take()))
    }
}

// Allows us to create a AssetContainerGeneric<T, N> from an Iterator that yields (Provider, T)
impl<T: Default, const N: usize> FromIterator<(Provider, Option<T>)>
    for AssetContainerGeneric<T, N>
{
    fn from_iter<U: IntoIterator<Item = (Provider, Option<T>)>>(iter: U) -> Self {
        iter.into_iter().fold(
            AssetContainerGeneric::default(),
            |mut acc, (provider, v)| {
                acc[provider] = v;
                acc
            },
        )
    }
}

// TODO add unit tests
