use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount};

use crate::cpi::solend;
use crate::state::Vault;

#[derive(Accounts)]
pub struct Rebalance<'info> {
    #[account(
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub vault_reserve_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_solend_lp_token: Account<'info, TokenAccount>,

    #[account(
        executable,
        address = spl_token_lending::ID,
    )]
    pub solend_program: AccountInfo<'info>,

    pub solend_market_authority: AccountInfo<'info>,

    #[account(owner = solend_program.key())]
    pub solend_market: AccountInfo<'info>,

    #[account(mut, owner = solend_program.key())]
    pub solend_reserve_state: AccountInfo<'info>,

    #[account(mut)]
    pub solend_lp_mint: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve_token: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    #[account(address = token::ID)]
    pub token_program: AccountInfo<'info>,
}

impl<'info> Rebalance<'info> {
    pub fn solend_deposit_reserve_liquidity_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::DepositReserveLiquidity<'info>> {
        CpiContext::new(
            self.solend_program.clone(),
            solend::DepositReserveLiquidity {
                lending_program: self.solend_program.clone(),
                source_liquidity: self.vault_reserve_token.to_account_info().clone(),
                destination_collateral_account: self.vault_solend_lp_token.to_account_info(),
                reserve_account: self.solend_reserve_state.clone(),
                reserve_collateral_mint: self.solend_lp_mint.clone(),
                reserve_liquidity_supply: self.solend_reserve_token.clone(),
                lending_market_account: self.solend_market.clone(),
                lending_market_authority: self.solend_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info().clone(),
                token_program_id: self.token_program.clone(),
            },
        )
    }
}

pub fn handler(ctx: Context<Rebalance>) -> ProgramResult {
    // TODO Find highest APY across multiple pools and rebalanace accordingly
    // TODO Refreshes reserve
    
    // TODO Calculates ideal allocations 

    // TODO Withdraws liquidity from lending markets

    let tokens_in_pool = ctx.accounts.vault_reserve_token.amount;

    let vault = &ctx.accounts.vault;

    // TODO Deposits liquidity to lending markets
    solend::deposit_reserve_liquidity(
        ctx.accounts.solend_deposit_reserve_liquidity_context().with_signer(
            &[&vault.authority_seeds()]
        ),
        tokens_in_pool,
    )?;

    Ok(())
}