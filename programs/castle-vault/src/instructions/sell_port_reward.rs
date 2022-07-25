use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token::{Token, TokenAccount};

use crate::{
    errors::ErrorCode,
    state::{DexStates, OrcaLegacyAccounts, Vault, VaultPortAdditionalState},
};

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
    /// CHECK: safe
    pub vault_authority: AccountInfo<'info>,

    #[account(
        seeds = [vault.key().as_ref(), b"port_additional_state".as_ref()], 
        bump
    )]
    pub port_additional_states: Box<Account<'info, VaultPortAdditionalState>>,

    #[account(
        seeds = [vault.key().as_ref(), b"dex_states".as_ref()], 
        bump
    )]
    pub dex_states: Box<Account<'info, DexStates>>,

    #[account(
        seeds = [vault.key().as_ref(), b"dex_orca_legacy".as_ref()],
        bump,
        has_one = orca_swap_program,
    )]
    pub orca_legacy_accounts: Box<Account<'info, OrcaLegacyAccounts>>,

    /// CHECK: safe
    pub orca_swap_state: AccountInfo<'info>,

    // DANGER why can we ignore this? because the the Orca program will check this
    //        and fail the CPI if this account is invalid
    // TODO Security audit to ensure it's really ok.
    /// CHECK: safe
    //#[soteria(ignore)]
    pub orca_swap_authority: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub orca_input_token_account: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub orca_output_token_account: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub orca_swap_token_mint: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub orca_fee_account: AccountInfo<'info>,

    /// CHECK: safe
    #[account(executable)]
    pub orca_swap_program: AccountInfo<'info>,

    #[account(mut)]
    // No need to check this account because coins in it will be sold
    // and vault_reserve_token collects the revenue.
    // We only have to check the integrity of vault_reserve_token
    pub vault_port_reward_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<SellPortReward>, market_id: u8) -> Result<()> {
    if market_id as usize > ctx.accounts.orca_legacy_accounts.orca_markets.len() {
        msg!("Invalid market Id");
        return Err(ErrorCode::InvalidArgument.into());
    }

    // No need to check other orca accounts, because those are checked by the swap program.
    // using data stored in orca_swap_state.
    // We only have to check the integrity of orca_swap_state
    if ctx.accounts.orca_swap_state.key()
        != ctx.accounts.orca_legacy_accounts.orca_markets[market_id as usize]
    {
        return Err(ErrorCode::InvalidAccount.into());
    }

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
