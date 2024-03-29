use anchor_lang::prelude::*;

use crate::state::Vault;

#[derive(Accounts)]
pub struct UpdateYieldSourceFlags<'info> {
    #[account(
        mut,
        has_one = owner,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,
}

pub fn handler(ctx: Context<UpdateYieldSourceFlags>, flags: u16) -> Result<()> {
    #[cfg(feature = "debug")]
    msg!("New yield source flags: {:?}", flags);

    ctx.accounts.vault.set_yield_source_flags(flags)
}
