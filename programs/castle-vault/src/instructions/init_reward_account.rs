use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Token, TokenAccount},
};
use port_anchor_adaptor::{port_staking_id};
use std::convert::Into;

use crate::state::*;

#[account]
#[derive(Default)]
pub struct Dummy {
}

#[derive(Accounts)]
#[instruction(_reward_bump: u8)]
pub struct InitializeRewardAccount<'info> {
    #[account(
        mut,
        has_one = owner,
        has_one = vault_authority,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    pub vault_port_stake_account: AccountInfo<'info>,

    /// Token account for storing Port liquidity mining reward
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), port_native_token_mint.key().as_ref(), b"port_reward".as_ref()],
        bump = _reward_bump,
        token::authority = vault_authority,
        token::mint = port_native_token_mint,
    )]
    pub vault_port_reward_token: Box<Account<'info, TokenAccount>>,

    /// Mint of the port finance token (liquidity reward will be issued by this one)
    pub port_native_token_mint: AccountInfo<'info>,

    /// ID of the staking pool
    pub port_staking_pool: AccountInfo<'info>,

    #[account(
        executable,
        address = port_staking_id(),
    )]
    pub port_stake_program: AccountInfo<'info>,

    /// Account that pays for above account inits
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Owner of the vault
    /// Only this account can call restricted instructions
    /// Acts as authority of the fee receiver account
    pub owner: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<InitializeRewardAccount>, _reward_bump: u8) -> ProgramResult {

    let context = CpiContext::new(
        ctx.accounts.port_stake_program.clone(),
        port_anchor_adaptor::CreateStakeAccount {
            staking_pool: ctx.accounts.port_staking_pool.to_account_info(),
            stake_account: ctx.accounts.vault_port_stake_account.to_account_info(),
            owner: ctx.accounts.vault_authority.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        },
    );

    port_anchor_adaptor::create_stake_account(
        context.with_signer(&[&ctx.accounts.vault.authority_seeds()])
    )?;

    ctx.accounts.vault.vault_port_stake_account = ctx.accounts.vault_port_stake_account.key();
    ctx.accounts.vault.vault_port_reward_token = ctx.accounts.vault_port_reward_token.key();
    Ok(())
}
