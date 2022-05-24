use anchor_lang::prelude::*;

use std::convert::Into;

use crate::state::Vault;

#[derive(Accounts)]
pub struct UpdateHaltFlags<'info> {
    #[account(
        mut,
        has_one = owner,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,
}

pub fn handler(ctx: Context<UpdateHaltFlags>, flags: u16) -> ProgramResult {
    #[cfg(feature = "debug")]
    msg!("New flags: {:?}", flags);

    ctx.accounts.vault.set_halt_flags(flags)
}
