use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

use std::convert::Into;

use crate::errors::ErrorCode;
use crate::math::calc_deposit_to_vault;
use crate::state::Vault;

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(signer)]
    pub user_authority: AccountInfo<'info>,

    // Account from which tokens are transferred
    #[account(mut)]
    pub user_reserve_token: Account<'info, TokenAccount>,

    // Account where tokens in pool are stored
    #[account(mut)]
    pub vault_reserve_token: Account<'info, TokenAccount>,

    // Account where pool LP tokens are minted to 
    #[account(mut)]
    pub user_lp_token: Account<'info, TokenAccount>,

    // Mint address of pool LP token
    #[account(mut)]
    pub lp_token_mint: Account<'info, Mint>,

    // SPL token program
    pub token_program: AccountInfo<'info>,
}

impl<'info> Deposit<'info> {
    fn mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        CpiContext::new(
            self.token_program.clone(),
            MintTo {
                mint: self.lp_token_mint.to_account_info().clone(),
                to: self.user_lp_token.to_account_info().clone(),
                authority: self.vault_authority.clone(),
            },
        )
    }

    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.clone(),
            Transfer {
                from: self.user_reserve_token.to_account_info().clone(),
                to: self.vault_reserve_token.to_account_info().clone(),
                authority: self.user_authority.clone(),
            },
        )
    }
}

pub fn handler(ctx: Context<Deposit>, reserve_token_amount: u64) -> ProgramResult {
    let vault = &ctx.accounts.vault;

    // TODO check accounts

    // TODO check last update slot

    let lp_tokens_to_mint = calc_deposit_to_vault(
        reserve_token_amount, 
        ctx.accounts.lp_token_mint.supply, 
        vault.total_value,
    ).ok_or(ErrorCode::MathError)?;

    let seeds = &[
        &vault.to_account_info().key.to_bytes(), 
        &[vault.bump_seed][..],
    ];

    token::transfer(
        ctx.accounts.transfer_context(),
        reserve_token_amount,
    )?;

    token::mint_to(
        ctx.accounts.mint_to_context().with_signer(&[&seeds[..]]),
        lp_tokens_to_mint,
    )?;

    Ok(())
}