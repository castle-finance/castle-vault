use crate::borsh::{BorshDeserialize, BorshSerialize};
use crate::rebalance::assets::Provider;
use crate::rebalance::assets::ProviderIter;
use anchor_lang::prelude::{ProgramError, Pubkey};
use std::{iter::FromIterator, ops::Index};
use strum::IntoEnumIterator;

#[derive(Clone)]
pub struct BackendContainer<T> {
    pub solend: T,
    pub port: T,
    pub jet: T,
}

impl<T> BackendContainer<T> {
    pub fn apply<U, F: Fn(Provider, &T) -> U>(&self, f: F) -> BackendContainer<U> {
        BackendContainer {
            solend: f(Provider::Solend, &self.solend),
            port: f(Provider::Port, &self.port),
            jet: f(Provider::Jet, &self.jet),
        }
    }

    pub fn try_apply<U, E, F: Fn(Provider, &T) -> Result<U, E>>(
        &self,
        f: F,
    ) -> Result<BackendContainer<U>, E> {
        Ok(BackendContainer {
            solend: f(Provider::Solend, &self.solend)?,
            port: f(Provider::Port, &self.port)?,
            jet: f(Provider::Jet, &self.jet)?,
        })
    }

    pub fn len(&self) -> usize {
        3
    }

    pub fn is_empty(&self) -> bool {
        false
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

impl<T> Index<Provider> for BackendContainer<T> {
    type Output = T;

    fn index(&self, provider: Provider) -> &Self::Output {
        match provider {
            Provider::Solend => &self.solend,
            Provider::Port => &self.port,
            Provider::Jet => &self.jet,
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

impl<T> std::fmt::Debug for BackendContainer<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackendContainer")
            .field("solend", &self.solend)
            .field("port", &self.port)
            .field("jet", &self.jet)
            .finish()
    }
}

////////////////////////////////////////////////////////////////////////////////
/// De/serialization code
/// This is required if we want to pass this in directly to a handler, e.g. `rebalance_chris()`
////////////////////////////////////////////////////////////////////////////

impl<T> anchor_lang::AccountDeserialize for BackendContainer<T>
where
    T: anchor_lang::AccountDeserialize,
{
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        let mut solend = None;
        let mut port = None;
        let mut jet = None;
        for provider in Provider::iter() {
            match provider {
                Provider::Solend => solend = Some(T::try_deserialize(buf)?),
                Provider::Port => port = Some(T::try_deserialize(buf)?),
                Provider::Jet => jet = Some(T::try_deserialize(buf)?),
            }
        }
        Ok(BackendContainer {
            solend: solend.expect("missing item in AccountDeserialize for BackendContainer"),
            port: port.expect("missing item in AccountDeserialize for BackendContainer"),
            jet: jet.expect("missing item in AccountDeserialize for BackendContainer"),
        })
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Self::try_deserialize(buf)
    }
}

impl<T> anchor_lang::AccountSerialize for BackendContainer<T>
where
    T: anchor_lang::AccountSerialize,
{
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<(), ProgramError> {
        Provider::iter().try_for_each(|provider| self[provider].try_serialize(writer))
    }
}

impl<T> anchor_lang::Owner for BackendContainer<T> {
    fn owner() -> Pubkey {
        todo!()
    }
}

impl<T> BorshSerialize for BackendContainer<T>
where
    T: BorshSerialize,
{
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        Provider::iter().try_for_each(|provider| self[provider].serialize(writer))
    }
}

impl<T> BorshDeserialize for BackendContainer<T>
where
    T: BorshDeserialize,
{
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let mut solend = None;
        let mut port = None;
        let mut jet = None;
        for provider in Provider::iter() {
            match provider {
                Provider::Solend => solend = Some(T::deserialize(buf)?),
                Provider::Port => port = Some(T::deserialize(buf)?),
                Provider::Jet => jet = Some(T::deserialize(buf)?),
            }
        }
        Ok(BackendContainer {
            solend: solend.expect("missing item in BorshDeserialize for BackendContainer"),
            port: port.expect("missing item in BorshDeserialize for BackendContainer"),
            jet: jet.expect("missing item in BorshDeserialize for BackendContainer"),
        })
    }
}
