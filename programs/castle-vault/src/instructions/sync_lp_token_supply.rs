use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::state::Vault;

#[event]
pub struct WithdrawEvent {
    vault: Pubkey,
    user: Pubkey,
    amount: u64,
}

#[derive(Accounts)]
pub struct SyncLpTokenSupply<'info> {
    /// Vault state account
    /// Checks that the refresh has been called in the same slot
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = owner,
        has_one = lp_token_mint,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Mint for the vault's lp token
    #[account(mut)]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    pub owner: Signer<'info>,
}

pub fn handler(ctx: Context<SyncLpTokenSupply>) -> Result<()> {
    #[cfg(feature = "debug")]
    msg!("Sync vault.lp_token_supply with lp_token_mint.supply");

    ctx.accounts.vault.lp_token_supply = ctx.accounts.lp_token_mint.supply;
    Ok(())
}
