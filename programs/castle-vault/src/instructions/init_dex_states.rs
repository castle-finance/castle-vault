use anchor_lang::prelude::*;

use std::convert::Into;

use crate::{errors::ErrorCode, state::*};

#[derive(Accounts)]
pub struct InitializeDexStates<'info> {
    #[account(
        mut,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init,
        payer = payer,
        space = 128 + 8,
        seeds = [vault.key().as_ref(), b"dex_states".as_ref()],
        bump,
    )]
    pub dex_states: Box<Account<'info, DexStates>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// Create a PDA that stores DEX status (should only do it once)
pub fn handler(ctx: Context<InitializeDexStates>) -> Result<()> {
    ctx.accounts.vault.dex_states_bump =
        *ctx.bumps.get("dex_states").ok_or(ErrorCode::BumpError)?;
    Ok(())
}
