use std::convert::TryFrom;
use std::ops::Deref;

use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use port_anchor_adaptor::PortReserve;
use solana_maths::{Decimal, TryMul};
use solend::SolendReserve;

use crate::cpi::solend;
use crate::errors::ErrorCode;
use crate::rebalance::assets::{Asset, LendingMarket};
use crate::rebalance::strategies::{EqualAllocationStrategy, Strategy};
use crate::state::*;

#[derive(Accounts)]
pub struct Rebalance<'info> {
    #[account(
        mut,
        constraint = !vault.last_update.stale @ ErrorCode::VaultIsNotRefreshed,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
        has_one = vault_port_lp_token,
        has_one = vault_jet_lp_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,

    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,

    pub solend_reserve_state: Box<Account<'info, SolendReserve>>,

    pub port_reserve_state: Box<Account<'info, PortReserve>>,

    pub jet_reserve_state: AccountLoader<'info, jet::state::Reserve>,
}

pub fn handler(ctx: Context<Rebalance>, to_withdraw_option: u64) -> ProgramResult {
    if to_withdraw_option != 0 {
        // TODO use introspection make sure that there is a withdraw instruction after
    }

    // Convert reserve states to assets
    let mut assets: Vec<Box<dyn Asset>> = Vec::new();
    assets.push(Box::new(LendingMarket::try_from(
        ctx.accounts.solend_reserve_state.clone().into_inner(),
    )?));
    assets.push(Box::new(LendingMarket::try_from(
        ctx.accounts.port_reserve_state.clone().into_inner(),
    )?));
    assets.push(Box::new(LendingMarket::try_from(
        *ctx.accounts.jet_reserve_state.load()?.deref(),
    )?));

    // Create strategy
    //let strategy = Strategy::from_config(ctx.accounts.vault.strategy_config);
    let strategy = EqualAllocationStrategy;

    // Run strategy to get allocations
    let strategy_allocations = strategy
        .calculate_allocations(assets)
        .ok_or(ErrorCode::StrategyError)?;
    msg!("{:?}", strategy_allocations);

    // Store allocations
    let vault_value = ctx
        .accounts
        .vault
        .total_value
        .checked_sub(to_withdraw_option)
        .ok_or(ErrorCode::MathError)?;
    let clock = Clock::get()?;
    let vault_allocations = &mut ctx.accounts.vault.allocations;
    // TODO is there a way to make this less repetitive?
    vault_allocations.solend.update(
        Decimal::from(strategy_allocations[0].try_mul(vault_value)?).try_floor_u64()?,
        clock.slot,
    );
    vault_allocations.port.update(
        Decimal::from(strategy_allocations[1].try_mul(vault_value)?).try_floor_u64()?,
        clock.slot,
    );
    vault_allocations.jet.update(
        Decimal::from(strategy_allocations[2].try_mul(vault_value)?).try_floor_u64()?,
        clock.slot,
    );

    Ok(())
}
