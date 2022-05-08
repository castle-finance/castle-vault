#![allow(dead_code)]
#![allow(unused_imports)]

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use port_anchor_adaptor::{port_lending_id, PortReserve};

use crate::{
    adapters::{solend, SolendReserve},
    errors::ErrorCode,
};

pub trait Refresher<'info> {
    fn update_actual_allocation(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> ProgramResult;
}

/// Refreshes the reserves of downstream lending markets
pub fn handler<'info, T: Refresher<'info>>(ctx: Context<'_, '_, '_, 'info, T>) -> ProgramResult {
    #[cfg(feature = "debug")]
    msg!("Refreshing lending pool");

    ctx.accounts
        .update_actual_allocation(ctx.remaining_accounts)
}
