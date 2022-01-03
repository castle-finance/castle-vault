use anchor_lang::prelude::*;

use anchor_spl::token::TokenAccount;

use crate::cpi::solend;
use crate::state::Vault;

#[derive(Accounts)]
pub struct Rebalance<'info> {
    pub vault_state: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(mut, constraint = vault_reserve_token.owner == *vault_authority.key)]
    pub vault_reserve_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_lp_token: Account<'info, TokenAccount>,

    pub solend_program: AccountInfo<'info>,

    pub solend_market_authority: AccountInfo<'info>,

    pub solend_market: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve_state_account: AccountInfo<'info>,

    #[account(mut)]
    pub solend_lp_mint_account: AccountInfo<'info>,

    #[account(mut)]
    pub solend_deposit_token_account: AccountInfo<'info>,

    pub solend_pyth: AccountInfo<'info>,

    pub solend_switchboard: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    // SPL token program
    pub token_program: AccountInfo<'info>,
}

impl<'info> Rebalance<'info> {
    pub fn solend_deposit_reserve_liquidity_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::DepositReserveLiquidity<'info>> {
        let cpi_accounts = solend::DepositReserveLiquidity {
            lending_program: self.solend_program.clone(),
            source_liquidity: self.vault_reserve_token.to_account_info().clone(),
            destination_collateral_account: self.vault_lp_token.to_account_info().clone(),
            reserve_account: self.solend_reserve_state_account.clone(),
            reserve_collateral_mint: self.solend_lp_mint_account.clone(),
            reserve_liquidity_supply: self.solend_deposit_token_account.clone(),
            lending_market_account: self.solend_market.clone(),
            lending_market_authority: self.solend_market_authority.clone(),
            transfer_authority: self.vault_authority.clone(),
            clock: self.clock.to_account_info().clone(),
            token_program_id: self.token_program.clone(),
        };
        CpiContext::new(self.solend_program.clone(), cpi_accounts)
    }

    pub fn solend_refresh_reserve_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::RefreshReserve<'info>> {
        let cpi_accounts = solend::RefreshReserve {
            lending_program: self.solend_program.clone(),
            reserve: self.solend_reserve_state_account.clone(),
            pyth_reserve_liquidity_oracle: self.solend_pyth.clone(),
            switchboard_reserve_liquidity_oracle: self.solend_switchboard.clone(),
            clock: self.clock.to_account_info().clone(),
        };
        CpiContext::new(self.solend_program.clone(), cpi_accounts)
    }
}

pub fn handler(ctx: Context<Rebalance>) -> ProgramResult {
    // TODO account checking
    // TODO Find highest APY across multiple pools and rebalanace accordingly
    // TODO Refreshes reserve
    
    // TODO Calculates ideal allocations 

    // TODO Withdraws liquidity from lending markets

    let tokens_in_pool = ctx.accounts.vault_reserve_token.clone().amount;

    let reserve_pool = &ctx.accounts.vault_state;
    let seeds = &[
        &reserve_pool.to_account_info().key.to_bytes(), 
        &[reserve_pool.bump_seed][..],
    ];

    // TODO Deposits liquidity to lending markets
    solend::refresh_reserve(ctx.accounts.solend_refresh_reserve_context())?;
    solend::deposit_reserve_liquidity(
        ctx.accounts.solend_deposit_reserve_liquidity_context().with_signer(&[&seeds[..]]),
        tokens_in_pool,
    )?;

    Ok(())
}