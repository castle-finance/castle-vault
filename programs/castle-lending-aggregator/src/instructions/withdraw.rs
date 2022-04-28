use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use std::convert::Into;

use crate::errors::ErrorCode;
use crate::state::Vault;

#[derive(Accounts)]
pub struct Withdraw<'info, const N: usize> {
    /// Vault state account
    /// Checks that the refresh has been called in the same slot
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        constraint = !vault.last_update.is_stale(clock.slot)? @ ErrorCode::VaultIsNotRefreshed,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = lp_token_mint,
    )]
    pub vault: Box<Account<'info, Vault<N>>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's reserve tokens
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Mint for the vault's lp token
    #[account(mut)]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// Token account from which lp tokens are burned
    #[account(mut)]
    pub user_lp_token: Box<Account<'info, TokenAccount>>,

    /// Account where vault LP tokens are transferred to
    #[account(mut)]
    //#[soteria(ignore)]
    pub user_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Authority of the user_lp_token account
    /// Must be a signer
    pub user_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info, const N: usize> Withdraw<'info, N> {
    /// CpiContext for burning vault lp tokens from user account
    fn burn_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint: self.lp_token_mint.to_account_info(),
                to: self.user_lp_token.to_account_info(),
                authority: self.user_authority.to_account_info(),
            },
        )
    }

    /// CpiContext for transferring reserve tokens from vault to user
    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.vault_reserve_token.to_account_info(),
                to: self.user_reserve_token.to_account_info(),
                authority: self.vault_authority.clone(),
            },
        )
    }
}

/// Withdraw from the vault
///
/// Burns the user's lp tokens and transfers their share of reserve tokens
pub fn handler<const N: usize>(ctx: Context<Withdraw<N>>, lp_token_amount: u64) -> ProgramResult {
    #[cfg(feature = "debug")]
    msg!("Withdrawing {} lp tokens", lp_token_amount);

    let vault = &ctx.accounts.vault;

    let reserve_tokens_to_transfer = crate::math::calc_lp_to_reserve(
        lp_token_amount,
        ctx.accounts.lp_token_mint.supply,
        vault.total_value,
    )
    .ok_or(ErrorCode::MathError)?;

    token::burn(ctx.accounts.burn_context(), lp_token_amount)?;

    #[cfg(feature = "debug")]
    msg!("Transferring {} reserve tokens", reserve_tokens_to_transfer);

    token::transfer(
        ctx.accounts
            .transfer_context()
            .with_signer(&[&vault.authority_seeds()]),
        reserve_tokens_to_transfer,
    )?;

    // This is so that the SDK can read an up-to-date total value without calling refresh
    ctx.accounts.vault.total_value = ctx
        .accounts
        .vault
        .total_value
        .checked_sub(reserve_tokens_to_transfer)
        .ok_or(ErrorCode::MathError)?;

    Ok(())
}
