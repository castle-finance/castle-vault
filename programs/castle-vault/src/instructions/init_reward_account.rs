use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Token, TokenAccount},
};

use std::convert::Into;

use crate::state::*;

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializeRewardAccount<'info> {
    #[account(
        mut,
        has_one = owner,
        has_one = vault_authority,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    /// Token account for storing Port liquidity mining reward
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), port_native_token_mint.key().as_ref()],
        bump = bump,
        token::authority = vault_authority,
        token::mint = port_native_token_mint,
    )]
    pub vault_port_reward_token: Box<Account<'info, TokenAccount>>,

    /// Mint of the port finance token (liquidity reward will be issued by this one)
    pub port_native_token_mint: AccountInfo<'info>,

    /// Account that pays for above account inits
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Owner of the vault
    /// Only this account can call restricted instructions
    /// Acts as authority of the fee receiver account
    pub owner: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<InitializeRewardAccount>, _bump: u8) -> ProgramResult {
    ctx.accounts.vault.vault_port_reward_token = ctx.accounts.vault_port_reward_token.key();
    Ok(())
}
