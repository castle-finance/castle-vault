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

#[cfg_attr(test, automock)]
pub trait ReserveAccessor {
    fn utilization_rate(&self) -> Result<Rate, ProgramError>;
    fn borrow_rate(&self) -> Result<Rate, ProgramError>;

    fn reserve_with_deposit(
        &self,
        allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>, ProgramError>;
}

#[cfg_attr(test, automock)]
pub trait ReturnCalculator {
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError>;
}

impl<T> ReturnCalculator for T
where
    T: ReserveAccessor,
{
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError> {
        let reserve = self.reserve_with_deposit(allocation)?;
        reserve.utilization_rate()?.try_mul(reserve.borrow_rate()?)
    }
}

#[derive(Clone)]
pub enum Reserves {
    Solend(SolendReserve),
    Port(PortReserve),
    Jet(Box<jet::state::Reserve>),
}

// TODO Is there a cleaner way to do this?
impl<'a> ReserveAccessor for Reserves {
    fn utilization_rate(&self) -> Result<Rate, ProgramError> {
        match self {
            Reserves::Solend(reserve) => reserve.utilization_rate(),
            Reserves::Port(reserve) => reserve.utilization_rate(),
            Reserves::Jet(reserve) => reserve.utilization_rate(),
        }
    }

    fn borrow_rate(&self) -> Result<Rate, ProgramError> {
        match self {
            Reserves::Solend(reserve) => reserve.borrow_rate(),
            Reserves::Port(reserve) => reserve.borrow_rate(),
            Reserves::Jet(reserve) => reserve.borrow_rate(),
        }
    }

    fn reserve_with_deposit(
        &self,
        allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>, ProgramError> {
        match self {
            Reserves::Solend(reserve) => reserve.reserve_with_deposit(allocation),
            Reserves::Port(reserve) => reserve.reserve_with_deposit(allocation),
            Reserves::Jet(reserve) => reserve.reserve_with_deposit(allocation),
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
            .return_const(Ok(Rate::from_percent(50)));
        mock_ra_inner
            .expect_borrow_rate()
            .return_const(Ok(Rate::from_percent(80)));

        let mut mock_ra = MockReserveAccessor::new();
        mock_ra
            .expect_reserve_with_deposit()
            .return_once(|_| Ok(Box::new(mock_ra_inner)));

        assert_eq!(mock_ra.calculate_return(10), Ok(Rate::from_percent(40)));
    }
}