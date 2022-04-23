use std::ops::Deref;
use std::ops::Index;

use anchor_lang::prelude::*;
use port_anchor_adaptor::PortReserve;
use solana_maths::{Rate, TryAdd, TryMul};
use strum::IntoEnumIterator;

use crate::adapters::SolendReserve;
use crate::errors::ErrorCode;
use crate::rebalance::assets::*;
use crate::rebalance::strategies::*;
use crate::BackendContainer;
use crate::{impl_provider_index, state::*};

#[event]
pub struct RebalanceEvent {
    vault: Pubkey,
}

/// Used by the SDK to figure out the order in which reconcile TXs should be sent
#[event]
pub struct RebalanceDataEvent {
    solend: u64,
    port: u64,
    jet: u64,
}

// TODO connect this to same indexing?
impl From<&Allocations> for RebalanceDataEvent {
    fn from(allocations: &Allocations) -> Self {
        RebalanceDataEvent {
            solend: allocations[Provider::Solend].value,
            port: allocations[Provider::Port].value,
            jet: allocations[Provider::Jet].value,
        }
    }
}

#[derive(Clone)]
pub enum Reserves {
    Solend(SolendReserve),
    Port(PortReserve),
    Jet(jet::state::Reserve),
}

impl ReserveAccessor for Reserves {
    fn utilization_rate(&self) -> Result<Rate, ProgramError> {
        match self {
            Reserves::Solend(reserve) => reserve.utilization_rate(),
            Reserves::Port(reserve) => reserve.utilization_rate(),
            Reserves::Jet(reserve) => reserve.utilization_rate(),
        }
    }

    fn borrow_rate(&self) -> Result<Rate, ProgramError> {
        match self {
            Reserves::Solend(reserve) => reserve.borrow_rate(),
            Reserves::Port(reserve) => reserve.borrow_rate(),
            Reserves::Jet(reserve) => reserve.borrow_rate(),
        }
    }

    fn reserve_with_deposit(
        &self,
        allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>, ProgramError> {
        match self {
            Reserves::Solend(reserve) => reserve.reserve_with_deposit(allocation),
            Reserves::Port(reserve) => reserve.reserve_with_deposit(allocation),
            Reserves::Jet(reserve) => reserve.reserve_with_deposit(allocation),
        }
    }
}

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

    pub container: Account<'info, BackendContainer<'info, Reserves>>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub struct StrategyWeightsArg {
    solend: u16,
    port: u16,
    jet: u16,
}
impl_provider_index!(StrategyWeightsArg, u16);

// impl<'a, T> From<&'a T> for StrategyWeightsArg
// where
//     T: Index<Provider>,
// {
//     fn from(_: &'a T) -> Self {
//         todo!()
//     }
// }

impl From<StrategyWeightsArg> for StrategyWeights {
    fn from(value: StrategyWeightsArg) -> Self {
        let mut strategy_weights = Self::default();
        // let _val = StrategyWeights::from(&value);

        for p in Provider::iter() {
            strategy_weights[p] = Rate::from_bips(value[p] as u64);
        }
        strategy_weights
    }
}

/// Calculate and store optimal allocations to downstream lending markets
pub fn handler(ctx: Context<Rebalance>, proposed_weights_arg: StrategyWeightsArg) -> ProgramResult {
    #[cfg(feature = "debug")]
    msg!("Rebalancing");

    let vault_value = ctx.accounts.vault.total_value;
    let clock = Clock::get()?;

    ////////////////////////////////////////////////////////////////////////////////

    // let assets: Vec<LendingMarketAsset> = ctx
    let _val = ctx
        .accounts
        .container
        .apply(|_provider, container| {
            // Optionally apply some function to each backend item
            container
        })
        .into_iter()
        .map(|(provider, reserve)| {
            LendingMarketAsset(Box::new(reserve.clone()))
            // match ctx.accounts.vault.strategy_type {
            //     StrategyType::MaxYield => MaxYieldStrategy.calculate_weights(&assets),
            //     StrategyType::EqualAllocation => EqualAllocationStrategy.calculate_weights(&assets),
            // }
        });
    // .collect();

    ////////////////////////////////////////////////////////////////////////////////
    let assets = Assets {
        solend: LendingMarketAsset(Box::new(
            ctx.accounts.solend_reserve.as_ref().deref().deref().clone(),
        )),
        port: LendingMarketAsset(Box::new(
            ctx.accounts.port_reserve.as_ref().deref().deref().clone(),
        )),
        jet: LendingMarketAsset(Box::new(*ctx.accounts.jet_reserve.load()?)),
    };

    // TODO reduce the duplication between the Enum and Struct
    let strategy_weights = match ctx.accounts.vault.strategy_type {
        StrategyType::MaxYield => {
            MaxYieldStrategy.calculate_weights(&assets, ctx.accounts.vault.allocation_cap_pct)
        }
        StrategyType::EqualAllocation => EqualAllocationStrategy
            .calculate_weights(&assets, ctx.accounts.vault.allocation_cap_pct),
    }?;

    // Convert weights to allocations
    let strategy_allocations =
        Allocations::try_from_weights(strategy_weights, vault_value, clock.slot)?;

    let final_allocations = match ctx.accounts.vault.rebalance_mode {
        RebalanceMode::ProofChecker => {
            let proposed_weights: StrategyWeights = proposed_weights_arg.into();
            let proposed_allocations =
                Allocations::try_from_weights(proposed_weights, vault_value, clock.slot)?;

            #[cfg(feature = "debug")]
            msg!(
                "Running as proof checker with proposed weights: {:?}",
                proposed_weights
            );

            match ctx.accounts.vault.strategy_type {
                StrategyType::MaxYield => MaxYieldStrategy
                    .verify_weights(&proposed_weights, ctx.accounts.vault.allocation_cap_pct),
                StrategyType::EqualAllocation => EqualAllocationStrategy
                    .verify_weights(&proposed_weights, ctx.accounts.vault.allocation_cap_pct),
            }?;

            let proposed_apr = get_apr(&proposed_weights, &proposed_allocations, &assets)?;
            let proof_apr = get_apr(&strategy_weights, &strategy_allocations, &assets)?;

            #[cfg(feature = "debug")]
            msg!(
                "Proposed APR: {:?}\nProof APR: {:?}",
                proposed_apr,
                proof_apr
            );

            if proposed_apr < proof_apr {
                return Err(ErrorCode::RebalanceProofCheckFailed.into());
            }
            proposed_allocations
        }
        RebalanceMode::Calculator => {
            #[cfg(feature = "debug")]
            msg!("Running as calculator");
            strategy_allocations
        }
    };

    #[cfg(feature = "debug")]
    msg!("Final allocations: {:?}", final_allocations);

    emit!(RebalanceEvent {
        vault: ctx.accounts.vault.key()
    });
    emit!(RebalanceDataEvent::from(&final_allocations));

    ctx.accounts.vault.allocations = final_allocations;

    Ok(())
}

fn get_apr(
    weights: &StrategyWeights,
    allocations: &Allocations,
    assets: &Assets,
) -> Result<Rate, ProgramError> {
    Provider::iter()
        .map(|p| weights[p].try_mul(assets[p].calculate_return(allocations[p].value)?))
        .collect::<Result<Vec<_>, ProgramError>>()?
        .iter()
        .try_fold(Rate::zero(), |acc, r| acc.try_add(*r))
}
