use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use port_anchor_adaptor::PortReserve;
use solend::SolendReserve;

use crate::cpi::solend;
use crate::errors::ErrorCode;
use crate::state::*;

#[derive(Accounts)]
pub struct Rebalance<'info> {
    #[account(
        mut,
        constraint = !vault.last_update.stale @ ErrorCode::VaultIsNotRefreshed,
        has_one = vault_reserve_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    #[account(mut, owner = spl_token_lending::ID)]
    pub solend_reserve_state: Box<Account<'info, SolendReserve>>,

    #[account(mut, owner = port_variable_rate_lending_instructions::ID)]
    pub port_reserve_state: Box<Account<'info, PortReserve>>,
}

pub fn handler(ctx: Context<Rebalance>, to_withdraw_option: u64) -> ProgramResult {
    if to_withdraw_option != 0 {
        // TODO use introspection make sure that there is a withdraw instruction after
    }

    // TODO Calculates ideal allocations and stores in vault
    let reserve_tokens_in_vault = ctx.accounts.vault_reserve_token.amount;
    let reserve_tokens_net = reserve_tokens_in_vault.checked_sub(to_withdraw_option);

    match reserve_tokens_net {
        Some(reserve_tokens_to_deposit) => {
            ctx.accounts.vault.to_reconcile[0].deposit = reserve_tokens_to_deposit
        }
        None => {
            let reserve_tokens_to_redeem = to_withdraw_option
                .checked_sub(reserve_tokens_in_vault)
                .unwrap_or(0);
            let solend_exchange_rate = ctx
                .accounts
                .solend_reserve_state
                .collateral_exchange_rate()?;
            let solend_collateral_amount =
                solend_exchange_rate.liquidity_to_collateral(reserve_tokens_to_redeem)?;
            ctx.accounts.vault.to_reconcile[0].redeem = solend_collateral_amount;
        }
    }

    Ok(())
}
