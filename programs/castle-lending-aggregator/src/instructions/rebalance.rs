use std::convert::TryFrom;
use std::ops::Deref;

use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use port_anchor_adaptor::PortReserve;
use solana_maths::{Decimal, Rate, TryMul};
use solend::SolendReserve;

use crate::cpi::solend;
use crate::errors::ErrorCode;
use crate::events::RebalanceEvent;
use crate::rebalance::assets::{Asset, LendingMarket};
use crate::rebalance::strategies::*;
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

pub fn handler(ctx: Context<Rebalance>) -> ProgramResult {
    // Convert reserve states to assets
    let assets: Vec<Box<dyn Asset>> = vec![
        Box::new(LendingMarket::try_from(
            ctx.accounts.solend_reserve_state.clone().into_inner(),
        )?),
        Box::new(LendingMarket::try_from(
            ctx.accounts.port_reserve_state.clone().into_inner(),
        )?),
        Box::new(LendingMarket::try_from(
            *ctx.accounts.jet_reserve_state.load()?.deref(),
        )?),
    ];

    // Run strategy to get allocations
    let strategy_allocations = match ctx.accounts.vault.strategy_type {
        StrategyType::MaxYield => MaxYieldStrategy.calculate_allocations(assets),
        StrategyType::EqualAllocation => EqualAllocationStrategy.calculate_allocations(assets),
    }
    .ok_or(ErrorCode::StrategyError)?;

    msg!("Strategy allocations: {:?}", strategy_allocations);

    let new_vault_allocations = strategy_allocations
        .iter()
        .map(|a| calc_vault_allocation(*a, ctx.accounts.vault.total_value))
        .collect::<Result<Vec<_>, _>>()?;

    // TODO use https://doc.rust-lang.org/std/ops/trait.Index.html to make this less bad
    let clock = Clock::get()?;
    let vault_allocations = &mut ctx.accounts.vault.allocations;
    vault_allocations
        .solend
        .update(new_vault_allocations[0], clock.slot);
    vault_allocations
        .port
        .update(new_vault_allocations[1], clock.slot);
    vault_allocations
        .jet
        .update(new_vault_allocations[2], clock.slot);

    emit!(RebalanceEvent {
        solend: new_vault_allocations[0],
        port: new_vault_allocations[1],
        jet: new_vault_allocations[2],
    });
    Ok(())
}

fn calc_vault_allocation(strategy_allocation: Rate, vault_value: u64) -> Result<u64, ProgramError> {
    Decimal::from(strategy_allocation.try_mul(vault_value)?).try_floor_u64()
}
