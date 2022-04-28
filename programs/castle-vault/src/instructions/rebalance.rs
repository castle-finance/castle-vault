use std::convert::TryFrom;
use std::ops::Deref;

use anchor_lang::prelude::*;
use boolinator::Boolinator;
use port_anchor_adaptor::PortReserve;
use solana_maths::{Rate, TryAdd, TryMul};
use strum::IntoEnumIterator;

use crate::adapters::SolendReserve;
use crate::backend_container::BackendContainer;
use crate::errors::ErrorCode;
use crate::events::RebalanceEvent;
use crate::rebalance::assets::*;
use crate::rebalance::strategies::*;
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

/// If we can't find a way to make these all the same underlying type
/// one solution is to use an enum to wrap them all
#[derive(Clone)]
pub enum Reserves {
    Solend(SolendReserve),
    Port(PortReserve),
    Jet(Box<jet::state::Reserve>),
}

impl<'a> ReserveAccessor for Reserves {
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

    // TODO: I'm not sure if there is any way to make this work, but I don't think so
    // pub reserves_container: Account<'info, BackendContainer<Reserves>>,
    pub clock: Sysvar<'info, Clock>,
}

impl TryFrom<&Rebalance<'_>> for BackendContainer<Reserves> {
    type Error = ProgramError;
    fn try_from(r: &Rebalance<'_>) -> Result<BackendContainer<Reserves>, Self::Error> {
        // NOTE: I tried pretty hard to get rid of these clones and only use the references.
        // The problem is that these references originate from a deref() (or as_ref())
        // and end up sharing lifetimes with the Context<Rebalance>.accounts lifetime,
        // which means that the lifetimes are shared, preventing any other borrows
        // (in particular the mutable borrow required at the end to save state)
        let solend = Some(Reserves::Solend(r.solend_reserve.deref().deref().clone()));
        let port = Some(Reserves::Port(r.port_reserve.deref().deref().clone()));
        let jet = Some(Reserves::Jet(Box::new(*r.jet_reserve.load()?)));
        Ok(BackendContainer {
            inner: [solend, port, jet],
        })
    }
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
        let mut strategy_weights = Self::default();

        for p in Provider::iter() {
            strategy_weights[p] = Rate::from_bips(value[p] as u64);
        }
        strategy_weights
    }
}

/// Calculate and store optimal allocations to downstream lending markets
// This is identical to `handler_chris()` below (at least that's the intention), just a different style
pub fn handler_chris_concise(
    ctx: Context<Rebalance>,
    proposed_weights_arg: BackendContainer<u16>,
) -> ProgramResult {
    let vault_value = ctx.accounts.vault.total_value;
    let slot = Clock::get()?.slot;

    let assets = Box::new(BackendContainer::try_from(&*ctx.accounts)?);
    let strategy_weights = assets.calculate_weights(ctx.accounts.vault.strategy_type)?;

    BackendContainer::<Allocation>::try_from_weights(&strategy_weights, vault_value, slot)
        .and_then(
            |strategy_allocations| match ctx.accounts.vault.rebalance_mode {
                RebalanceMode::ProofChecker => {
                    let proposed_weights = BackendContainer::<Rate>::from(proposed_weights_arg);
                    let proposed_allocations = BackendContainer::<Allocation>::try_from_weights(
                        &strategy_weights,
                        vault_value,
                        slot,
                    )?;

                    #[cfg(feature = "debug")]
                    msg!(
                        "Running as proof checker with proposed weights: {:?}",
                        proposed_weights
                    );

                    proposed_weights.verify_weights()?;

                    let proposed_apr = assets.get_apr(&proposed_weights, &proposed_allocations)?;
                    let proof_apr = assets.get_apr(&strategy_weights, &strategy_allocations)?;

                    #[cfg(feature = "debug")]
                    msg!(
                        "Proposed APR: {:?}\nProof APR: {:?}",
                        proposed_apr,
                        proof_apr
                    );

                    (proposed_apr >= proof_apr).as_result(
                        proposed_allocations,
                        ErrorCode::RebalanceProofCheckFailed.into(),
                    )
                }
                RebalanceMode::Calculator => {
                    #[cfg(feature = "debug")]
                    msg!("Running as calculator");
                    Ok(strategy_allocations)
                }
            },
        )
        .map(|final_allocations| {
            #[cfg(feature = "debug")]
            msg!("Final allocations: {:?}", final_allocations);

            // emit!(RebalanceEventChris {
            //     allocations: final_allocations.clone()
            // });

            ctx.accounts.vault.allocations_chris = final_allocations;
        })
}
/// Calculate and store optimal allocations to downstream lending markets
pub fn handler_chris(
    ctx: Context<Rebalance>,
    proposed_weights_arg: BackendContainer<u16>,
) -> ProgramResult {
    ////////////////////////////////////////////////////////////////////////////////
    // Here's an example of how a BackendContainer can be used
    let _unused_example_container: BackendContainer<&'static str> =
        BackendContainer::<Reserves>::try_from(&*ctx.accounts)?
            .apply(|_provider, reserve| {
                // apply some function to each backend item
                fn foo<T: Clone>(t: &T) -> T {
                    t.clone()
                }
                (foo(reserve), String::from("this is a happy provider!"))
            })
            .into_iter()
            .map(|(provider, (_reserve, _happy_string))| {
                // if we want to collect into a BackendContainer<T>, we need to return
                // a tuple with the first item being the provider
                (provider, "this is a happy provider!")
            })
            // You can `.collect()` a BackendContainerIterator<T> into a BackendContainer<T>
            .collect();
    ////////////////////////////////////////////////////////////////////////////////

    let vault_value = ctx.accounts.vault.total_value;
    let clock = Clock::get()?;

    let final_allocations = {
        ////////////////////////////////////////////////////////////////////////////////
        // Here is an alternative that uses LendingMarketAsset
        let val = [
            (
                Provider::Solend,
                LendingMarketAsset(Box::new(
                    ctx.accounts.solend_reserve.as_ref().deref().deref().clone(),
                )),
            ),
            (
                Provider::Port,
                LendingMarketAsset(Box::new(
                    ctx.accounts.port_reserve.as_ref().deref().deref().clone(),
                )),
            ),
            (
                Provider::Jet,
                LendingMarketAsset(Box::new(*ctx.accounts.jet_reserve.load()?)),
            ),
        ];

        // IntoIterator::into_iter() is used here because it can consume an array by value
        let _assets: BackendContainer<LendingMarketAsset> = IntoIterator::into_iter(val).collect();
        ////////////////////////////////////////////////////////////////////////////////

        // For now we'll use the `Reserves` type
        let assets = BackendContainer::<Reserves>::try_from(&*ctx.accounts)?;

        // Here we'll build a BackendContainer<Rate> from the BackendContainer<Reserves>
        let strategy_weights = assets
            // This `calculate_weights()` is provided by the impl BackendContainer<Reserves>
            .calculate_weights(ctx.accounts.vault.strategy_type)?;

        // And here we'll use that to build a BackendContainer<Allocation> with try_apply()
        // (instead of apply() because we're dealing with Result<>'s)
        let strategy_allocations = BackendContainer::<Allocation>::try_from_weights(
            &strategy_weights,
            vault_value,
            clock.slot,
        )?;
        match ctx.accounts.vault.rebalance_mode {
            RebalanceMode::ProofChecker => {
                let proposed_weights: BackendContainer<Rate> = proposed_weights_arg.into();

                let proposed_allocations = BackendContainer::<Allocation>::try_from_weights(
                    &strategy_weights,
                    vault_value,
                    clock.slot,
                )?;

                #[cfg(feature = "debug")]
                msg!(
                    "Running as proof checker with proposed weights: {:?}",
                    proposed_weights
                );

                proposed_weights.verify_weights()?;

                let proposed_apr = assets.get_apr(&proposed_weights, &proposed_allocations)?;
                let proof_apr = assets.get_apr(&strategy_weights, &strategy_allocations)?;

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
        }
    };
    #[cfg(feature = "debug")]
    msg!("Final allocations: {:?}", final_allocations);

    // emit!(RebalanceEventChris {
    //     allocations: final_allocations.clone()
    // });

    ctx.accounts.vault.allocations_chris = final_allocations;

    Ok(())
}
/// Calculate and store optimal allocations to downstream lending markets
pub fn handler(ctx: Context<Rebalance>, proposed_weights_arg: StrategyWeightsArg) -> ProgramResult {
    let vault_value = ctx.accounts.vault.total_value;
    let clock = Clock::get()?;

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
