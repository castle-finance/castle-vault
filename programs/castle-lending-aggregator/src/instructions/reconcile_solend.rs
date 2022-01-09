use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount};

use crate::{cpi::solend, state::Vault};

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

    fn solend_redeem_reserve_collateral_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::RedeemReserveCollateral<'info>> {
        CpiContext::new(
            self.solend_program.clone(),
            solend::RedeemReserveCollateral {
                lending_program: self.solend_program.clone(),
                source_collateral: self.vault_solend_lp_token.to_account_info(),
                destination_liquidity: self.vault_reserve_token.to_account_info(),
                refreshed_reserve_account: self.solend_reserve_state.clone(),
                reserve_collateral_mint: self.solend_lp_mint.clone(),
                reserve_liquidity: self.solend_reserve_token.clone(),
                lending_market: self.solend_market.clone(),
                lending_market_authority: self.solend_market_authority.clone(),
                user_transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program_id: self.token_program.clone(),
            },
        )
    }
}

pub fn handler(ctx: Context<ReconcileSolend>) -> ProgramResult {
    let vault = &ctx.accounts.vault;

    let deposit_amount = vault.to_reconcile[0].deposit;
    let redeem_amount = vault.to_reconcile[0].redeem;

    if deposit_amount > 0 {
        solend::deposit_reserve_liquidity(
            ctx.accounts
                .solend_deposit_reserve_liquidity_context()
                .with_signer(&[&vault.authority_seeds()]),
            deposit_amount,
        )?;
    }
    if redeem_amount > 0 {
        solend::redeem_reserve_collateral(
            ctx.accounts
                .solend_redeem_reserve_collateral_context()
                .with_signer(&[&vault.authority_seeds()]),
            redeem_amount,
        )?;
    }

    ctx.accounts.vault.to_reconcile[0].reset();

    Ok(())
}
