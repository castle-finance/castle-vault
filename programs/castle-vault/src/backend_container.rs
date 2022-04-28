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
use anchor_lang::prelude::ProgramError;
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use std::cmp::Ordering;
use std::ops::{Index, IndexMut};

/// Provides an abstraction over supported backends
#[derive(PartialEq, AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct BackendContainer<T, const N: usize> {
    inner: [Option<T>; N],
}

impl<T, const N: usize> BackendContainer<T, N> {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.iter().all(|x| x.is_none())
    }
}

impl<T, const N: usize> Index<Provider> for BackendContainer<T, N> {
    type Output = T;

    fn index(&self, index: Provider) -> &Self::Output {
        self.inner[index as usize]
            .as_ref()
            .expect("missing index in BackendContainer")
    }
}

impl<T, const N: usize> IndexMut<Provider> for BackendContainer<T, N> {
    fn index_mut(&mut self, index: Provider) -> &mut Self::Output {
        self.inner[index as usize]
            .as_mut()
            .expect("missing index in BackendContainer")
    }
}

impl<T, const N: usize> Default for BackendContainer<T, N> {
    fn default() -> Self {
        // TODO: Is there a better way to do this...?
        Self {
            inner: [(); N].map(|_| None),
        }
    }
}

impl<'a, T, const N: usize> From<&'a dyn Index<Provider, Output = &'a T>>
    for BackendContainer<&'a T, N>
{
    fn from(p: &'a dyn Index<Provider, Output = &'a T>) -> Self {
        Provider::iter().fold(BackendContainer::default(), |mut acc, provider| {
            acc[provider] = p[provider];
            acc
        })
    }
}

impl<T, const N: usize> BackendContainer<T, N> {
    pub fn apply_owned<U: Clone, F: Fn(Provider, T) -> U>(
        mut self,
        f: F,
    ) -> BackendContainer<U, N> {
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
    pub fn apply<U, F: Fn(Provider, &T) -> U>(&self, f: F) -> BackendContainer<U, N> {
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
    ) -> Result<BackendContainer<U, N>, E> {
        Provider::iter()
            .map(|provider| f(provider, &self[provider]).map(|res| (provider, res)))
            // collect() will stop at the first failure
            .collect()
    }
}

impl<T, const N: usize> BackendContainer<T, N>
where
    T: ReturnCalculator,
{
    pub fn compare(&self, lhs: &T, rhs: &T) -> Result<Ordering, ProgramError> {
        Ok(lhs.calculate_return(0)?.cmp(&rhs.calculate_return(0)?))
    }
}

////////////////////////////////////////////////////////////////////////////////

// #[derive(PartialEq, AnchorSerialize, AnchorDeserialize, Debug, Clone)]
// pub struct BackendContainer<T> {
//     pub solend: Option<T>,
//     pub port: Option<T>,
//     pub jet: Option<T>,
// }

// impl<T> Index<Provider> for BackendContainer<T> {
//     type Output = T;

//     fn index(&self, provider: Provider) -> &Self::Output {
//         match provider {
//             Provider::Solend => self
//                 .solend
//                 .as_ref()
//                 .expect("missing Solend in BackendContainer"),
//             Provider::Port => self
//                 .port
//                 .as_ref()
//                 .expect("missing Port in BackendContainer"),
//             Provider::Jet => self.jet.as_ref().expect("missing Jet in BackendContainer"),
//         }
//     }
// }

// impl<T> IndexMut<Provider> for BackendContainer<T> {
//     fn index_mut(&mut self, provider: Provider) -> &mut T {
//         match provider {
//             Provider::Solend => self
//                 .solend
//                 .as_mut()
//                 .expect("missing Solend in BackendContainer"),
//             Provider::Port => self
//                 .port
//                 .as_mut()
//                 .expect("missing Port in BackendContainer"),
//             Provider::Jet => self.jet.as_mut().expect("missing Jet in BackendContainer"),
//         }
//     }
// }

// impl<'a, T> From<&'a dyn Index<Provider, Output = &'a T>> for BackendContainer<&'a T> {
//     fn from(p: &'a dyn Index<Provider, Output = &'a T>) -> Self {
//         Provider::iter().fold(BackendContainer::default(), |mut acc, provider| {
//             acc[provider] = p[provider];
//             acc
//         })
//     }
// }

// impl<T, const N: usize> BackendContainer<T, N> {
//     pub fn len(&self) -> usize {
//         3
//     }

//     pub fn is_empty(&self) -> bool {
//         false
//     }
//     fn take(&mut self, provider: Provider) -> Option<T> {
//         match provider {
//             Provider::Solend => self.solend.take(),
//             Provider::Port => self.port.take(),
//             Provider::Jet => self.jet.take(),
//         }
//     }
// }

// impl<T> Default for BackendContainer<T> {
//     fn default() -> Self {
//         Self {
//             solend: Default::default(),
//             port: Default::default(),
//             jet: Default::default(),
//         }
//     }
// }

// impl<T, const N: usize> BackendContainer<T, N> {
//     pub fn apply_owned<U, F: Fn(Provider, T) -> U>(mut self, f: F) -> BackendContainer<U> {
//         Provider::iter()
//             .map(|provider| {
//                 (
//                     provider,
//                     f(
//                         provider,
//                         self.take(provider)
//                             .expect("unable to take() in apply_owned()"),
//                     ),
//                 )
//             })
//             .collect()
//     }

//     /// Applies `f` to each element of the container individually, yielding a new container
//     pub fn apply<U, F: Fn(Provider, &T) -> U>(&self, f: F) -> BackendContainer<U> {
//         // Because we have FromIterator<(Provider, T)>, if we yield a tuple of
//         // `(Provider, U)` we can `collect()` this into a `BackendContainer<U>`
//         Provider::iter()
//             .map(|provider| (provider, f(provider, &self[provider])))
//             .collect()
//     }

//     /// Identical to `apply` but returns a `Result<BackendContainer<..>>`
//     pub fn try_apply<U, E, F: Fn(Provider, &T) -> Result<U, E>>(
//         &self,
//         f: F,
//     ) -> Result<BackendContainer<U>, E> {
//         Provider::iter()
//             .map(|provider| f(provider, &self[provider]).map(|res| (provider, res)))
//             // collect() will stop at the first failure
//             .collect()
//     }
// }

// impl<T, const N: usize> BackendContainer<T, N>
// where
//     T: ReturnCalculator,
// {
//     pub fn compare(&self, lhs: &T, rhs: &T) -> Result<Ordering, ProgramError> {
//         Ok(lhs.calculate_return(0)?.cmp(&rhs.calculate_return(0)?))
//     }
// }
