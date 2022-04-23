use anchor_lang::prelude::*;
use solana_maths::{Rate, TryMul};
use strum_macros::EnumIter;

// TODO rebalance module should not be dependent on specific names of assets
#[derive(Clone, Copy, Debug, EnumIter, PartialEq)]
pub enum Provider {
    Solend,
    Port,
    Jet,
}

#[macro_export]
macro_rules! impl_provider_index {
    ($t:ty, $o:ty) => {
        impl core::ops::Index<Provider> for $t {
            type Output = $o;

            fn index(&self, provider: Provider) -> &Self::Output {
                match provider {
                    Provider::Solend => &self.solend,
                    Provider::Port => &self.port,
                    Provider::Jet => &self.jet,
                }
            }
        }

        impl core::ops::IndexMut<Provider> for $t {
            fn index_mut(&mut self, provider: Provider) -> &mut Self::Output {
                match provider {
                    Provider::Solend => &mut self.solend,
                    Provider::Port => &mut self.port,
                    Provider::Jet => &mut self.jet,
                }
            }
        }
    };
}

pub struct Assets {
    pub solend: LendingMarketAsset,
    pub port: LendingMarketAsset,
    pub jet: LendingMarketAsset,
}
impl_provider_index!(Assets, LendingMarketAsset);

impl Assets {
    pub fn len(&self) -> usize {
        3_usize
    }

    pub fn is_empty(&self) -> bool {
        false
    }
}

pub trait ReturnCalculator {
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError>;
}

pub trait ReserveAccessor {
    fn utilization_rate(&self) -> Result<Rate, ProgramError>;
    fn borrow_rate(&self) -> Result<Rate, ProgramError>;

    fn reserve_with_deposit(
        &self,
        allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>, ProgramError>;
}

pub struct LendingMarketAsset(pub Box<dyn ReserveAccessor>);

impl ReturnCalculator for LendingMarketAsset {
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError> {
        let reserve = self.0.reserve_with_deposit(allocation)?;
        reserve.utilization_rate()?.try_mul(reserve.borrow_rate()?)
    }
}

// impl `ReturnCalculator` for any type that implements `ReserveAccessor`
impl<T> ReturnCalculator for T
where
    T: ReserveAccessor,
{
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError> {
        self.reserve_with_deposit(allocation)?
            .utilization_rate()?
            .try_mul(self.borrow_rate()?)
    }
}
