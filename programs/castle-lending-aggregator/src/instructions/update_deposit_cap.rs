use anchor_lang::prelude::*;

use crate::state::Vault;
use std::convert::Into;

#[derive(Accounts)]
pub struct UpdateDepositCap<'info, const N: usize> {
    #[account(
        mut,
        has_one = owner,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,
}

pub fn handler<const N: usize>(
    ctx: Context<UpdateDepositCap<N>>,
    new_deposit_cap: u64,
) -> ProgramResult {
    #[cfg(feature = "debug")]
    msg!("New deposit cap value: {}", new_deposit_cap);

    ctx.accounts.vault.deposit_cap = new_deposit_cap;
    Ok(())
}
