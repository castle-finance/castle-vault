use anchor_lang::prelude::*;

use std::convert::Into;

use crate::state::*;

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializeDexStates<'info> {
    #[account(
        mut,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"dex_states".as_ref()],
        bump = bump,
    )]
    pub dex_states: Box<Account<'info, DexStates>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializeDexStates>, bump: u8) -> ProgramResult {
    ctx.accounts.vault.dex_states_bump = bump;
    Ok(())
}
