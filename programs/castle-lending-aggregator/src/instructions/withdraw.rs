use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Burn, TokenAccount, Transfer};

use std::convert::Into; 

use crate::errors::ErrorCode;
use crate::math::calc_withdraw_from_vault;
use crate::state::*;


#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub reserve_pool: Box<Account<'info, ReservePool>>,

    pub authority: AccountInfo<'info>,

    #[account(signer)]
    pub user_authority: AccountInfo<'info>,

    // Account from which pool tokens are burned
    #[account(mut)]
    pub source: Account<'info, TokenAccount>,

    // Account where tokens in pool are stored
    #[account(mut)]
    pub token: Account<'info, TokenAccount>,

    // Account where tokens are transferred to
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,

    // Mint address of pool LP token
    #[account(mut)]
    pub pool_mint: Account<'info, Mint>,

    // SPL token program
    pub token_program: AccountInfo<'info>,
}

impl<'info> Withdraw<'info> {
    fn burn_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        let cpi_accounts = Burn {
            mint: self.pool_mint.to_account_info().clone(),
            to: self.source.to_account_info().clone(),
            authority: self.user_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.token.to_account_info().clone(),
            to: self.destination.to_account_info().clone(),
            authority: self.authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

pub fn handler(ctx: Context<Withdraw>, lp_token_amount: u64) -> ProgramResult {
    let reserve_pool = &mut ctx.accounts.reserve_pool;

    // TODO check accounts

    // TODO calculate total vault value
    let reserve_tokens_in_vault = ctx.accounts.token.amount;

    let reserve_tokens_to_transfer = calc_withdraw_from_vault(
        lp_token_amount, 
        ctx.accounts.pool_mint.supply, 
        reserve_tokens_in_vault,
    ).ok_or(ErrorCode::MathError)?;

    let seeds = &[
        &reserve_pool.to_account_info().key.to_bytes(), 
        &[reserve_pool.bump_seed][..],
    ];

    // TODO redeem collateral

    // Transfer reserve tokens to user
    token::transfer(
        ctx.accounts.transfer_context().with_signer(&[&seeds[..]]),
            reserve_tokens_to_transfer,
    )?;

    // Burn LP tokens
    token::burn(
        ctx.accounts.burn_context(),
        lp_token_amount,
    )?;
    
    Ok(())
}