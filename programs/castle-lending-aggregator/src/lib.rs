use anchor_lang::prelude::*;

pub mod adapters;
pub mod errors;
pub mod instructions;
pub mod math;
pub mod rebalance;
pub mod state;

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
