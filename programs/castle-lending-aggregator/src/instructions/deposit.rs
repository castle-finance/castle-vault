use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

use std::convert::Into;

use crate::errors::ErrorCode;
use crate::math::calc_deposit_to_vault;
use crate::state::*;

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub reserve_pool: Box<Account<'info, ReservePool>>,

    pub authority: AccountInfo<'info>,

    #[account(signer)]
    pub user_authority: AccountInfo<'info>,

    // Account from which tokens are transferred
    #[account(mut)]
    pub source: Account<'info, TokenAccount>,

    // Account where tokens in pool are stored
    #[account(mut)]
    pub token: Account<'info, TokenAccount>,

    // Account where pool LP tokens are minted to 
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,

    // Mint address of pool LP token
    #[account(mut)]
    pub pool_mint: Account<'info, Mint>,

    // SPL token program
    pub token_program: AccountInfo<'info>,
}

impl<'info> Deposit<'info> {
    fn mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.pool_mint.to_account_info().clone(),
            to: self.destination.to_account_info().clone(),
            authority: self.authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.source.to_account_info().clone(),
            to: self.token.to_account_info().clone(),
            authority: self.user_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

pub fn handler(ctx: Context<Deposit>, reserve_token_amount: u64) -> ProgramResult {
    // TODO handle case where there is no pool token supply
    let reserve_pool = &ctx.accounts.reserve_pool;
    // TODO calculate total vault value
    let reserve_tokens_in_vault = ctx.accounts.token.amount;

    let lp_tokens_to_mint = calc_deposit_to_vault(
        reserve_token_amount, 
        ctx.accounts.pool_mint.supply, 
        reserve_tokens_in_vault,
    ).ok_or(ErrorCode::MathError)?;

    let seeds = &[
        &reserve_pool.to_account_info().key.to_bytes(), 
        &[reserve_pool.bump_seed][..],
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