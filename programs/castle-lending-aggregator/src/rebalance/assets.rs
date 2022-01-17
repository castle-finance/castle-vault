use std::convert::TryFrom;

use jet::state::Reserve as JetReserve;
use port_anchor_adaptor::PortReserve;
use solana_maths::Rate;

use crate::cpi::SolendReserve;
use crate::errors::ErrorCode::{self, TryFromReserveError};

pub trait Asset {
    fn expected_return(&self) -> Rate;
}

pub struct LendingMarket {}

impl Asset for LendingMarket {
    fn expected_return(&self) -> Rate {
        Rate::zero()
    }
}

impl TryFrom<SolendReserve> for LendingMarket {
    type Error = ErrorCode;

    fn try_from(value: SolendReserve) -> Result<Self, Self::Error> {
        Ok(LendingMarket {})
    }
}

impl TryFrom<PortReserve> for LendingMarket {
    type Error = ErrorCode;

    fn try_from(value: PortReserve) -> Result<Self, Self::Error> {
        Ok(LendingMarket {})
    }
}

impl TryFrom<JetReserve> for LendingMarket {
    type Error = ErrorCode;

    fn try_from(value: JetReserve) -> Result<Self, Self::Error> {
        Ok(LendingMarket {})
    }
}
