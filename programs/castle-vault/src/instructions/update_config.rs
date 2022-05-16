use anchor_lang::prelude::*;

use std::convert::Into;

use crate::state::{Vault, VaultConfig};

use super::VaultConfigArg;

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        has_one = owner,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,
}

pub fn handler(ctx: Context<UpdateConfig>, config: VaultConfigArg) -> ProgramResult {
    #[cfg(feature = "debug")]
    msg!("New config: {:?}", config);

    ctx.accounts.vault.config = VaultConfig::new(config)?;
    ctx.accounts.vault.adjust_allocation_cap()
}
