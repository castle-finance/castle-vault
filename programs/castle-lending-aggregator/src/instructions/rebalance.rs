use anchor_lang::prelude::*;

use anchor_spl::token::TokenAccount;

use crate::cpi::spl::{deposit_reserve_liquidity, DepositReserveLiquidity};
use crate::state::ReservePool;

#[derive(Accounts)]
#[instruction(nonce: u8, _bump: u8)]
pub struct Rebalance<'info> {
    pub reserve_pool: Box<Account<'info, ReservePool>>,

    pub authority: AccountInfo<'info>,

    pub lending_program: AccountInfo<'info>,

    #[account(mut)]
    pub pool_deposit_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub pool_lp_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub lending_market_reserve_state_account: AccountInfo<'info>,

    #[account(mut)]
    pub lending_market_lp_mint_account: AccountInfo<'info>,

    #[account(mut)]
    pub lending_market_deposit_token_account: AccountInfo<'info>,

    pub lending_market: AccountInfo<'info>,

    pub lending_market_authority: AccountInfo<'info>,

    // Clock
    pub clock: Sysvar<'info, Clock>,

    // SPL token program
    pub token_program: AccountInfo<'info>,
}

impl<'info> Rebalance<'info> {
    pub fn deposit_reserve_liquidity_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, DepositReserveLiquidity<'info>> {
        let cpi_accounts = DepositReserveLiquidity {
            lending_program: self.lending_program.clone(),
            source_liquidity: self.pool_deposit_token_account.to_account_info().clone(),
            destination_collateral_account: self
                .pool_lp_token_account
                .to_account_info()
                .clone(),
            reserve_account: self.lending_market_reserve_state_account.clone(),
            reserve_collateral_mint: self.lending_market_lp_mint_account.clone(),
            reserve_liquidity_supply: self.lending_market_deposit_token_account.clone(),
            lending_market_account: self.lending_market.clone(),
            lending_market_authority: self.lending_market_authority.clone(),
            transfer_authority: self.authority.clone(),
            clock: self.clock.to_account_info().clone(),
            token_program_id: self.token_program.clone(),
        };
        CpiContext::new(self.lending_program.clone(), cpi_accounts)
    }
}

pub fn handler(ctx: Context<Rebalance>) -> ProgramResult {
    // Forward tokens from reserve pool to lending market
    // TODO account checking
    // TODO Find highest APY across multiple pools and rebalanace accordingly
    let reserve_pool = &mut ctx.accounts.reserve_pool;

    let tokens_in_pool = ctx.accounts.pool_deposit_token_account.amount;

    let seeds = &[
        &reserve_pool.to_account_info().key.to_bytes(), 
        &[reserve_pool.bump_seed][..],
    ];

    deposit_reserve_liquidity(
        ctx.accounts.deposit_reserve_liquidity_context().with_signer(&[&seeds[..]]),
        tokens_in_pool,
    )?;

    Ok(())
}