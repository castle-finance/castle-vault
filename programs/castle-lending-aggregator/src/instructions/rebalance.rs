use std::cmp::Ordering;
use std::ops::{Deref, Index};

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_pack::{IsInitialized, Pack, Sealed};
use port_anchor_adaptor::PortReserve;
use solana_maths::Decimal;
use solana_maths::{Rate, TryAdd, TryMul};
use strum::IntoEnumIterator;

use crate::adapters::SolendReserve;
use crate::backend_container::BackendContainer;
use crate::errors::ErrorCode;
use crate::events::RebalanceEvent;
use crate::rebalance::assets::*;
use crate::rebalance::strategies::*;
use crate::{impl_provider_index, state::*};

/// If we can't find a way to make these all the same underlying type
/// one solution is to use an enum to wrap them all
#[derive(Clone)]
pub enum Reserves {
    Solend(SolendReserve),
    Port(PortReserve),
    Jet(jet::state::Reserve),
}

impl AccountDeserialize for Reserves {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Self::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Self::unpack(buf)
    }
}

// TODO
impl Pack for Reserves {
    const LEN: usize = 42;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        match self {
            Reserves::Solend(v) => v.pack_into_slice(dst),
            Reserves::Port(v) => v.pack_into_slice(dst),
            Reserves::Jet(_v) => todo!(), //v.pack_into_slice(dst),
        }
    }

    fn unpack_from_slice(_src: &[u8]) -> Result<Self, ProgramError> {
        todo!()
    }
}

impl Sealed for Reserves {}
impl IsInitialized for Reserves {
    fn is_initialized(&self) -> bool {
        todo!()
    }
}
impl AccountSerialize for Reserves {
    fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> Result<(), ProgramError> {
        todo!()
    }
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
pub fn handler_chris(
    ctx: Context<Rebalance>,
    proposed_weights_arg: BackendContainer<u16>,
) -> ProgramResult {
    ////////////////////////////////////////////////////////////////////////////////
    // Here's an example of how a BackendContainer can be used
    let _unused_example_container: BackendContainer<&'static str> = ctx
        .accounts
        .reserves_container
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
    let assets = ctx.accounts.reserves_container.deref();

    // Here we'll build a BackendContainer<Rate> from the BackendContainer<Reserves>
    let strategy_weights = assets
        // This `calculate_weights()` is provided by the impl BackendContainer<Reserves>
        .calculate_weights(ctx.accounts.vault.strategy_type)?;

    // // And here we'll use that to build a BackendContainer<Allocation> with try_apply()
    // // (instead of apply() because we're dealing with Result<>'s)
    let strategy_allocations = BackendContainer::<Allocation>::try_from_weights(
        &strategy_weights,
        vault_value,
        clock.slot,
    )?;

    let final_allocations = match ctx.accounts.vault.rebalance_mode {
        RebalanceMode::ProofChecker => {
            // We can build a BackendContainer<u16> from the proposed_weights_arg
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

            let proposed_apr = get_apr_chris(&proposed_weights, &proposed_allocations, assets)?;
            let proof_apr = get_apr_chris(&strategy_weights, &strategy_allocations, assets)?;
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

    // TODO
    // emit!(RebalanceEvent::from(&final_allocations));

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

fn get_apr_chris(
    weights: &dyn Index<Provider, Output = Rate>,
    allocations: &dyn Index<Provider, Output = Allocation>,
    assets: &dyn Index<Provider, Output = Reserves>,
) -> Result<Rate, ProgramError> {
    Provider::iter()
        .map(|p| weights[p].try_mul(assets[p].calculate_return(allocations[p].value)?))
        .collect::<Result<Vec<_>, ProgramError>>()?
        .iter()
        .try_fold(Rate::zero(), |acc, r| acc.try_add(*r))
}

impl BackendContainer<Allocation> {
    pub fn try_from_weights(
        rates: &BackendContainer<Rate>,
        vault_value: u64,
        slot: u64,
    ) -> Result<Self, ProgramError> {
        rates.try_apply(|_provider, rate| {
            rate.try_mul(vault_value).and_then(|product| {
                Decimal::from(product)
                    .try_floor_u64()
                    .map(|value| Allocation {
                        value,
                        last_update: LastUpdate::new(slot),
                    })
            })
        })
    }
}

impl<T> BackendContainer<T>
where
    T: ReturnCalculator,
{
    fn compare(&self, lhs: &T, rhs: &T) -> Result<Ordering, ProgramError> {
        Ok(lhs.calculate_return(0)?.cmp(&rhs.calculate_return(0)?))
    }
}

impl BackendContainer<Reserves> {
    fn calculate_weights_max_yield(&self) -> Result<BackendContainer<Rate>, ProgramError> {
        self.into_iter()
            .max_by(|(_prov_x, alloc_x), (_prov_y, alloc_y)| {
                // TODO: can we remove the unwrap() in any way?
                self.compare(*alloc_x, *alloc_y).unwrap()
            })
            .map(|(max_yielding_provider, _a)| {
                self.apply(|provider, _v| {
                    if provider == max_yielding_provider {
                        Rate::one()
                    } else {
                        Rate::zero()
                    }
                })
            })
            // TODO make this error handling more granular and informative
            .ok_or(ErrorCode::StrategyError)
            .map_err(Into::into)
    }

    pub fn calculate_weights(
        &self,
        stype: StrategyType,
    ) -> Result<BackendContainer<Rate>, ProgramError> {
        match stype {
            StrategyType::MaxYield => self.calculate_weights_max_yield(),
            StrategyType::EqualAllocation => todo!(),
        }
    }
}

impl Strategy for BackendContainer<u16> {
    fn calculate_weights(&self, _assets: &Assets) -> Result<StrategyWeights, ProgramError> {
        todo!()
    }

    fn calculate_weights_chris<T>(&self) -> Result<BackendContainer<Rate>, ProgramError> {
        todo!()
    }

    fn verify_weights_chris(&self) -> ProgramResult {
        // let sum = Provider::iter()
        //     .map(|p| proposed_weights[p])
        //     .try_fold(Rate::zero(), |acc, x| acc.try_add(x))?;

        // self.into_iter()
        //     .try_fold(Rate::zero(), |acc, (_, x)| acc.try_add(x.value))?;
        // .map(|(_provider, allocation)| {
        //     if allocation.value > 0 {
        //         Ok(())
        //     } else {
        //         Err(ErrorCode::RebalanceProofCheckFailed.into())
        //     }
        // })
        // .collect::<Result<Vec<_>, ProgramError>>()?;

        let sum = Rate::default();
        if sum != Rate::one() {
            return Err(ErrorCode::InvalidProposedWeights.into());
        }
        Ok(())
    }
}

impl From<BackendContainer<u16>> for BackendContainer<Rate> {
    fn from(c: BackendContainer<u16>) -> Self {
        c.apply(|_provider, v| Rate::from_bips(*v as u64))
    }
}

/*
impl TryFrom<BackendContainer<Rate>> for BackendContainer<Allocation> {
    type Error = ProgramError;

    fn try_from(value: BackendContainer<Rate>) -> Result<Self, Self::Error> {
        value.try_apply(|_provider, rate| {
            rate.try_mul(vault_value).and_then(|product| {
                Decimal::from(product)
                    .try_floor_u64()
                    .map(|value| Allocation {
                        value,
                        last_update: LastUpdate::new(clock.slot),
                    })
            })
        })
    }
}
*/
