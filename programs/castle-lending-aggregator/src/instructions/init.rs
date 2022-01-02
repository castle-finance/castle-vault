use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount};

use std::convert::Into;

use crate::state::*;

#[derive(Accounts)]
pub struct InitializePool<'info> {
    pub authority: AccountInfo<'info>,

    #[account(signer, zero)]
    pub reserve_pool: Box<Account<'info, ReservePool>>,

    // Mint address of pool LP token
    #[account(mut)]
    pub pool_mint: Account<'info, Mint>,

    // Account where tokens in pool are stored
    #[account(mut)]
    pub token: Account<'info, TokenAccount>,

    // Account where pool LP tokens are minted to 
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,

    // SPL token program
    pub token_program: AccountInfo<'info>,    
}

// Context for calling token mintTo
impl<'info> InitializePool<'info> {
    fn mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.pool_mint.to_account_info().clone(),
            to: self.destination.to_account_info().clone(),
            authority: self.authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

pub fn handler(ctx: Context<InitializePool>) -> ProgramResult {
    let (____pool_authority, bump_seed) = Pubkey::find_program_address(
        &[&ctx.accounts.reserve_pool.to_account_info().key.to_bytes()],
        ctx.program_id,
    );   
    let seeds = &[
        &ctx.accounts.reserve_pool.to_account_info().key.to_bytes(),
        &[bump_seed][..],
    ];

    // TODO safety checks

    // TODO remove this logic and add an init check to deposit
    // Mint initial LP tokens
    // TODO make smaller as to not int overflow with more $ in pool
    let initial_amount:u64 = 1000000;
    token::mint_to(
        ctx.accounts.mint_to_context().with_signer(&[&seeds[..]]),
        initial_amount,
    )?;

    // Initialize reserve pool
    let reserve_pool = &mut ctx.accounts.reserve_pool;
    reserve_pool.bump_seed = bump_seed;
    reserve_pool.token_program_id = *ctx.accounts.token_program.key;
    reserve_pool.token_account = *ctx.accounts.token.to_account_info().key;
    reserve_pool.pool_mint = *ctx.accounts.pool_mint.to_account_info().key;
    reserve_pool.token_mint = ctx.accounts.token.mint;

    Ok(())
}