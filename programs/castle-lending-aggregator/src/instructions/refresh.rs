use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use port_anchor_adaptor::PortReserve;

use crate::cpi::solend::{self, SolendReserve};
use crate::state::Vault;

#[derive(Accounts)]
pub struct Refresh<'info> {
    #[account(
        mut,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
        has_one = vault_port_lp_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_reserve_token: Account<'info, TokenAccount>,

    pub vault_solend_lp_token: Account<'info, TokenAccount>,

    pub vault_port_lp_token: Account<'info, TokenAccount>,

    #[account(
        executable,
        address = spl_token_lending::ID,
    )]
    pub solend_program: AccountInfo<'info>,

    #[account(mut, owner = solend_program.key())]
    pub solend_reserve_state: Box<Account<'info, SolendReserve>>,

    pub solend_pyth: AccountInfo<'info>,

    pub solend_switchboard: AccountInfo<'info>,

    #[account(
        executable,
        address = port_variable_rate_lending_instructions::ID,
    )]
    pub port_program: AccountInfo<'info>,

    #[account(mut, owner = port_program.key())]
    pub port_reserve_state: Box<Account<'info, PortReserve>>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info> Refresh<'info> {
    pub fn solend_refresh_reserve_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::RefreshReserve<'info>> {
        CpiContext::new(
            self.solend_program.clone(),
            solend::RefreshReserve {
                lending_program: self.solend_program.clone(),
                reserve: self.solend_reserve_state.to_account_info(),
                pyth_reserve_liquidity_oracle: self.solend_pyth.clone(),
                switchboard_reserve_liquidity_oracle: self.solend_switchboard.clone(),
                clock: self.clock.to_account_info(),
            },
        )
    }

    pub fn port_refresh_reserve_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, port_anchor_adaptor::RefreshReserve<'info>> {
        CpiContext::new(
            self.port_program.clone(),
            port_anchor_adaptor::RefreshReserve {
                reserve: self.port_reserve_state.to_account_info(),
                clock: self.clock.to_account_info(),
            },
        )
    }
}

pub fn handler(ctx: Context<Refresh>) -> ProgramResult {
    // TODO redeem liquidity mining rewards

    solend::refresh_reserve(ctx.accounts.solend_refresh_reserve_context())?;
    port_anchor_adaptor::refresh_port_reserve(ctx.accounts.port_refresh_reserve_context())?;

    let vault = &mut ctx.accounts.vault;

    let vault_reserve_token_amount = ctx.accounts.vault_reserve_token.amount;

    let solend_exchange_rate = ctx
        .accounts
        .solend_reserve_state
        .collateral_exchange_rate()?;
    let solend_value =
        solend_exchange_rate.collateral_to_liquidity(ctx.accounts.vault_solend_lp_token.amount)?;

    let port_exchange_rate = ctx.accounts.port_reserve_state.collateral_exchange_rate()?;
    let port_value =
        port_exchange_rate.collateral_to_liquidity(ctx.accounts.vault_port_lp_token.amount)?;

    vault.total_value = vault_reserve_token_amount + solend_value + port_value;
    vault.last_update.update_slot(ctx.accounts.clock.slot);

    Ok(())
}
