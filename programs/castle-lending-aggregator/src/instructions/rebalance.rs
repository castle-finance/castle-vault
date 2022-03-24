use std::convert::TryFrom;
use std::ops::Deref;

use anchor_lang::prelude::*;
use port_anchor_adaptor::PortReserve;
use solana_maths::{Decimal, Rate, TryMul};
use solend_cpi::SolendReserve;

use crate::cpi::solend_cpi;
use crate::errors::ErrorCode;
use crate::events::RebalanceEvent;
use crate::rebalance::assets::{LendingMarket, Provider};
use crate::rebalance::strategies::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Rebalance<'info> {
    /// Vault state account
    /// Checks that the refresh has been called in the same slot
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        constraint = !vault.last_update.is_stale(clock.slot)? @ ErrorCode::VaultIsNotRefreshed,
        has_one = solend_reserve,
        has_one = port_reserve,
        has_one = jet_reserve,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub solend_reserve: Box<Account<'info, SolendReserve>>,

    pub port_reserve: Box<Account<'info, PortReserve>>,

    pub jet_reserve: AccountLoader<'info, jet::state::Reserve>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(Debug, Clone)]
pub struct RateUpdate {
    pub provider: Provider,
    pub rate: Rate,
}

impl RateUpdate {
    pub fn try_apply(
        &self,
        clock: &Clock,
        vault_value: u64,
        vault_allocations: &mut Allocations,
    ) -> Result<(), ProgramError> {
        let rate = self
            .rate
            .try_mul(vault_value)
            .and_then(|product| Decimal::from(product).try_floor_u64())?;

        match self.provider {
            Provider::Solend => vault_allocations.solend.update(rate, clock.slot),
            Provider::Port => vault_allocations.port.update(rate, clock.slot),
            Provider::Jet => vault_allocations.jet.update(rate, clock.slot),
        }
        Ok(())
    }
}

/// Calculate and store optimal allocations to downstream lending markets
pub fn handler(ctx: Context<Rebalance>) -> ProgramResult {
    msg!("Rebalancing");
    let assets = [
        LendingMarket::try_from(ctx.accounts.solend_reserve.as_ref().deref())?,
        LendingMarket::try_from(ctx.accounts.port_reserve.as_ref().deref())?,
        LendingMarket::try_from(&*ctx.accounts.jet_reserve.load()?)?,
    ];

    let strategy_allocations = match ctx.accounts.vault.strategy_type {
        StrategyType::MaxYield => MaxYieldStrategy.calculate_allocations(&assets),
        StrategyType::EqualAllocation => EqualAllocationStrategy.calculate_allocations(&assets),
    }
    .ok_or(ErrorCode::StrategyError)?;

    msg!("Strategy allocations: {:?}", strategy_allocations);

    let vault_value = ctx.accounts.vault.total_value;
    let vault_allocations = &mut ctx.accounts.vault.allocations;
    let clock = Clock::get()?;

    strategy_allocations
        .iter()
        .try_for_each(|s| s.try_apply(&clock, vault_value, vault_allocations))?;

    emit!(RebalanceEvent {
        solend: vault_allocations.solend.value,
        port: vault_allocations.port.value,
        jet: vault_allocations.jet.value,
    });

    Ok(())
}
