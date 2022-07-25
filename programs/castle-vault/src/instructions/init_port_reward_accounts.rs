use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use port_anchor_adaptor::{
    port_lending_id, port_staking_id, PortLendingMarket, PortObligation, PortStakeAccount,
    PortStakingPool,
};
use std::convert::Into;

use crate::{errors::ErrorCode, state::*};

#[derive(Accounts)]
pub struct InitializePortRewardAccounts<'info> {
    #[account(
        mut,
        has_one = owner,
        has_one = vault_authority,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// CHECK: safe
    pub vault_authority: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"port_additional_state".as_ref()], 
        bump,
    )]
    pub port_additional_states: Box<Account<'info, VaultPortAdditionalState>>,

    /// CHECK: safe
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"port_obligation".as_ref()],
        bump,
        space = PortObligation::LEN,
        owner = port_lend_program.key(),
    )]
    pub vault_port_obligation: AccountInfo<'info>,

    /// CHECK: safe
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"port_stake".as_ref()],
        bump,
        space = PortStakeAccount::LEN,
        owner = port_stake_program.key(),
    )]
    pub vault_port_stake_account: AccountInfo<'info>,

    /// Token account for storing Port liquidity mining reward
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"port_reward".as_ref()],
        bump,
        token::authority = vault_authority,
        token::mint = port_reward_token_mint,
    )]
    pub vault_port_reward_token: Box<Account<'info, TokenAccount>>,

    /// Token account for storing Port liquidity mining sub-reward
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"port_sub_reward".as_ref()],
        bump,
        token::authority = vault_authority,
        token::mint = port_sub_reward_token_mint,
    )]
    pub vault_port_sub_reward_token: Box<Account<'info, TokenAccount>>,

    // Account to which the token should be transfered for the purpose of staking
    pub port_lp_token_account: Box<Account<'info, TokenAccount>>,

    /// Mint of the port finance token (liquidity reward will be issued by this one)
    pub port_reward_token_mint: Box<Account<'info, Mint>>,

    /// Mint of the port stake sub-reward token
    pub port_sub_reward_token_mint: Box<Account<'info, Mint>>,

    /// ID of the staking pool
    pub port_staking_pool: Box<Account<'info, PortStakingPool>>,

    /// CHECK: safe
    pub port_staking_reward_pool: AccountInfo<'info>,

    /// CHECK: safe
    pub port_staking_sub_reward_pool: AccountInfo<'info>,

    /// CHECK: safe
    pub port_reward_token_oracle: AccountInfo<'info>,

    /// CHECK: safe
    pub port_sub_reward_token_oracle: AccountInfo<'info>,

    /// CHECK: safe
    #[account(
        executable,
        address = port_staking_id(),
    )]
    pub port_stake_program: AccountInfo<'info>,

    /// CHECK: safe
    #[account(
        executable,
        address = port_lending_id(),
    )]
    pub port_lend_program: AccountInfo<'info>,

    pub port_lending_market: Box<Account<'info, PortLendingMarket>>,

    /// Account that pays for above account inits
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Owner of the vault
    /// Only this account can call restricted instructions
    /// Acts as authority of the fee receiver account
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub clock: Sysvar<'info, Clock>,

    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializePortRewardAccounts>,
    sub_reward_available: bool,
) -> Result<()> {
    let init_obligation_ctx = CpiContext::new(
        ctx.accounts.port_lend_program.clone(),
        port_anchor_adaptor::InitObligation {
            obligation: ctx.accounts.vault_port_obligation.to_account_info(),
            lending_market: ctx.accounts.port_lending_market.to_account_info(),
            obligation_owner: ctx.accounts.vault_authority.to_account_info(),
            clock: ctx.accounts.clock.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
            spl_token_id: ctx.accounts.token_program.to_account_info(),
        },
    );

    let init_stake_ctx = CpiContext::new(
        ctx.accounts.port_stake_program.clone(),
        port_anchor_adaptor::CreateStakeAccount {
            staking_pool: ctx.accounts.port_staking_pool.to_account_info(),
            stake_account: ctx.accounts.vault_port_stake_account.to_account_info(),
            owner: ctx.accounts.vault_authority.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        },
    );

    port_anchor_adaptor::init_obligation(
        init_obligation_ctx.with_signer(&[&ctx.accounts.vault.authority_seeds()]),
    )?;

    port_anchor_adaptor::create_stake_account(
        init_stake_ctx.with_signer(&[&ctx.accounts.vault.authority_seeds()]),
    )?;

    ctx.accounts
        .port_additional_states
        .vault_port_stake_account_bump = *ctx
        .bumps
        .get("vault_port_stake_account")
        .ok_or(ErrorCode::BumpError)?;
    ctx.accounts
        .port_additional_states
        .vault_port_reward_token_bump = *ctx
        .bumps
        .get("vault_port_reward_token")
        .ok_or(ErrorCode::BumpError)?;
    ctx.accounts
        .port_additional_states
        .vault_port_obligation_bump = *ctx
        .bumps
        .get("vault_port_obligation")
        .ok_or(ErrorCode::BumpError)?;
    ctx.accounts
        .port_additional_states
        .vault_port_sub_reward_token_bump = *ctx
        .bumps
        .get("vault_port_sub_reward_token")
        .ok_or(ErrorCode::BumpError)?;

    ctx.accounts.port_additional_states.port_lp_token_account =
        ctx.accounts.port_lp_token_account.key();
    ctx.accounts.port_additional_states.port_staking_pool = ctx.accounts.port_staking_pool.key();
    ctx.accounts.port_additional_states.port_staking_reward_pool =
        ctx.accounts.port_staking_reward_pool.key();
    ctx.accounts
        .port_additional_states
        .port_staking_sub_reward_pool = ctx.accounts.port_staking_sub_reward_pool.key();

    ctx.accounts.port_additional_states.sub_reward_available = sub_reward_available;

    ctx.accounts.port_additional_states.port_reward_token_oracle =
        ctx.accounts.port_reward_token_oracle.key();
    ctx.accounts
        .port_additional_states
        .port_sub_reward_token_oracle = ctx.accounts.port_sub_reward_token_oracle.key();

    Ok(())
}
