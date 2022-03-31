use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

use std::convert::Into;

use crate::errors::ErrorCode;
use crate::state::Vault;

#[derive(Accounts)]
pub struct Deposit<'info> {
    /// Vault state account
    /// Checks that the refresh has been called in the same slot
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        constraint = !vault.last_update.is_stale(clock.slot)? @ ErrorCode::VaultIsNotRefreshed,
        has_one = lp_token_mint,
        has_one = vault_authority,
        has_one = vault_reserve_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's reserve tokens
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Mint for the vault's lp token
    #[account(mut)]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// Token account from which reserve tokens are transferred
    #[account(mut)]
    pub user_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Account where vault LP tokens are minted to
    #[account(mut)]
    pub user_lp_token: Box<Account<'info, TokenAccount>>,

    /// Authority of the user_reserve_token account
    /// Must be a signer
    pub user_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info> Deposit<'info> {
    /// CpiContext for minting vault Lp tokens to user account
    fn mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            MintTo {
                mint: self.lp_token_mint.to_account_info(),
                to: self.user_lp_token.to_account_info(),
                authority: self.vault_authority.clone(),
            },
        )
    }

    /// CpiContext for transferring reserve tokens from user to vault
    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_reserve_token.to_account_info(),
                to: self.vault_reserve_token.to_account_info(),
                authority: self.user_authority.to_account_info(),
            },
        )
    }
}

/// Deposit to the vault
///
/// Transfers reserve tokens from user to vault and mints their share of lp tokens
pub fn handler(ctx: Context<Deposit>, reserve_token_amount: u64) -> ProgramResult {
    msg!("Depositing {} reserve tokens", reserve_token_amount);

    let vault = &ctx.accounts.vault;

    let lp_tokens_to_mint = crate::math::calc_reserve_to_lp(
        reserve_token_amount,
        ctx.accounts.lp_token_mint.supply,
        vault.total_value,
    )
    .ok_or(ErrorCode::MathError)?;

    token::transfer(ctx.accounts.transfer_context(), reserve_token_amount)?;

    msg!("Minting {} LP tokens", lp_tokens_to_mint);

    token::mint_to(
        ctx.accounts
            .mint_to_context()
            .with_signer(&[&vault.authority_seeds()]),
        lp_tokens_to_mint,
    )?;

    // This is so that the SDK can read an up-to-date total value without calling refresh
    ctx.accounts.vault.total_value = ctx
        .accounts
        .vault
        .total_value
        .checked_add(reserve_token_amount)
        .ok_or(ErrorCode::MathError)?;

    Ok(())
}
