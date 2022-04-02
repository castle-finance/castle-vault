use std::convert::TryFrom;
use std::ops::Deref;

use anchor_lang::prelude::*;
use port_anchor_adaptor::PortReserve;
use solana_maths::Rate;
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

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub struct StrategyWeightsArg {
    solend: u16,
    port: u16,
    jet: u16,
}
impl_provider_index!(StrategyWeightsArg, u16);

impl From<StrategyWeightsArg> for StrategyWeights {
    fn from(value: StrategyWeightsArg) -> Self {
        let strategy_weights = &mut Self::default();
        for p in Provider::iter() {
            strategy_weights[p] = Rate::from_bips(value[p] as u64);
        }
        *strategy_weights
    }
}

/// Calculate and store optimal allocations to downstream lending markets
pub fn handler(ctx: Context<Rebalance>, proposed_weights_arg: StrategyWeightsArg) -> ProgramResult {
    msg!("Rebalancing");

    let vault_value = ctx.accounts.vault.total_value;
    let clock = Clock::get()?;

    let assets = [
        LendingMarket::try_from(ctx.accounts.solend_reserve.as_ref().deref())?,
        LendingMarket::try_from(ctx.accounts.port_reserve.as_ref().deref())?,
        LendingMarket::try_from(&*ctx.accounts.jet_reserve.load()?)?,
    ];

    // TODO reduce the duplication between the Enum and Struct
    let strategy_weights = match ctx.accounts.vault.strategy_type {
        StrategyType::MaxYield => MaxYieldStrategy.calculate_weights(&assets),
        StrategyType::EqualAllocation => EqualAllocationStrategy.calculate_weights(&assets),
    }?;

    // Convert weights to allocations
    let strategy_allocations =
        Allocations::try_from_weights(strategy_weights, vault_value, clock.slot)?;

    let final_allocations = if ctx.accounts.vault.proof_checker != 0 {
        let proposed_weights: StrategyWeights = proposed_weights_arg.into();
        let proposed_allocations =
            Allocations::try_from_weights(proposed_weights, vault_value, clock.slot)?;

        msg!(
            "Running as proof checker with proposed weights: {:?}",
            proposed_weights
        );
        // Validate inputs
        match ctx.accounts.vault.strategy_type {
            StrategyType::MaxYield => MaxYieldStrategy.verify(&proposed_weights),
            StrategyType::EqualAllocation => EqualAllocationStrategy.verify(&proposed_weights),
        }?;

        // Get APY of proposed weights
        let proposed_apy_bps = todo!();

        // Get APY of proof check
        let proof_apy_bps = todo!();

        // Fail if proposed APY < provable APY
        if proposed_apy_bps < proof_apy_bps {
            return Err(ErrorCode::RebalanceProofCheckFailed.into());
        }
        proposed_allocations
    } else {
        msg!("Running as calculator");
        strategy_allocations
    };

    msg!("Final allocations: {:?}", final_allocations);
    emit!(RebalanceEvent::from(&final_allocations));

    ctx.accounts.vault.allocations = final_allocations;

    Ok(())
}
