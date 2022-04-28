use anchor_lang::prelude::*;

// use crate::borsh::{BorshDeserialize, BorshSerialize};

pub mod adapters;
pub mod backend_container;
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

const MAX_NUM_PROVIDERS: usize = 4;

#[program]
pub mod castle_vault {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize<MAX_NUM_PROVIDERS>>,
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
        ctx: Context<UpdateDepositCap<MAX_NUM_PROVIDERS>>,
        deposit_cap_new_value: u64,
    ) -> ProgramResult {
        instructions::update_deposit_cap::handler(ctx, deposit_cap_new_value)
    }

    pub fn update_fees(
        ctx: Context<UpdateFees<MAX_NUM_PROVIDERS>>,
        new_fees: FeeArgs,
    ) -> ProgramResult {
        instructions::update_fees::handler(ctx, new_fees)
    }

    pub fn deposit(
        ctx: Context<Deposit<MAX_NUM_PROVIDERS>>,
        reserve_token_amount: u64,
    ) -> ProgramResult {
        instructions::deposit::handler(ctx, reserve_token_amount)
    }

    pub fn withdraw(
        ctx: Context<Withdraw<MAX_NUM_PROVIDERS>>,
        lp_token_amount: u64,
    ) -> ProgramResult {
        instructions::withdraw::handler(ctx, lp_token_amount)
    }

    pub fn rebalance(
        ctx: Context<Rebalance<MAX_NUM_PROVIDERS>>,
        proposed_weights: StrategyWeightsArg,
    ) -> ProgramResult {
        instructions::rebalance::handler(ctx, proposed_weights)
    }

    pub fn rebalance_chris(
        ctx: Context<Rebalance<'_, MAX_NUM_PROVIDERS>>,
        proposed_weights: backend_container::BackendContainer<u16, MAX_NUM_PROVIDERS>,
    ) -> ProgramResult {
        instructions::rebalance::handler_chris(ctx, proposed_weights)
    }

    pub fn refresh<'info, const N: usize>(
        ctx: Context<'_, '_, '_, 'info, Refresh<'info, MAX_NUM_PROVIDERS>>,
        use_port_oracle: bool,
    ) -> ProgramResult {
        instructions::refresh::handler(ctx, use_port_oracle)
    }

    pub fn reconcile_solend(
        ctx: Context<SolendAccounts<MAX_NUM_PROVIDERS>>,
        withdraw_option: u64,
    ) -> ProgramResult {
        instructions::reconcile::handler(ctx, withdraw_option)
    }

    pub fn reconcile_port(
        ctx: Context<PortAccounts<MAX_NUM_PROVIDERS>>,
        withdraw_option: u64,
    ) -> ProgramResult {
        instructions::reconcile::handler(ctx, withdraw_option)
    }

    pub fn reconcile_jet(
        ctx: Context<JetAccounts<MAX_NUM_PROVIDERS>>,
        withdraw_option: u64,
    ) -> ProgramResult {
        instructions::reconcile::handler(ctx, withdraw_option)
    }
}
