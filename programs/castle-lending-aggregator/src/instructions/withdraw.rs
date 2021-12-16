use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Burn, TokenAccount, Transfer};
use spl_math::precise_number::PreciseNumber;

use std::convert::{Into, TryFrom};

use crate::state::*;

/// TODO modify to withdraw from solend

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
    fn into_burn_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        let cpi_accounts = Burn {
            mint: self.pool_mint.to_account_info().clone(),
            to: self.source.to_account_info().clone(),
            authority: self.user_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn into_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.token.to_account_info().clone(),
            to: self.destination.to_account_info().clone(),
            authority: self.authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

pub fn handler(ctx: Context<Withdraw>, pool_token_amount: u64) -> ProgramResult {
    let reserve_pool = &mut ctx.accounts.reserve_pool;

    let pool_token_supply = PreciseNumber::new(ctx.accounts.pool_mint.supply as u128).unwrap();
    let tokens_in_pool = PreciseNumber::new(ctx.accounts.token.amount as u128).unwrap();
    let pool_token_amount_converted = PreciseNumber::new(pool_token_amount as u128).unwrap();
    let tokens_to_transfer = tokens_in_pool.checked_mul(
        &pool_token_amount_converted.checked_div(&pool_token_supply).unwrap()
    ).unwrap().to_imprecise().unwrap();

    let seeds = &[
        &reserve_pool.to_account_info().key.to_bytes(), 
        &[reserve_pool.bump_seed][..],
    ];

    token::transfer(
        ctx.accounts.into_transfer_context().with_signer(&[&seeds[..]]),
        u64::try_from(tokens_to_transfer).unwrap(),
    )?;

    token::burn(
        ctx.accounts.into_burn_context(),
        pool_token_amount,
    )?;

    Ok(())
}