use std::convert::TryFrom;
use std::ops::Deref;

use anchor_lang::prelude::*;
use port_anchor_adaptor::PortReserve;
use solana_maths::{Decimal, Rate, TryMul};
use strum::IntoEnumIterator;

use crate::adapters::SolendReserve;
use crate::errors::ErrorCode;
use crate::events::RebalanceEvent;
use crate::rebalance::assets::LendingMarket;
use crate::rebalance::strategies::*;
use crate::{impl_provider_index, state::*};

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
        let allocation = self
            .rate
            .try_mul(vault_value)
            .and_then(|product| Decimal::from(product).try_floor_u64())?;
        //msg!("Setting allocation: {}", allocation);

        vault_allocations[self.provider].update(allocation, clock.slot);
        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct ProposedWeightsBps {
    pub solend: u16,
    pub port: u16,
    pub jet: u16,
}

impl_provider_index!(ProposedWeightsBps, u16);

impl ProposedWeightsBps {
    fn verify(&self) -> ProgramResult {
        // Weights must add up to 10,000
        let sum = [self.solend, self.port, self.jet]
            .iter()
            .try_fold(0, |acc: u16, &x| acc.checked_add(x))
            .ok_or(ErrorCode::OverflowError)?;

        if sum != 10000 {
            return Err(ErrorCode::InvalidProposedWeights.into());
        }

        Ok(())
    }
}

/// Calculate and store optimal allocations to downstream lending markets
pub fn handler(ctx: Context<Rebalance>, proposed_weights: ProposedWeightsBps) -> ProgramResult {
    msg!("Rebalancing");

    let assets = [
        LendingMarket::try_from(ctx.accounts.solend_reserve.as_ref().deref())?,
        LendingMarket::try_from(ctx.accounts.port_reserve.as_ref().deref())?,
        LendingMarket::try_from(&*ctx.accounts.jet_reserve.load()?)?,
    ];

    let strategy_allocations = match ctx.accounts.vault.strategy_type {
        StrategyType::MaxYield => MaxYieldStrategy.calculate_allocations(&assets),
        StrategyType::EqualAllocation => EqualAllocationStrategy.calculate_allocations(&assets),
    }?;

    let final_allocations = if ctx.accounts.vault.proof_checker != 0 {
        msg!(
            "Running as proof checker with proposed weights: {:?}",
            proposed_weights
        );
        proposed_weights.verify()?;

        // Get APY of proposed weights

        // Get APY of proof check

        // Fail if proposed APY < provable APY

        // Convert proposed_weigts to Vec<RateUpdate> and return
        Provider::iter()
            .map(|p| RateUpdate {
                provider: p,
                rate: Rate::from_bips(proposed_weights[p] as u64),
            })
            .collect::<Vec<RateUpdate>>()
    } else {
        msg!("Running as calculator");
        strategy_allocations
    };

    msg!("Final allocations: {:?}", final_allocations);

    let vault_value = ctx.accounts.vault.total_value;
    let vault_allocations = &mut ctx.accounts.vault.allocations;
    let clock = Clock::get()?;

    final_allocations
        .iter()
        .try_for_each(|s| s.try_apply(&clock, vault_value, vault_allocations))?;

    emit!(RebalanceEvent::from(&*vault_allocations));

    Ok(())
}
