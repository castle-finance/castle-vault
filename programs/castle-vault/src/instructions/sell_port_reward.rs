use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token::{Token, TokenAccount};

use crate::state::{DexStates, OrcaLegacyAccounts, Vault, VaultPortAdditionalState};

#[derive(Accounts)]
pub struct SellPortReward<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    #[account(
        seeds = [vault.key().as_ref(), b"port_additional_state".as_ref()], 
        bump = vault.vault_port_additional_state_bump
    )]
    pub port_additional_states: Box<Account<'info, VaultPortAdditionalState>>,

    #[account(
        seeds = [vault.key().as_ref(), b"dex_states".as_ref()], 
        bump = vault.dex_states_bump,
    )]
    pub dex_states: Box<Account<'info, DexStates>>,

    #[account(
        seeds = [vault.key().as_ref(), b"dex_orca_legacy".as_ref()],
        bump = dex_states.orca_legacy_accounts_bump,
        has_one = orca_swap_program,
        has_one = orca_swap_state,
        has_one = orca_swap_authority,
        has_one = orca_input_token_account,
        has_one = orca_output_token_account,
        has_one = orca_swap_token_mint,
    )]
    pub orca_legacy_accounts: Box<Account<'info, OrcaLegacyAccounts>>,

    pub orca_swap_state: AccountInfo<'info>,

    pub orca_swap_authority: AccountInfo<'info>,

    #[account(mut)]
    pub orca_input_token_account: AccountInfo<'info>,

    #[account(mut)]
    pub orca_output_token_account: AccountInfo<'info>,

    #[account(mut)]
    pub orca_swap_token_mint: AccountInfo<'info>,

    #[account(mut)]
    pub orca_fee_account: AccountInfo<'info>,

    #[account(executable)]
    pub orca_swap_program: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"port_reward".as_ref()], 
        bump = port_additional_states.vault_port_reward_token_bump
    )]
    pub vault_port_reward_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<SellPortReward>) -> ProgramResult {
    let amount_in = ctx.accounts.vault_port_reward_token.amount;
    let minimum_amount_out = 1;

    let ix = spl_token_swap::instruction::swap(
        &ctx.accounts.orca_swap_program.key(),
        &ctx.accounts.token_program.key(),
        &ctx.accounts.orca_swap_state.key(),
        &ctx.accounts.orca_swap_authority.key(),
        &ctx.accounts.vault_authority.key(),
        &ctx.accounts.vault_port_reward_token.key(),
        &ctx.accounts.orca_input_token_account.key(),
        &ctx.accounts.orca_output_token_account.key(),
        &ctx.accounts.vault_reserve_token.key(),
        &ctx.accounts.orca_swap_token_mint.key(),
        &ctx.accounts.orca_fee_account.key(),
        None,
        spl_token_swap::instruction::Swap {
            amount_in,
            minimum_amount_out,
        },
    )?;

    let accounts: Vec<AccountInfo> = vec![
        ctx.accounts.orca_swap_program.clone(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.orca_swap_state.clone(),
        ctx.accounts.orca_swap_authority.clone(),
        ctx.accounts.vault_authority.clone(),
        ctx.accounts.vault_port_reward_token.to_account_info(),
        ctx.accounts.orca_input_token_account.clone(),
        ctx.accounts.orca_output_token_account.clone(),
        ctx.accounts.vault_reserve_token.to_account_info(),
        ctx.accounts.orca_swap_token_mint.clone(),
        ctx.accounts.orca_fee_account.clone(),
    ];

    invoke_signed(&ix, &accounts, &[&ctx.accounts.vault.authority_seeds()]).map_err(|e| e.into())
}
