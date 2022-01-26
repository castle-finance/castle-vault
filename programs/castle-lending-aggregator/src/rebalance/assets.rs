use std::convert::TryFrom;

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use jet::state::Reserve as JetReserve;
use port_anchor_adaptor::PortReserve;
use solana_maths::{Rate, TryMul};

use crate::cpi::SolendReserve;

pub trait Asset {
    fn expected_return(&self) -> Option<Rate>;
}

pub struct LendingMarket {
    utilization_rate: Rate,
    borrow_rate: Rate,
}

impl Asset for LendingMarket {
    fn expected_return(&self) -> Option<Rate> {
        // TODO add liquidity mining rewards
        self.utilization_rate.try_mul(self.borrow_rate).ok()
    }
}

impl TryFrom<SolendReserve> for LendingMarket {
    type Error = solana_program::program_error::ProgramError;

    fn try_from(value: SolendReserve) -> Result<Self, Self::Error> {
        let utilization_rate = value.liquidity.utilization_rate()?;
        let borrow_rate = value.current_borrow_rate()?;

        let converted_utilization_rate =
            Rate::from_scaled_val(utilization_rate.to_scaled_val() as u64);
        let converted_borrow_rate = Rate::from_scaled_val(borrow_rate.to_scaled_val() as u64);

        //msg!("solend util {:?}", converted_utilization_rate);
        //msg!("solend port borrow {:?}", converted_borrow_rate);

        Ok(LendingMarket {
            utilization_rate: converted_utilization_rate,
            borrow_rate: converted_borrow_rate,
        })
    }
}

impl TryFrom<PortReserve> for LendingMarket {
    type Error = solana_program::program_error::ProgramError;

    fn try_from(value: PortReserve) -> Result<Self, Self::Error> {
        let utilization_rate = value.liquidity.utilization_rate()?;
        let borrow_rate = value.current_borrow_rate()?;

        let converted_utilization_rate =
            Rate::from_scaled_val(utilization_rate.to_scaled_val() as u64);
        let converted_borrow_rate = Rate::from_scaled_val(borrow_rate.to_scaled_val() as u64);

        //msg!("port util {:?}", converted_utilization_rate);
        //msg!("port borrow {:?}", converted_borrow_rate);

        Ok(LendingMarket {
            utilization_rate: converted_utilization_rate,
            borrow_rate: converted_borrow_rate,
        })
    }
}

impl TryFrom<JetReserve> for LendingMarket {
    type Error = ProgramError;

    fn try_from(value: JetReserve) -> Result<Self, Self::Error> {
        let vault_total = value.total_deposits();
        let outstanding_debt = *value.unwrap_outstanding_debt(Clock::get()?.slot);

        let utilization_rate = jet::state::utilization_rate(outstanding_debt, vault_total);
        let borrow_rate = value.interest_rate(outstanding_debt, vault_total);

        let converted_util = Rate::from_bips(utilization_rate.as_u64(-4));
        let converted_borrow = Rate::from_bips(borrow_rate.as_u64(-4));

        //msg!("jet util {:?}", converted_util);
        //msg!("jet borrow {:?}", converted_borrow);

        Ok(LendingMarket {
            utilization_rate: converted_util,
            borrow_rate: converted_borrow,
        })
    }
}
