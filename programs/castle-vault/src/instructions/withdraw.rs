use boolinator::Boolinator;

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::{
    errors::ErrorCode,
    state::{Vault, VaultFlags},
};

#[event]
pub struct WithdrawEvent {
    vault: Pubkey,
    user: Pubkey,
    amount: u64,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// Vault state account
    /// Checks that the refresh has been called in the same slot
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        constraint = !vault.value.last_update.is_stale(clock.slot)? @ ErrorCode::VaultIsNotRefreshed,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = lp_token_mint,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    /// CHECK: safe
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

impl<'info> Withdraw<'info> {
    /// CpiContext for burning vault lp tokens from user account
    fn burn_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint: self.lp_token_mint.to_account_info(),
                from: self.user_lp_token.to_account_info(),
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
pub fn handler(ctx: Context<Withdraw>, lp_token_amount: u64) -> Result<()> {
    #[cfg(feature = "debug")]
    msg!("Withdrawing {} lp tokens", lp_token_amount);

    // Check that withdrawals are not halted
    (!ctx
        .accounts
        .vault
        .get_halt_flags()
        .contains(VaultFlags::HALT_DEPOSITS_WITHDRAWS))
    .ok_or(ErrorCode::HaltedVault)?;

    let vault = &ctx.accounts.vault;

    let reserve_tokens_to_transfer = crate::math::calc_lp_to_reserve(
        lp_token_amount,
        ctx.accounts.vault.lp_token_supply,
        vault.value.value,
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

    ctx.accounts.vault.lp_token_supply = ctx
        .accounts
        .vault
        .lp_token_supply
        .checked_sub(lp_token_amount)
        .ok_or(ErrorCode::MathError)?;

    // This is so that the SDK can read an up-to-date total value without calling refresh
    ctx.accounts.vault.value.value = ctx
        .accounts
        .vault
        .value
        .value
        .checked_sub(reserve_tokens_to_transfer)
        .ok_or(ErrorCode::MathError)?;

    emit!(WithdrawEvent {
        vault: ctx.accounts.vault.key(),
        user: ctx.accounts.user_authority.key(),
        amount: lp_token_amount,
    });

    Ok(())
}
