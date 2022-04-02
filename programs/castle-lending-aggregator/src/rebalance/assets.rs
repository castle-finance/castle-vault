use std::convert::TryFrom;

use anchor_lang::prelude::*;
use jet::state::Reserve as JetReserve;
use port_anchor_adaptor::PortReserve;
use solana_maths::{Rate, TryMul};
use strum::IntoEnumIterator;

use crate::{adapters::SolendReserve, impl_provider_index, state::Provider};

#[derive(Debug, Clone, Copy)]
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

pub trait Asset {
    // TODO remove solana-specific error types
    fn expected_return(&self, allocation: u64) -> Result<Rate, ProgramError>;
    fn provider(&self) -> Provider;
}

// TODO impl Asset for a reserve

#[derive(Debug, Clone, Copy)]
pub struct LendingMarket {
    utilization_rate: Rate,
    borrow_rate: Rate,
    provider: Provider,
}

impl Asset for LendingMarket {
    fn expected_return(&self, allocation: u64) -> Result<Rate, ProgramError> {
        // TODO add liquidity mining rewards
        self.utilization_rate.try_mul(self.borrow_rate)
    }
    fn provider(&self) -> Provider {
        self.provider
    }
}

// TODO move these implementations and Provider out of this module
impl TryFrom<&SolendReserve> for LendingMarket {
    type Error = ProgramError;

    fn try_from(value: &SolendReserve) -> Result<Self, Self::Error> {
        let utilization_rate = value.liquidity.utilization_rate()?;
        let borrow_rate = value.current_borrow_rate()?;

        let converted_utilization_rate =
            Rate::from_scaled_val(utilization_rate.to_scaled_val() as u64);
        let converted_borrow_rate = Rate::from_scaled_val(borrow_rate.to_scaled_val() as u64);

        #[cfg(feature = "debug")]
        {
            msg!("solend util {:?}", converted_utilization_rate);
            msg!("solend port borrow {:?}", converted_borrow_rate);
        }

        Ok(LendingMarket {
            utilization_rate: converted_utilization_rate,
            borrow_rate: converted_borrow_rate,
            provider: Provider::Solend,
        })
    }
}

impl TryFrom<&PortReserve> for LendingMarket {
    type Error = ProgramError;

    fn try_from(value: &PortReserve) -> Result<Self, Self::Error> {
        let utilization_rate = value.liquidity.utilization_rate()?;
        let borrow_rate = value.current_borrow_rate()?;

        let converted_utilization_rate =
            Rate::from_scaled_val(utilization_rate.to_scaled_val() as u64);
        let converted_borrow_rate = Rate::from_scaled_val(borrow_rate.to_scaled_val() as u64);

        #[cfg(feature = "debug")]
        {
            msg!("port util {:?}", converted_utilization_rate);
            msg!("port borrow {:?}", converted_borrow_rate);
        }

        Ok(LendingMarket {
            utilization_rate: converted_utilization_rate,
            borrow_rate: converted_borrow_rate,
            provider: Provider::Port,
        })
    }
}

impl TryFrom<&JetReserve> for LendingMarket {
    type Error = ProgramError;

    fn try_from(value: &JetReserve) -> Result<Self, Self::Error> {
        let vault_total = value.total_deposits();
        let outstanding_debt = *value.unwrap_outstanding_debt(Clock::get()?.slot);

        let utilization_rate = jet::state::utilization_rate(outstanding_debt, vault_total);
        let borrow_rate = value.interest_rate(outstanding_debt, vault_total);

        let converted_util = Rate::from_bips(utilization_rate.as_u64(-4));
        let converted_borrow = Rate::from_bips(borrow_rate.as_u64(-4));

        #[cfg(feature = "debug")]
        {
            msg!("jet util {:?}", converted_util);
            msg!("jet borrow {:?}", converted_borrow);
        }

        Ok(LendingMarket {
            utilization_rate: converted_util,
            borrow_rate: converted_borrow,
            provider: Provider::Jet,
        })
    }
}
