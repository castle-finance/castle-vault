#[cfg(test)]
use mockall::*;

use anchor_lang::prelude::*;
use port_anchor_adaptor::PortReserve;
use solana_maths::{Rate, TryMul};
use strum_macros::{EnumCount, EnumIter};

use crate::adapters::solend::SolendReserve;

#[derive(
    Clone,
    Copy,
    Debug,
    EnumIter,
    EnumCount,
    PartialEq,
    Ord,
    Hash,
    Eq,
    PartialOrd,
    AnchorSerialize,
    AnchorDeserialize,
)]
pub enum Provider {
    Solend = 0,
    Port,
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
                }
            }
        }

        impl core::ops::IndexMut<Provider> for $t {
            fn index_mut(&mut self, provider: Provider) -> &mut Self::Output {
                match provider {
                    Provider::Solend => &mut self.solend,
                    Provider::Port => &mut self.port,
                }
            }
        }
    };
}

#[cfg_attr(test, automock)]
pub trait ReserveAccessor {
    fn utilization_rate(&self) -> Result<Rate>;
    fn borrow_rate(&self) -> Result<Rate>;

    fn reserve_with_deposit(
        &self,
        new_allocation: u64,
        old_allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>>;
}

#[cfg_attr(test, automock)]
pub trait ReturnCalculator {
    fn calculate_return(
        &self,
        new_allocation: u64,
        old_allocation: u64,
    ) -> Result<Rate>;
}

impl<T> ReturnCalculator for T
where
    T: ReserveAccessor,
{
    fn calculate_return(
        &self,
        new_allocation: u64,
        old_allocation: u64,
    ) -> Result<Rate> {
        let reserve = self.reserve_with_deposit(new_allocation, old_allocation)?;
        reserve.utilization_rate()?.try_mul(reserve.borrow_rate()?).map_err(|e| e.into())
    }
}

#[derive(Clone)]
pub enum Reserves {
    Solend(Box<SolendReserve>),
    Port(Box<PortReserve>),
}

// TODO Is there a cleaner way to do this?
impl<'a> ReserveAccessor for Reserves {
    fn utilization_rate(&self) -> Result<Rate> {
        match self {
            Reserves::Solend(reserve) => reserve.utilization_rate(),
            Reserves::Port(reserve) => reserve.utilization_rate(),
        }
    }

    fn borrow_rate(&self) -> Result<Rate> {
        match self {
            Reserves::Solend(reserve) => reserve.borrow_rate(),
            Reserves::Port(reserve) => reserve.borrow_rate(),
        }
    }

    fn reserve_with_deposit(
        &self,
        new_allocation: u64,
        old_allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>> {
        match self {
            Reserves::Solend(reserve) => {
                reserve.reserve_with_deposit(new_allocation, old_allocation)
            }
            Reserves::Port(reserve) => reserve.reserve_with_deposit(new_allocation, old_allocation),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_calculate_return() {
        let mut mock_ra_inner = MockReserveAccessor::new();
        mock_ra_inner
            .expect_utilization_rate()
            .return_once(move || Ok(Rate::from_percent(50)));
        mock_ra_inner
            .expect_borrow_rate()
            .return_once(move || Ok(Rate::from_percent(80)));

        let mut mock_ra = MockReserveAccessor::new();
        mock_ra
            .expect_reserve_with_deposit()
            .return_once(|_, _| Ok(Box::new(mock_ra_inner)));

        assert_eq!(mock_ra.calculate_return(10, 0).unwrap(), Rate::from_percent(40));
    }
}
