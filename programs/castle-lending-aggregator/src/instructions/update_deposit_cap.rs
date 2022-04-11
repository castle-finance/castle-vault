use anchor_lang::prelude::*;

use crate::state::Vault;
use std::convert::Into;

#[derive(Accounts)]
pub struct UpdateDepositCap<'info> {
    #[account(
        mut,
        has_one = owner,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,
}

pub fn handler(ctx: Context<UpdateDepositCap>, deposit_cap_new_value: u64) -> ProgramResult {
    #[cfg(feature = "debug")]
    msg!("New deposit cap value: {}", deposit_cap_new_value);

    ctx.accounts.vault.pool_size_limit = deposit_cap_new_value;
    Ok(())
}
