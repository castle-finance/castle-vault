use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Token, TokenAccount},
};
use port_anchor_adaptor::{port_lending_id, port_staking_id, PortObligation, PortStakeAccount};
use std::convert::Into;

use crate::{errors::ErrorCode, state::*};

#[derive(Accounts)]
#[instruction(_obligation_bump:u8, _stake_bump:u8, _reward_bump: u8)]
pub struct InitializeRewardAccount<'info> {
    #[account(
        mut,
        has_one = owner,
        has_one = vault_authority,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub port_additional_states: Box<Account<'info, VaultPortAdditionalState>>,

    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"port_obligation".as_ref()],
        bump = _obligation_bump,
        space = PortObligation::LEN,
        owner = port_lend_program.key(),
    )]
    pub vault_port_obligation: AccountInfo<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"port_stake".as_ref()],
        bump = _stake_bump,
        space = PortStakeAccount::LEN,
        owner = port_stake_program.key(),
    )]
    pub vault_port_stake_account: AccountInfo<'info>,

    /// Token account for storing Port liquidity mining reward
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"port_reward".as_ref()],
        bump = _reward_bump,
        token::authority = vault_authority,
        token::mint = port_reward_token_mint,
    )]
    pub vault_port_reward_token: Box<Account<'info, TokenAccount>>,

    /// Mint of the port finance token (liquidity reward will be issued by this one)
    pub port_reward_token_mint: AccountInfo<'info>,

    /// ID of the staking pool
    pub port_staking_pool: AccountInfo<'info>,

    #[account(
        executable,
        address = port_staking_id(),
    )]
    pub port_stake_program: AccountInfo<'info>,

    #[account(
        executable,
        address = port_lending_id(),
    )]
    pub port_lend_program: AccountInfo<'info>,

    pub port_lending_market: AccountInfo<'info>,

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
    ctx: Context<InitializeRewardAccount>,
    _obligation_bump: u8,
    _stake_bump: u8,
    _reward_bump: u8,
) -> ProgramResult {
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

    let port_additional_states_pda_key = Pubkey::create_program_address(
        &[
            ctx.accounts.vault.key().as_ref(),
            b"port_additional_state".as_ref(),
            &[ctx.accounts.vault.vault_port_additional_state_bump],
        ],
        &ctx.program_id,
    )?;
    if port_additional_states_pda_key != ctx.accounts.port_additional_states.key() {
        return Err(ErrorCode::InvalidAccount.into());
    }

    ctx.accounts
        .port_additional_states
        .vault_port_stake_account_bump = _stake_bump;
    ctx.accounts
        .port_additional_states
        .vault_port_reward_token_bump = _reward_bump;
    ctx.accounts
        .port_additional_states
        .vault_port_obligation_bump = _obligation_bump;

    Ok(())
}
