use std::iter::FromIterator;
use std::ops::Deref;

use anchor_lang::prelude::*;
use port_anchor_adaptor::PortReserve;
use solana_maths::Decimal;
use solana_maths::{Rate, TryAdd, TryMul};
use strum::IntoEnumIterator;

use crate::adapters::SolendReserve;
use crate::errors::ErrorCode;
use crate::events::RebalanceEvent;
use crate::rebalance::assets::*;
use crate::rebalance::strategies::*;
use crate::BackendContainer;
use crate::{impl_provider_index, state::*};

/// If we can't find a way to make these all the same underlying type
/// one solution is to use an enum to wrap them all
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

    pub reserves_container: Account<'info, BackendContainer<Reserves>>,

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
    let vault_value = ctx.accounts.vault.total_value;
    let clock = Clock::get()?;

    ////////////////////////////////////////////////////////////////////////////////

    // Here's an example of how the BackendContainer can be used
    let _unused_example_container: BackendContainer<&'static str> = ctx
        .accounts
        .reserves_container
        .apply(|_provider, reserve| {
            // apply some function to each backend item
            (reserve.clone(), String::from("this is a happy provider!"))
        })
        .into_iter()
        .map(|(provider, (_reserve, _happy_string))| {
            // if we want to collect into a BackendContainer<T>, we need to return
            // a tuple with the first item being the provider
            (provider, "this is a happy provider!")
        })
        .collect();

    // Here we'll build a BackendContainer<Rate>
    let rates = match ctx.accounts.vault.strategy_type {
        StrategyType::MaxYield => {
            MaxYieldStrategy.calculate_weights_chris(&ctx.accounts.reserves_container)
        }
        StrategyType::EqualAllocation => {
            EqualAllocationStrategy.calculate_weights_chris(&ctx.accounts.reserves_container)
        }
    }?;

    // And here we'll use that to build a BackendContainer<Allocation> with try_apply() because we're dealing with Result<>'s
    let strategy_allocations = rates.try_apply(|_provider, rate| {
        rate.try_mul(vault_value).and_then(|product| {
            let value = Decimal::from(product).try_floor_u64()?;
            Ok(Allocation {
                value,
                last_update: LastUpdate::new(clock.slot),
            })
        })
    })?;

    // We can build a BackendContainer<u16> from the proposed_weights_arg
    let proposed_weights =
        BackendContainer::from_iter(Provider::iter().map(|p| (p, proposed_weights_arg[p])));

    let final_allocations = match ctx.accounts.vault.rebalance_mode {
        RebalanceMode::ProofChecker => {}
        RebalanceMode::Calculator => {}
    };

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

    emit!(RebalanceEvent::from(&final_allocations));

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
