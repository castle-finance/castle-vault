//! A cashiers check example. The funds are immediately withdrawn from a user's
//! account and sent to a program controlled `Check` account, where the funds
//! reside until they are "cashed" by the intended recipient. The creator of
//! the check can cancel the check at any time to get back the funds.

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount};
use std::convert::{Into, TryFrom};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod castle_lending_aggregator {
    use super::*;

    pub fn initialize_pool(ctx: Context<InitializePool>) -> ProgramResult {
        // 
        let (____pool_authority, bump_seed) = Pubkey::find_program_address(
            &[&ctx.accounts.reserve_pool.to_account_info().key.to_bytes()],
            ctx.program_id,
        );   
        let seeds = &[
            &ctx.accounts.reserve_pool.to_account_info().key.to_bytes(),
            &[bump_seed][..],
        ];

        // TODO safety checks

        // Mint initial LP tokens
        let initial_amount = 1000000;
        token::mint_to(
            ctx.accounts.into_mint_to_context().with_signer(&[&seeds[..]]),
            u64::try_from(initial_amount).unwrap()
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
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    pub authority: AccountInfo<'info>,

    #[account(signer, zero)]
    pub reserve_pool: ProgramAccount<'info, ReservePool>,

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
    fn into_mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.pool_mint.to_account_info().clone(),
            to: self.destination.to_account_info().clone(),
            authority: self.authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[account]
pub struct ReservePool {
    pub initializer_key: Pubkey,
    pub initializer_deposit_token_amount: Pubkey,
    pub initializer_receive_token_amount: Pubkey,
    pub initializer_amount: u64,

    // Bump seed used to generate PDA
    pub bump_seed: u8,

    // SPL token program
    pub token_program_id: Pubkey,

    // Account where tokens are stored
    pub token_account: Pubkey,

    // Mint address of pool LP tokens
    pub pool_mint: Pubkey,

    // Mint address of the tokens that are stored in pool
    pub token_mint: Pubkey,
}

#[error]
pub enum ErrorCode {
    #[msg("The given nonce does not create a valid program derived address.")]
    InvalidCheckNonce,
    #[msg("The derived check signer does not match that which was given.")]
    InvalidCheckSigner,
    #[msg("The given check has already been burned.")]
    AlreadyBurned,
}