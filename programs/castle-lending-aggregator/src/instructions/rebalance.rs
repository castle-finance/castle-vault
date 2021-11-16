use anchor_lang::prelude::*;

use anchor_lending::cpi::{deposit_reserve_liquidity, DepositReserveLiquidity};
use anchor_spl::token::TokenAccount;

use crate::state::*;

#[derive(Accounts)]
#[instruction(nonce: u8, _bump: u8)]
pub struct Rebalance<'info> {
    pub reserve_pool: ProgramAccount<'info, ReservePool>,

    #[account(signer)]
    pub authority: AccountInfo<'info>,

    // Account where tokens in pool are stored
    #[account(mut)]
    pub pool_token_acount: Account<'info, TokenAccount>,

    pub lending_program: AccountInfo<'info>,

    // Solend CPI accounts
    // Token account for asset to deposit into reserve and make sure account owner is transfer authority PDA
    #[account(
        //constraint = source_liquidity.amount >= liquidity_amount,
        constraint = source_liquidity.owner == *transfer_authority.key
    )]
    pub source_liquidity: Account<'info, TokenAccount>,
    // Token account for reserve collateral token
    // Make sure it has a 0 balance to ensure empty account and make sure account owner is transfer authority PDA
    #[account(
        constraint = destination_collateral.amount == 0,
        constraint = destination_collateral.owner == *transfer_authority.key,
    )]
    pub destination_collateral: Account<'info, TokenAccount>,

    // Reserve state account
    pub reserve: AccountInfo<'info>,

    // Token mint for reserve collateral token
    pub reserve_collateral_mint: AccountInfo<'info>,

    // Reserve liquidity supply SPL token account
    pub reserve_liquidity_supply: AccountInfo<'info>,

    // Lending market account
    pub lending_market: AccountInfo<'info>,

    // Lending market authority (PDA)
    pub lending_market_authority: AccountInfo<'info>,

    // Transfer authority for source_liquidity and destination_collateral accounts
    #[account(seeds = [&reserve_pool.to_account_info().key.to_bytes(), &[reserve_pool.bump_seed][..]], bump=_bump)]
    pub transfer_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    // Clock
    pub clock: Sysvar<'info, Clock>,

    // SPL token program
    pub token_program: AccountInfo<'info>,
}

impl<'info> Rebalance<'info> {
    pub fn into_deposit_reserve_liquidity_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, DepositReserveLiquidity<'info>> {
        let cpi_accounts = DepositReserveLiquidity {
            lending_program: self.lending_program.clone(),
            source_liquidity: self.source_liquidity.to_account_info().clone(),
            destination_collateral_account: self
                .destination_collateral
                .to_account_info()
                .clone(),
            reserve_account: self.reserve.clone(),
            reserve_collateral_mint: self.reserve_collateral_mint.clone(),
            reserve_liquidity_supply: self.reserve_liquidity_supply.clone(),
            lending_market_account: self.lending_market.clone(),
            lending_market_authority: self.lending_market_authority.clone(),
            transfer_authority: self.transfer_authority.clone(),
            clock: self.clock.to_account_info().clone(),
            token_program_id: self.token_program.clone(),
        };
        CpiContext::new(self.lending_program.clone(), cpi_accounts)
    }
}

pub fn handler(ctx: Context<Rebalance>) -> ProgramResult {
    // Forward tokens from reserve pool to solend
    // TODO Find highest APY across multiple pools and rebalanace accordingly
    let reserve_pool = &mut ctx.accounts.reserve_pool;

    let tokens_in_pool = ctx.accounts.pool_token_acount.amount;

    let seeds = &[
        &reserve_pool.to_account_info().key.to_bytes(), 
        &[reserve_pool.bump_seed][..],
    ];

    deposit_reserve_liquidity(
        ctx.accounts.into_deposit_reserve_liquidity_context().with_signer(&[&seeds[..]]),
        tokens_in_pool,
    )?;

    Ok(())
}