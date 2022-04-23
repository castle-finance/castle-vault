use std::iter::FromIterator;
use std::ops::Index;

use anchor_lang::prelude::*;
use rebalance::assets::Provider;

pub mod adapters;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod math;
pub mod rebalance;
pub mod state;

use rebalance::assets::ProviderIter;
use strum::IntoEnumIterator;

use crate::state::{RebalanceMode, StrategyType};
use adapters::*;
use instructions::*;

#[cfg(not(feature = "devnet-castle-addr"))]
declare_id!("Cast1eoVj8hwfKKRPji4cqX7WFgcnYz3um7TTgnaJKFn");

#[cfg(feature = "devnet-castle-addr")]
declare_id!("4tSMVfVbnwZcDwZB1M1j27dx9hdjL72VR9GM8AykpAvK");

#[program]
pub mod castle_lending_aggregator {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        _bumps: InitBumpSeeds,
        strategy_type: StrategyType,
        rebalance_mode: RebalanceMode,
        fees: FeeArgs,
        deposit_cap: u64,
        allocation_cap_pct: u8,
    ) -> ProgramResult {
        instructions::init::handler(
            ctx,
            _bumps,
            strategy_type,
            rebalance_mode,
            fees,
            deposit_cap,
            allocation_cap_pct,
        )
    }

    pub fn update_deposit_cap(
        ctx: Context<UpdateDepositCap>,
        deposit_cap_new_value: u64,
    ) -> ProgramResult {
        instructions::update_deposit_cap::handler(ctx, deposit_cap_new_value)
    }

    pub fn update_fees(ctx: Context<UpdateFees>, new_fees: FeeArgs) -> ProgramResult {
        instructions::update_fees::handler(ctx, new_fees)
    }

    pub fn deposit(ctx: Context<Deposit>, reserve_token_amount: u64) -> ProgramResult {
        instructions::deposit::handler(ctx, reserve_token_amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, lp_token_amount: u64) -> ProgramResult {
        instructions::withdraw::handler(ctx, lp_token_amount)
    }

    pub fn rebalance(
        ctx: Context<Rebalance>,
        proposed_weights: StrategyWeightsArg,
    ) -> ProgramResult {
        instructions::rebalance::handler(ctx, proposed_weights)
    }

    pub fn refresh<'info>(
        ctx: Context<'_, '_, '_, 'info, Refresh<'info>>,
        use_port_oracle: bool,
    ) -> ProgramResult {
        instructions::refresh::handler(ctx, use_port_oracle)
    }

    pub fn reconcile_solend(ctx: Context<SolendAccounts>, withdraw_option: u64) -> ProgramResult {
        instructions::reconcile::handler(ctx, withdraw_option)
    }

    pub fn reconcile_port(ctx: Context<PortAccounts>, withdraw_option: u64) -> ProgramResult {
        instructions::reconcile::handler(ctx, withdraw_option)
    }

    pub fn reconcile_jet(ctx: Context<JetAccounts>, withdraw_option: u64) -> ProgramResult {
        instructions::reconcile::handler(ctx, withdraw_option)
    }
}

#[derive(Clone)]
pub struct BackendContainer<'a, T> {
    pub solend: &'a T,
    pub port: &'a T,
    pub jet: &'a T,
}

impl<'a, T> BackendContainer<'a, T> {
    pub fn apply<U, F: Fn(Provider, &T) -> &U>(&mut self, f: F) -> BackendContainer<'a, U> {
        BackendContainer {
            solend: f(Provider::Solend, self.solend),
            port: f(Provider::Port, self.port),
            jet: f(Provider::Jet, self.jet),
        }
    }
}

// impl<'a, T> From<Iterator<Item = (Provider, &'a T)>> for BackendContainer<'a, T> {
//     fn from(_: Iterator<Item = (Provider, &'a T)>) -> Self {
//         todo!()
//     }
// }

impl<'a, T> FromIterator<(Provider, &'a T)> for BackendContainer<'a, T> {
    fn from_iter<U: IntoIterator<Item = (Provider, &'a T)>>(iter: U) -> Self {
        let mut solend = None;
        let mut port = None;
        let mut jet = None;
        for (provider, backend) in iter {
            match provider {
                Provider::Solend => solend = Some(backend),
                Provider::Port => port = Some(backend),
                Provider::Jet => jet = Some(backend),
            }
        }
        Self {
            solend: solend.expect("missing item in FromIterator for BackendContainer"),
            port: port.expect("missing item in FromIterator for BackendContainer"),
            jet: jet.expect("missing item in FromIterator for BackendContainer"),
        }
    }
}

// fn from(iter: Iterator<Item = (Provider, &'a T)>) -> Self {
//     let mut solend = None;
//     let mut port = None;
//     let mut jet = None;
//     for (provider, backend) in iter {
//         match provider {
//             Provider::Solend => solend = Some(backend),
//             Provider::Port => port = Some(backend),
//             Provider::Jet => jet = Some(backend),
//         }
//     }
//     Self {
//         solend: solend.unwrap(),
//         port: port.unwrap(),
//         jet: jet.unwrap(),
//     }
// }
// }

impl<T> Index<Provider> for BackendContainer<'_, T> {
    type Output = T;

    fn index(&self, provider: Provider) -> &Self::Output {
        match provider {
            Provider::Solend => self.solend,
            Provider::Port => self.port,
            Provider::Jet => self.jet,
        }
    }
}

pub struct BackendContainerIterator<'inner, T> {
    inner: &'inner BackendContainer<'inner, T>,
    inner_iter: ProviderIter,
}

impl<'inner, T> Iterator for BackendContainerIterator<'inner, T> {
    type Item = (Provider, &'inner T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter
            .next()
            .map(|provider| (provider, &self.inner[provider]))
    }
}

impl<'a, T> IntoIterator for &'a BackendContainer<'_, T> {
    type Item = (Provider, &'a T);
    type IntoIter = BackendContainerIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        BackendContainerIterator {
            inner: self,
            inner_iter: Provider::iter(),
        }
    }
}

impl<T> anchor_lang::AccountDeserialize for BackendContainer<'_, T> {
    fn try_deserialize(_: &mut &[u8]) -> Result<Self, ProgramError> {
        todo!()
    }

    fn try_deserialize_unchecked(_: &mut &[u8]) -> Result<Self, ProgramError> {
        todo!()
    }
}

impl<T> anchor_lang::AccountSerialize for BackendContainer<'_, T> {
    fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> Result<(), ProgramError> {
        todo!()
    }
}

impl<T> anchor_lang::Owner for BackendContainer<'_, T> {
    fn owner() -> Pubkey {
        todo!()
    }
}
