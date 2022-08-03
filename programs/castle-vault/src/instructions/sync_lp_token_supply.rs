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

// Sync vault-tracked lp_token_supply with the value provided by the lp token mint
// Needed because when updating existing vault to newer version, the vault-tracked lp_token_supply can be outdated.
// TODO consider removing this when no longer needed (e.g. when all vaults are updated)
pub fn handler(ctx: Context<SyncLpTokenSupply>) -> Result<()> {
    #[cfg(feature = "debug")]
    msg!("Sync vault.lp_token_supply with lp_token_mint.supply");

    ctx.accounts.vault.lp_token_supply = ctx.accounts.lp_token_mint.supply;
    Ok(())
}
