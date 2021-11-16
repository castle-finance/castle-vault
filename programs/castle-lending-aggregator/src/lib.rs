use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod castle_lending_aggregator {
    use super::*;

    pub fn initialize_pool(ctx: Context<InitializePool>) -> ProgramResult {
        instructions::init::handler(ctx)
    }

    pub fn deposit(ctx: Context<Deposit>, source_token_amount: u64) -> ProgramResult {
        instructions::deposit::handler(ctx, source_token_amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, pool_token_amount: u64) -> ProgramResult {
        instructions::withdraw::handler(ctx, pool_token_amount)
    }

    pub fn rebalance(ctx: Context<Rebalance>) -> ProgramResult {
        instructions::rebalance::handler(ctx)
    }
}
