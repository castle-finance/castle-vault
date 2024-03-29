use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use boolinator::Boolinator;
use port_anchor_adaptor::{port_lending_id, port_staking_id, PortStakeAccount, PortStakingPool};

use crate::state::{Vault, VaultPortAdditionalState};

#[derive(Accounts)]
pub struct ClaimPortReward<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_authority,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// CHECK: safe
    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    #[account(
        seeds = [vault.key().as_ref(), b"port_additional_state".as_ref()], 
        bump = vault.vault_port_additional_state_bump,
        has_one = port_staking_pool,
    )]
    pub port_additional_states: Box<Account<'info, VaultPortAdditionalState>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"port_stake".as_ref()], 
        bump = port_additional_states.vault_port_stake_account_bump
    )]
    pub vault_port_stake_account: Box<Account<'info, PortStakeAccount>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"port_reward".as_ref()], 
        bump = port_additional_states.vault_port_reward_token_bump
    )]
    pub vault_port_reward_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub vault_port_sub_reward_token: Box<Account<'info, TokenAccount>>,

    /// ID of the staking pool
    #[account(mut)]
    pub port_staking_pool: Box<Account<'info, PortStakingPool>>,

    /// CHECK: safe
    #[account(
        executable,
        address = port_lending_id(),
    )]
    pub port_lend_program: AccountInfo<'info>,

    /// CHECK: safe
    #[account(
        executable,
        address = port_staking_id(),
    )]
    pub port_stake_program: AccountInfo<'info>,

    // NOTE safe to ignore port_staking_reward_pool and  port_staking_sub_reward_pool
    // because they are checked by port_stake_program
    /// CHECK: safe
    #[account(mut)]
    //#[soteria(ignore)]
    pub port_staking_reward_pool: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    //#[soteria(ignore)]
    pub port_staking_sub_reward_pool: AccountInfo<'info>,

    /// CHECK: safe
    //#[soteria(ignore)]
    pub port_staking_authority: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ClaimPortReward>) -> Result<()> {
    let claim_reward_context = CpiContext::new(
        ctx.accounts.port_stake_program.clone(),
        port_anchor_adaptor::ClaimReward {
            stake_account_owner: ctx.accounts.vault_authority.clone(),
            stake_account: ctx.accounts.vault_port_stake_account.to_account_info(),
            staking_pool: ctx.accounts.port_staking_pool.to_account_info(),
            reward_token_pool: ctx.accounts.port_staking_reward_pool.clone(),
            reward_dest: ctx.accounts.vault_port_reward_token.to_account_info(),
            staking_program_authority: ctx.accounts.port_staking_authority.clone(),
            clock: ctx.accounts.clock.to_account_info(),
            token_program: ctx.accounts.port_lend_program.clone(),
        },
    );

    port_anchor_adaptor::claim_reward(
        claim_reward_context.with_signer(&[&ctx.accounts.vault.authority_seeds()]),
        ctx.accounts
            .port_additional_states
            .sub_reward_available
            .as_some(ctx.accounts.port_staking_sub_reward_pool.clone()),
        ctx.accounts
            .port_additional_states
            .sub_reward_available
            .as_some(ctx.accounts.vault_port_sub_reward_token.to_account_info()),
    )
}
