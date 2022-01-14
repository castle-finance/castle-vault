use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount};
use solend::SolendReserve;

use crate::{cpi::solend, errors::ErrorCode, state::Vault};

#[derive(Accounts)]
pub struct ReconcileSolend<'info> {
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        executable,
        address = spl_token_lending::ID,
    )]
    pub solend_program: AccountInfo<'info>,

    pub solend_market_authority: AccountInfo<'info>,

    #[account(owner = solend_program.key())]
    pub solend_market: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve_state: Box<Account<'info, SolendReserve>>,

    #[account(mut)]
    pub solend_lp_mint: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve_token: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    #[account(address = token::ID)]
    pub token_program: AccountInfo<'info>,
}

impl<'info> ReconcileSolend<'info> {
    pub fn solend_deposit_reserve_liquidity_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::DepositReserveLiquidity<'info>> {
        CpiContext::new(
            self.solend_program.clone(),
            solend::DepositReserveLiquidity {
                lending_program: self.solend_program.clone(),
                source_liquidity: self.vault_reserve_token.to_account_info(),
                destination_collateral_account: self.vault_solend_lp_token.to_account_info(),
                reserve: self.solend_reserve_state.to_account_info(),
                reserve_collateral_mint: self.solend_lp_mint.clone(),
                reserve_liquidity_supply: self.solend_reserve_token.clone(),
                lending_market: self.solend_market.clone(),
                lending_market_authority: self.solend_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program_id: self.token_program.clone(),
            },
        )
    }

    fn solend_redeem_reserve_collateral_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::RedeemReserveCollateral<'info>> {
        CpiContext::new(
            self.solend_program.clone(),
            solend::RedeemReserveCollateral {
                lending_program: self.solend_program.clone(),
                source_collateral: self.vault_solend_lp_token.to_account_info(),
                destination_liquidity: self.vault_reserve_token.to_account_info(),
                reserve: self.solend_reserve_state.to_account_info(),
                reserve_collateral_mint: self.solend_lp_mint.clone(),
                reserve_liquidity_supply: self.solend_reserve_token.clone(),
                lending_market: self.solend_market.clone(),
                lending_market_authority: self.solend_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program_id: self.token_program.clone(),
            },
        )
    }
}

pub fn handler(ctx: Context<ReconcileSolend>) -> ProgramResult {
    let vault = &ctx.accounts.vault;

    let solend_exchange_rate = ctx
        .accounts
        .solend_reserve_state
        .collateral_exchange_rate()?;
    let current_solend_value =
        solend_exchange_rate.collateral_to_liquidity(ctx.accounts.vault_solend_lp_token.amount)?;
    let allocation = ctx.accounts.vault.allocations.solend;

    match allocation.checked_sub(current_solend_value) {
        Some(tokens_to_deposit) => {
            solend::deposit_reserve_liquidity(
                ctx.accounts
                    .solend_deposit_reserve_liquidity_context()
                    .with_signer(&[&vault.authority_seeds()]),
                tokens_to_deposit,
            )?;
        }
        None => {
            let tokens_to_redeem = ctx
                .accounts
                .vault_solend_lp_token
                .amount
                .checked_sub(solend_exchange_rate.liquidity_to_collateral(allocation)?)
                .ok_or(ErrorCode::MathError)?;

            solend::redeem_reserve_collateral(
                ctx.accounts
                    .solend_redeem_reserve_collateral_context()
                    .with_signer(&[&vault.authority_seeds()]),
                tokens_to_redeem,
            )?;
        }
    }

    ctx.accounts.vault.allocations.solend = 0 as u64;

    Ok(())
}
