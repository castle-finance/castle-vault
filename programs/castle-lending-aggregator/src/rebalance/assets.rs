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
    pub solend: LendingMarket,
    pub port: LendingMarket,
    pub jet: LendingMarket,
}
impl_provider_index!(Assets, LendingMarket);

impl Assets {
    pub fn len(&self) -> usize {
        Provider::iter().len()
    }
}

pub struct LendingMarket(pub Box<dyn ReturnCalculator>);

impl ReturnCalculator for LendingMarket {
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError> {
        self.0.calculate_return(allocation)
    }
}

pub trait ReturnCalculator {
    // TODO remove solana-specific error types
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError>;
}

impl ReturnCalculator for SolendReserve {
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError> {
        let mut reserve = self.clone();
        reserve.liquidity.deposit(allocation)?;

        let utilization_rate =
            Rate::from_scaled_val(reserve.liquidity.utilization_rate()?.to_scaled_val() as u64);
        let borrow_rate =
            Rate::from_scaled_val(reserve.current_borrow_rate()?.to_scaled_val() as u64);

        utilization_rate.try_mul(borrow_rate)
    }
}

impl ReturnCalculator for PortReserve {
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError> {
        let mut reserve = self.clone();
        reserve.liquidity.available_amount = reserve
            .liquidity
            .available_amount
            .checked_add(allocation)
            .ok_or(ErrorCode::OverflowError)?;

        let utilization_rate =
            Rate::from_scaled_val(reserve.liquidity.utilization_rate()?.to_scaled_val() as u64);
        let borrow_rate =
            Rate::from_scaled_val(reserve.current_borrow_rate()?.to_scaled_val() as u64);

        utilization_rate.try_mul(borrow_rate)
    }
}

impl ReturnCalculator for JetReserve {
    fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError> {
        let vault_total = self
            .total_deposits()
            .checked_add(allocation)
            .ok_or(ErrorCode::OverflowError)?;
        let outstanding_debt = *self.unwrap_outstanding_debt(Clock::get()?.slot);

        let utilization_rate =
            Rate::from_bips(jet::state::utilization_rate(outstanding_debt, vault_total).as_u64(-4));
        let borrow_rate =
            Rate::from_bips(self.interest_rate(outstanding_debt, vault_total).as_u64(-4));

        utilization_rate.try_mul(borrow_rate)
    }
}
