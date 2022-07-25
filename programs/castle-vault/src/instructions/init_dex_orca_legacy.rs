use anchor_lang::prelude::*;
use std::convert::Into;

use crate::{errors::ErrorCode, state::*};

use std::mem;

#[derive(Accounts)]
pub struct InitializeDexOrcaLegacy<'info> {
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
        init,
        space = 672 + 8,
        payer = payer,
        seeds = [vault.key().as_ref(), b"dex_orca_legacy".as_ref()],
        bump
    )]
    pub orca_legacy_accounts: Box<Account<'info, OrcaLegacyAccounts>>,

    /// CHECK: safe
    //#[soteria(ignore)]
    #[account(executable)]
    pub orca_swap_program: AccountInfo<'info>,

    /// Account that pays for above account inits
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Owner of the vault
    /// Only this account can call restricted instructions
    /// Acts as authority of the fee receiver account
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// Create a PDA that stores Orca swap information (should only do it once)
pub fn handler(ctx: Context<InitializeDexOrcaLegacy>) -> Result<()> {
    ctx.accounts.dex_states.orca_legacy_accounts_bump = *ctx
        .bumps
        .get("orca_legacy_accounts")
        .ok_or(ErrorCode::BumpError)?;
    // All orca markets have the same program ID
    ctx.accounts.orca_legacy_accounts.orca_swap_program = ctx.accounts.orca_swap_program.key();
    Ok(())
}
