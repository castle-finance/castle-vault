use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::cpi::{solend, solend_accessor};
use crate::state::Vault;

#[derive(Accounts)]
pub struct Refresh<'info> {
    #[account(
        mut,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(mut)]
    pub vault_reserve_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_solend_lp_token: Account<'info, TokenAccount>,

    #[account(
        executable,
        address = spl_token_lending::ID,
    )]
    pub solend_program: AccountInfo<'info>,

    #[account(mut, owner = solend_program.key())]
    pub solend_reserve_state: AccountInfo<'info>,

    pub solend_pyth: AccountInfo<'info>,

    pub solend_switchboard: AccountInfo<'info>,

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
                reserve: self.solend_reserve_state.clone(),
                pyth_reserve_liquidity_oracle: self.solend_pyth.clone(),
                switchboard_reserve_liquidity_oracle: self.solend_switchboard.clone(),
                clock: self.clock.to_account_info(),
            },
        )
    }
}

pub fn handler(ctx: Context<Refresh>) -> ProgramResult {
    // TODO redeem liquidity mining rewards

    solend::refresh_reserve(ctx.accounts.solend_refresh_reserve_context())?;

    let vault = &mut ctx.accounts.vault;
    vault.total_value = get_vault_value(
        ctx.accounts.vault_reserve_token.clone(),
        ctx.accounts.vault_solend_lp_token.clone(),
        ctx.accounts.solend_reserve_state.clone(),
    )?;
    vault.last_update.update_slot(ctx.accounts.clock.slot);

    Ok(())
}

pub fn get_vault_value(
    vault_reserve_token_account: Account<TokenAccount>,
    vault_solend_lp_token_account: Account<TokenAccount>,
    solend_reserve_state_account: AccountInfo,
) -> Result<u64, ProgramError> {
    let vault_reserve_token_amount = vault_reserve_token_account.amount;
    let solend_exchange_rate = solend_accessor::exchange_rate(&solend_reserve_state_account)?;
    let solend_value = solend_exchange_rate.collateral_to_liquidity(
        vault_solend_lp_token_account.amount,
    )?;

    Ok(vault_reserve_token_amount + solend_value)
}