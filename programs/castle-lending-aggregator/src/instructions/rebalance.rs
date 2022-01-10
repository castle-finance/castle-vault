use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use port_anchor_adaptor::PortReserve;
use port_variable_rate_lending_instructions::math::TryMul as PortMul;
use solend::SolendReserve;
use spl_token_lending::math::TryMul as SolendMul;
use std::cmp::Ordering;

use crate::cpi::solend;
use crate::errors::ErrorCode;
use crate::state::*;

#[derive(Accounts)]
pub struct Rebalance<'info> {
    #[account(
        mut,
        constraint = !vault.last_update.stale @ ErrorCode::VaultIsNotRefreshed,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
        has_one = vault_port_lp_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    pub vault_solend_lp_token: Account<'info, TokenAccount>,

    pub vault_port_lp_token: Account<'info, TokenAccount>,

    #[account(mut, owner = spl_token_lending::ID)]
    pub solend_reserve_state: Box<Account<'info, SolendReserve>>,

    #[account(mut, owner = port_variable_rate_lending_instructions::ID)]
    pub port_reserve_state: Box<Account<'info, PortReserve>>,
}

pub fn handler(ctx: Context<Rebalance>, to_withdraw_option: u64) -> ProgramResult {
    if to_withdraw_option != 0 {
        // TODO use introspection make sure that there is a withdraw instruction after
    }

    // TODO make this not fucking disgusting

    let mut port_allocation: u64 = 0;
    let mut solend_allocation: u64 = 0;

    let solend_reserve = &ctx.accounts.solend_reserve_state;
    let solend_deposit_rate = solend_reserve
        .current_borrow_rate()?
        .try_mul(solend_reserve.liquidity.utilization_rate()?)?;

    let port_reserve = &ctx.accounts.port_reserve_state;
    let port_deposit_rate = port_reserve
        .current_borrow_rate()?
        .try_mul(port_reserve.liquidity.utilization_rate()?)?;

    match port_deposit_rate
        .to_scaled_val()
        .cmp(&solend_deposit_rate.to_scaled_val())
    {
        Ordering::Greater => port_allocation = 100,
        Ordering::Less => solend_allocation = 100,
        Ordering::Equal => {
            port_allocation = 50;
            solend_allocation = 50;
        }
    }

    // Convert to right units
    let vault_value = ctx
        .accounts
        .vault
        .total_value
        .checked_sub(to_withdraw_option)
        .ok_or(ErrorCode::MathError)?;

    let solend_exchange_rate = solend_reserve.collateral_exchange_rate()?;
    let solend_collateral_amount = solend_exchange_rate.liquidity_to_collateral(
        solend_allocation
            .checked_mul(vault_value)
            .ok_or(ErrorCode::MathError)?
            .checked_div(100)
            .ok_or(ErrorCode::MathError)?,
    )?;
    match ctx
        .accounts
        .vault_solend_lp_token
        .amount
        .checked_sub(solend_collateral_amount)
    {
        Some(collateral_to_redeem) => {
            ctx.accounts.vault.to_reconcile[0].redeem = collateral_to_redeem
        }
        None => {
            ctx.accounts.vault.to_reconcile[0].deposit = solend_exchange_rate
                .collateral_to_liquidity(
                    solend_collateral_amount
                        .checked_sub(ctx.accounts.vault_solend_lp_token.amount)
                        .ok_or(ErrorCode::MathError)?,
                )?
        }
    }

    let port_exchange_rate = port_reserve.collateral_exchange_rate()?;
    let port_collateral_amount = port_exchange_rate.liquidity_to_collateral(
        port_allocation
            .checked_mul(vault_value)
            .ok_or(ErrorCode::MathError)?
            .checked_div(100)
            .ok_or(ErrorCode::MathError)?,
    )?;
    match ctx
        .accounts
        .vault_port_lp_token
        .amount
        .checked_sub(port_collateral_amount)
    {
        Some(collateral_to_redeem) => {
            ctx.accounts.vault.to_reconcile[1].redeem = collateral_to_redeem
        }
        None => {
            ctx.accounts.vault.to_reconcile[1].deposit = port_exchange_rate
                .collateral_to_liquidity(
                    port_collateral_amount
                        .checked_sub(ctx.accounts.vault_port_lp_token.amount)
                        .ok_or(ErrorCode::MathError)?,
                )?
        }
    }

    Ok(())
}
