use anchor_lang::prelude::*;

use std::convert::Into;
use crate::state::Vault;

#[derive(Accounts)]
pub struct UpdateCap<'info> {
    #[account(
        mut,
    )]
    pub vault: Box<Account<'info, Vault>>,
}

pub fn handler(ctx: Context<UpdateCap>, deposit_cap_new_value: u64) -> ProgramResult {
    #[cfg(feature = "debug")]
    msg!("New deposit cap value: {}", deposit_cap_new_value);

    ctx.accounts.vault.pool_size_limit = deposit_cap_new_value;
    Ok(())
}
