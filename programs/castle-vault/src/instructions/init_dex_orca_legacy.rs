use anchor_lang::prelude::*;
use std::convert::Into;

use crate::state::*;

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializeDexOrcaLegacy<'info> {
    #[account(
        mut,
        has_one = owner,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"dex_states".as_ref()], 
        bump = vault.dex_states_bump
    )]
    pub dex_states: Box<Account<'info, DexStates>>,

    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"dex_orca_legacy".as_ref()],
        bump = bump
    )]
    pub orca_legacy_accounts: Box<Account<'info, OrcaLegacyAccounts>>,

    pub orca_swap_state: AccountInfo<'info>,

    pub orca_swap_authority: AccountInfo<'info>,

    pub orca_input_token_account: AccountInfo<'info>,

    pub orca_output_token_account: AccountInfo<'info>,

    pub orca_swap_token_mint: AccountInfo<'info>,

    #[account(executable)]
    pub orca_swap_program: AccountInfo<'info>,

    /// Account that pays for above account inits
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Owner of the vault
    /// Only this account can call restricted instructions
    /// Acts as authority of the fee receiver account
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>
}

pub fn handler(ctx: Context<InitializeDexOrcaLegacy>, bump: u8) -> ProgramResult {
    ctx.accounts.dex_states.orca_legacy_accounts_bump = bump;
    ctx.accounts.orca_legacy_accounts.orca_swap_program = ctx.accounts.orca_swap_program.key();
    ctx.accounts.orca_legacy_accounts.orca_swap_state = ctx.accounts.orca_swap_state.key();
    ctx.accounts.orca_legacy_accounts.orca_swap_authority = ctx.accounts.orca_swap_authority.key();
    ctx.accounts.orca_legacy_accounts.orca_input_token_account =
        ctx.accounts.orca_input_token_account.key();
    ctx.accounts.orca_legacy_accounts.orca_output_token_account =
        ctx.accounts.orca_output_token_account.key();
    ctx.accounts.orca_legacy_accounts.orca_swap_token_mint =
        ctx.accounts.orca_swap_token_mint.key();
    Ok(())
}
