use anchor_lang::prelude::*;

pub mod cpi;
pub mod errors;
pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;

declare_id!("6hSKFKsZvksTb4M7828LqWsquWnyatoRwgZbcpeyfWRb");

#[program]
pub mod castle_lending_aggregator {
    use super::*;

    // TODO add docs

    pub fn initialize(ctx: Context<Initialize>, _bumps: InitBumpSeeds) -> ProgramResult {
        instructions::init::handler(ctx, _bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, reserve_token_amount: u64) -> ProgramResult {
        instructions::deposit::handler(ctx, reserve_token_amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, lp_token_amount: u64) -> ProgramResult {
        instructions::withdraw::handler(ctx, lp_token_amount)
    }

    pub fn rebalance(ctx: Context<Rebalance>, to_withdraw_option: u64) -> ProgramResult {
        instructions::rebalance::handler(ctx, to_withdraw_option)
    }

    pub fn refresh(ctx: Context<Refresh>) -> ProgramResult {
        instructions::refresh::handler(ctx)
    }

    pub fn reconcile_solend(ctx: Context<ReconcileSolend>) -> ProgramResult {
        instructions::reconcile_solend::handler(ctx)
    }

    pub fn reconcile_port(ctx: Context<ReconcilePort>) -> ProgramResult {
        instructions::reconcile_port::handler(ctx)
    }
}
