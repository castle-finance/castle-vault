use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount};

use crate::cpi::solend;
use crate::state::Vault;

#[derive(Accounts)]
pub struct Rebalance<'info> {
    #[account(
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub vault_reserve_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_solend_lp_token: Account<'info, TokenAccount>,

    #[account(
        executable,
        address = spl_token_lending::ID,
    )]
    pub solend_program: AccountInfo<'info>,

    pub solend_market_authority: AccountInfo<'info>,

    #[account(owner = solend_program.key())]
    pub solend_market: AccountInfo<'info>,

    #[account(mut, owner = solend_program.key())]
    pub solend_reserve_state: AccountInfo<'info>,

    #[account(mut)]
    pub solend_lp_mint: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve_token: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    #[account(address = token::ID)]
    pub token_program: AccountInfo<'info>,
}

impl<'info> Rebalance<'info> {
}

pub fn handler(ctx: Context<Rebalance>, to_withdraw_option: u64) -> ProgramResult {
    // TODO Find highest APY across multiple pools and rebalanace accordingly
    // TODO Refreshes reserve
    
    let tokens_in_pool = ctx.accounts.vault_reserve_token.amount;

    let vault = &ctx.accounts.vault;
    // TODO Calculates ideal allocations 

    Ok(())
}