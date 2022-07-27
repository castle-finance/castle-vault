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
        bump
    )]
    pub dex_states: Box<Account<'info, DexStates>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"dex_orca_legacy".as_ref()],
        bump
    )]
    pub orca_legacy_accounts: Box<Account<'info, OrcaLegacyAccounts>>,

    // NOTE safe to ignore because owner signs this transaction
    /// CHECK: safe
    //#[soteria(ignore)]
    pub orca_swap_state: AccountInfo<'info>,

    /// Owner of the vault
    /// Only this account can call restricted instructions
    /// Acts as authority of the fee receiver account
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// Register a particular Orca swap market with the vault. A market is a token pair that can be swapped on Orca
// The market_id is used to identify the market in the future.
// The vault will use market(s) registered via this interface to sell/buy assets (e.g. sell_port_reward)
pub fn handler(ctx: Context<InitializeDexOrcaLegacyMarket>, market_id: u8) -> Result<()> {
    if market_id as usize > ctx.accounts.orca_legacy_accounts.orca_markets.len() {
        msg!("Invalid market Id");
        return Err(ErrorCode::InvalidArgument.into());
    }
    ctx.accounts.orca_legacy_accounts.orca_markets[market_id as usize] =
        ctx.accounts.orca_swap_state.key();
    Ok(())
}
