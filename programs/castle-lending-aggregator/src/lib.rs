use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, MintTo, TokenAccount, Transfer};
use spl_math::precise_number::PreciseNumber;
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

    pub fn deposit(ctx: Context<Deposit>, source_token_amount: u64) -> ProgramResult {
        let reserve_pool = &mut ctx.accounts.reserve_pool;

        let pool_token_supply = PreciseNumber::new(ctx.accounts.pool_mint.supply as u128).unwrap();
        let tokens_in_pool = PreciseNumber::new(ctx.accounts.token.amount as u128).unwrap();
        let source_token_amount_converted = PreciseNumber::new(source_token_amount as u128).unwrap();
        let pool_tokens_to_mint = pool_token_supply.checked_mul(
            &source_token_amount_converted.checked_div(&tokens_in_pool).unwrap()
        ).unwrap().to_imprecise().unwrap();

        let seeds = &[
            &reserve_pool.to_account_info().key.to_bytes(), 
            &[reserve_pool.bump_seed][..],
        ];

        token::transfer(
            ctx.accounts.into_transfer_context(),
            source_token_amount,
        )?;

        token::mint_to(
            ctx.accounts.into_mint_to_context().with_signer(&[&seeds[..]]),
            u64::try_from(pool_tokens_to_mint).unwrap(),
        )?;

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, pool_token_amount: u64) -> ProgramResult {
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

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub reserve_pool: ProgramAccount<'info, ReservePool>,

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
    fn into_mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.pool_mint.to_account_info().clone(),
            to: self.destination.to_account_info().clone(),
            authority: self.authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn into_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.source.to_account_info().clone(),
            to: self.token.to_account_info().clone(),
            authority: self.user_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub reserve_pool: ProgramAccount<'info, ReservePool>,

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