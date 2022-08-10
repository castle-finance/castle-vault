use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{AssociatedToken},
    token::{Mint, Token, TokenAccount},
};

use std::convert::Into;

use crate::{errors::ErrorCode, state::*};

#[derive(Accounts)]
pub struct InitializeMangoAdditionalState<'info> {
    #[account(
        mut,
        has_one = owner,
        has_one = vault_authority
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        init,
        payer = payer,
        space = 8 + 288,
        seeds = [vault.key().as_ref(), b"mango_additional_state".as_ref()],
        bump,
    )]
    pub mango_additional_state: Box<Account<'info, VaultMangoAdditionalState>>,

    /// Mint for custom mango lp token
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"mango_lp_mint".as_ref()],
        bump,
        mint::authority = vault_authority,
        mint::decimals = reserve_token_mint.decimals,
    )]
    pub mango_lp_token_mint: Box<Account<'info, Mint>>,

    /// Token account for custom mango lp tokens
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), mango_lp_token_mint.key().as_ref()],
        bump,
        token::authority = vault_authority,
        token::mint = mango_lp_token_mint,
    )]
    pub vault_mango_lp_token: Box<Account<'info, TokenAccount>>,

    /// Mint of the token that the vault accepts and stores
    pub reserve_token_mint: Box<Account<'info, Mint>>,

    // TODO add seeds check? vault has_one might suffice
    /// CHECK: safe
    pub vault_authority: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub mango_account: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub mango_group: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub owner: Signer<'info>,

    /// CHECK: safe
    pub mango_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<InitializeMangoAdditionalState>) -> Result<()> {
    ctx.accounts.mango_additional_state.mango_lp_token_mint =
        ctx.accounts.mango_lp_token_mint.key();
    ctx.accounts.vault.vault_mango_lp_token = ctx.accounts.vault_mango_lp_token.key();

    let instruction = mango::instruction::create_mango_account(
        &ctx.accounts.mango_program.key(),
        &ctx.accounts.mango_group.key(),
        &ctx.accounts.mango_account.key(),
        &ctx.accounts.vault_authority.key(),
        &ctx.accounts.system_program.key(),
        &ctx.accounts.payer.key(),
        1
    )?;

    solana_program::program::invoke_signed(
        &instruction,
        &[
            ctx.accounts.mango_program.to_account_info().clone(),
            ctx.accounts.mango_group.to_account_info().clone(),
            ctx.accounts.mango_account.to_account_info().clone(),
            ctx.accounts.vault_authority.to_account_info().clone(),
            ctx.accounts.payer.to_account_info().clone(),
            ctx.accounts.system_program.to_account_info().clone(),
        ],
        &[&ctx.accounts.vault.authority_seeds()],
    )?;

    Ok(())
}
