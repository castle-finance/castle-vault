use anchor_lang::prelude::*;

pub mod adapters;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod math;
pub mod rebalance;
pub mod state;

use crate::{init::FeeArgs, state::StrategyType};
use adapters::*;
use instructions::*;

#[cfg(not(feature = "devnet-castle-addr"))]
declare_id!("Cast1eoVj8hwfKKRPji4cqX7WFgcnYz3um7TTgnaJKFn");

#[cfg(feature = "devnet-castle-addr")]
declare_id!("6hSKFKsZvksTb4M7828LqWsquWnyatoRwgZbcpeyfWRb");

#[program]
pub mod castle_lending_aggregator {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        _bumps: InitBumpSeeds,
        strategy_type: StrategyType,
        fees: FeeArgs,
        pool_size_limit: u64
    ) -> ProgramResult {
        instructions::init::handler(ctx, _bumps, strategy_type, fees, pool_size_limit)
    }

    pub fn update_cap(ctx: Context<UpdateCap>, deposit_cap_new_value: u64) -> ProgramResult {
        instructions::update_cap::handler(ctx, deposit_cap_new_value)
    }

    pub fn deposit(ctx: Context<Deposit>, reserve_token_amount: u64) -> ProgramResult {
        instructions::deposit::handler(ctx, reserve_token_amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, lp_token_amount: u64) -> ProgramResult {
        instructions::withdraw::handler(ctx, lp_token_amount)
    }

    pub fn rebalance(ctx: Context<Rebalance>) -> ProgramResult {
        instructions::rebalance::handler(ctx)
    }

    pub fn refresh(ctx: Context<Refresh>) -> ProgramResult {
        instructions::refresh::handler(ctx)
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
