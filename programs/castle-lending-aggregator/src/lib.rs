use anchor_lang::prelude::*;

pub mod cpi;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod math;
pub mod rebalance;
pub mod state;

use crate::state::StrategyType;
use instructions::*;

declare_id!("6hSKFKsZvksTb4M7828LqWsquWnyatoRwgZbcpeyfWRb");

#[program]
pub mod castle_lending_aggregator {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        _bumps: InitBumpSeeds,
        strategy_type: StrategyType,
        fee_carry_bps: u16,
        fee_mgmt_bps: u16,
    ) -> ProgramResult {
        instructions::init::handler(ctx, _bumps, strategy_type, fee_carry_bps, fee_mgmt_bps)
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

    pub fn reconcile_solend(ctx: Context<ReconcileSolend>, withdraw_option: u64) -> ProgramResult {
        let option = if withdraw_option == 0 {
            None
        } else {
            Some(withdraw_option)
        };
        instructions::reconcile_solend::handler(ctx, option)
    }

    pub fn reconcile_port(ctx: Context<ReconcilePort>, withdraw_option: u64) -> ProgramResult {
        let option = if withdraw_option == 0 {
            None
        } else {
            Some(withdraw_option)
        };
        instructions::reconcile_port::handler(ctx, option)
    }

    pub fn reconcile_jet(ctx: Context<ReconcileJet>, withdraw_option: u64) -> ProgramResult {
        let option = if withdraw_option == 0 {
            None
        } else {
            Some(withdraw_option)
        };
        instructions::reconcile_jet::handler(ctx, option)
    }
}
