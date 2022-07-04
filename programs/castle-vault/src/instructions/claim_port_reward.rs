use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use port_anchor_adaptor::{port_lending_id, port_staking_id};

use crate::errors::ErrorCode;
use crate::state::{Vault, VaultPortAdditionalState};

#[derive(Accounts)]
pub struct ClaimPortReward<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    /// TODO check if we should verify has_one for the staking accounts and staking pool ID
    #[account(
        mut,
        has_one = vault_authority,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    pub port_additional_states: Box<Account<'info, VaultPortAdditionalState>>,

    #[account(mut)]
    pub vault_port_stake_account: AccountInfo<'info>,

    #[account(mut)]
    pub vault_port_reward_token: AccountInfo<'info>,

    #[account(mut)]
    pub vault_port_sub_reward_token: AccountInfo<'info>,

    /// ID of the staking pool
    #[account(mut)]
    pub port_staking_pool: AccountInfo<'info>,

    #[account(
        executable,
        address = port_lending_id(),
    )]
    pub port_lend_program: AccountInfo<'info>,

    #[account(
        executable,
        address = port_staking_id(),
    )]
    pub port_stake_program: AccountInfo<'info>,

    #[account(mut)]
    pub port_staking_reward_pool: AccountInfo<'info>,

    #[account(mut)]
    pub port_staking_sub_reward_pool: AccountInfo<'info>,

    //#[soteria(ignore)]
    pub port_staking_authority: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ClaimPortReward>) -> ProgramResult {
    let port_additional_states_pda_key = Pubkey::create_program_address(
        &[
            ctx.accounts.vault.key().as_ref(),
            b"port_additional_state".as_ref(),
            &[ctx.accounts.vault.vault_port_additional_state_bump],
        ],
        ctx.program_id,
    )?;
    let port_stake_account_pda_key = Pubkey::create_program_address(
        &[
            ctx.accounts.vault.key().as_ref(),
            b"port_stake".as_ref(),
            &[ctx
                .accounts
                .port_additional_states
                .vault_port_stake_account_bump],
        ],
        ctx.program_id,
    )?;
    let port_reward_account_pda_key = Pubkey::create_program_address(
        &[
            ctx.accounts.vault.key().as_ref(),
            b"port_reward".as_ref(),
            &[ctx
                .accounts
                .port_additional_states
                .vault_port_reward_token_bump],
        ],
        ctx.program_id,
    )?;
    let port_sub_reward_account_pda_key = Pubkey::create_program_address(
        &[
            ctx.accounts.vault.key().as_ref(),
            b"port_sub_reward".as_ref(),
            &[ctx
                .accounts
                .port_additional_states
                .vault_port_sub_reward_token_bump],
        ],
        ctx.program_id,
    )?;
    if port_additional_states_pda_key != ctx.accounts.port_additional_states.key()
        || port_stake_account_pda_key != ctx.accounts.vault_port_stake_account.key()
        || port_reward_account_pda_key != ctx.accounts.vault_port_reward_token.key()
        || port_sub_reward_account_pda_key != ctx.accounts.vault_port_sub_reward_token.key()
        || ctx.accounts.port_additional_states.port_staking_pool
            != ctx.accounts.port_staking_pool.key()
    {
        return Err(ErrorCode::InvalidAccount.into());
    }

    let claim_reward_context = CpiContext::new(
        ctx.accounts.port_stake_program.clone(),
        port_anchor_adaptor::ClaimReward {
            stake_account_owner: ctx.accounts.vault_authority.clone(),
            stake_account: ctx.accounts.vault_port_stake_account.clone(),
            staking_pool: ctx.accounts.port_staking_pool.clone(),
            reward_token_pool: ctx.accounts.port_staking_reward_pool.clone(),
            reward_dest: ctx.accounts.vault_port_reward_token.clone(),
            staking_program_authority: ctx.accounts.port_staking_authority.clone(),
            clock: ctx.accounts.clock.to_account_info(),
            token_program: ctx.accounts.port_lend_program.clone(),
        },
    );

    port_anchor_adaptor::claim_reward(
        claim_reward_context.with_signer(&[&ctx.accounts.vault.authority_seeds()]),
        Some(ctx.accounts.port_staking_sub_reward_pool.clone()),
        Some(ctx.accounts.vault_port_sub_reward_token.clone()),
    )
}
