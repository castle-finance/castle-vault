use anchor_lang::prelude::*;

use std::convert::Into;

use crate::state::*;

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializePortAdditionalState<'info> {
    #[account(
        mut,
        has_one = owner
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"port_additional_state".as_ref()],
        bump = bump,
    )]
    pub port_additional_states: Box<Account<'info, VaultPortAdditionalState>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializePortAdditionalState>, bump: u8) -> ProgramResult {
    ctx.accounts.vault.vault_port_additional_state_bump = bump;
    Ok(())
}
