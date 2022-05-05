#![allow(dead_code)]
#![allow(unused_imports)]

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use port_anchor_adaptor::{port_lending_id, PortReserve};

use crate::adapters::{solend, SolendReserve};
use crate::errors::ErrorCode;
use crate::state::Vault;

pub trait Refresher {
    fn update_actual_allocation(&mut self, use_oracle: bool) -> ProgramResult;
}

/// Refreshes the reserves of downstream lending markets,
/// updates the vault total value, and collects fees
pub fn handler<'info, T: Refresher>(
    ctx: Context<'_, '_, '_, 'info, T>,
    use_port_oracle: bool,
) -> ProgramResult {
    #[cfg(feature = "debug")]
    msg!("Refreshing lending pool");

    ctx.accounts.update_actual_allocation(use_port_oracle)
}
