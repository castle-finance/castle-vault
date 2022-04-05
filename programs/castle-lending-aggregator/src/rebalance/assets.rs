use anchor_lang::prelude::*;
use jet::state::Reserve as JetReserve;
use port_variable_rate_lending_instructions::state::Reserve as PortReserve;
use solana_maths::{Rate, TryMul};
use spl_token_lending::state::Reserve as SolendReserve;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::errors::ErrorCode;

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
        Provider::iter().len()
    }
}

pub trait ReturnCalculator {
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError>;
}
pub struct LendingMarketAsset(pub Box<dyn ReserveAccessor>);

impl ReturnCalculator for LendingMarketAsset {
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError> {
        let reserve = self.0.reserve_with_deposit(allocation)?;
        reserve.utilization_rate()?.try_mul(reserve.borrow_rate()?)
    }
}

pub trait ReserveAccessor {
    fn utilization_rate(&self) -> Result<Rate, ProgramError>;
    fn borrow_rate(&self) -> Result<Rate, ProgramError>;

    fn reserve_with_deposit(
        &self,
        allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>, ProgramError>;
}

impl ReserveAccessor for SolendReserve {
    fn utilization_rate(&self) -> Result<Rate, ProgramError> {
        Ok(Rate::from_scaled_val(
            self.liquidity.utilization_rate()?.to_scaled_val() as u64,
        ))
    }

    fn borrow_rate(&self) -> Result<Rate, ProgramError> {
        Ok(Rate::from_scaled_val(
            self.current_borrow_rate()?.to_scaled_val() as u64,
        ))
    }

    fn reserve_with_deposit(
        &self,
        allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>, ProgramError> {
        let mut reserve = Box::new(self.clone());
        reserve.liquidity.deposit(allocation)?;
        Ok(reserve)
    }
}

impl ReserveAccessor for PortReserve {
    fn utilization_rate(&self) -> Result<Rate, ProgramError> {
        Ok(Rate::from_scaled_val(
            self.liquidity.utilization_rate()?.to_scaled_val() as u64,
        ))
    }

    fn borrow_rate(&self) -> Result<Rate, ProgramError> {
        Ok(Rate::from_scaled_val(
            self.current_borrow_rate()?.to_scaled_val() as u64,
        ))
    }

    fn reserve_with_deposit(
        &self,
        allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>, ProgramError> {
        let mut reserve = Box::new(self.clone());
        reserve.liquidity.available_amount = reserve
            .liquidity
            .available_amount
            .checked_add(allocation)
            .ok_or(ErrorCode::OverflowError)?;
        Ok(reserve)
    }
}

impl ReserveAccessor for JetReserve {
    fn utilization_rate(&self) -> Result<Rate, ProgramError> {
        let vault_amount = self.total_deposits();
        let outstanding_debt = *self.unwrap_outstanding_debt(Clock::get()?.slot);

        Ok(Rate::from_bips(
            jet::state::utilization_rate(outstanding_debt, vault_amount).as_u64(-4),
        ))
    }

    fn borrow_rate(&self) -> Result<Rate, ProgramError> {
        let vault_amount = self.total_deposits();
        let outstanding_debt = *self.unwrap_outstanding_debt(Clock::get()?.slot);

        Ok(Rate::from_bips(
            self.interest_rate(outstanding_debt, vault_amount)
                .as_u64(-4),
        ))
    }

    fn reserve_with_deposit(
        &self,
        allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>, ProgramError> {
        let mut reserve = Box::new(self.clone());
        // We only care about the token amount here
        reserve.deposit(allocation, 0);

        Ok(reserve)
    }
}
