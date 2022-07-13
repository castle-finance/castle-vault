use anchor_lang::prelude::*;
use std::convert::Into;

use crate::{errors::ErrorCode, state::*};

#[derive(Accounts)]
pub struct InitializeDexOrcaLegacyMarket<'info> {
    #[account(
        mut,
        has_one = owner,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"dex_states".as_ref()], 
        bump = vault.dex_states_bump
    )]
    pub dex_states: Box<Account<'info, DexStates>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"dex_orca_legacy".as_ref()],
        bump = dex_states.orca_legacy_accounts_bump
    )]
    pub orca_legacy_accounts: Box<Account<'info, OrcaLegacyAccounts>>,

    pub orca_swap_state: AccountInfo<'info>,

    /// Owner of the vault
    /// Only this account can call restricted instructions
    /// Acts as authority of the fee receiver account
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializeDexOrcaLegacyMarket>, market_id: u8) -> ProgramResult {
    if market_id as usize > ctx.accounts.orca_legacy_accounts.orca_markets.len() {
        msg!("Invalid market Id");
        return Err(ErrorCode::InvalidAccount.into());
    }
    ctx.accounts.orca_legacy_accounts.orca_markets[market_id as usize] =
        ctx.accounts.orca_swap_state.key();
    Ok(())
}
