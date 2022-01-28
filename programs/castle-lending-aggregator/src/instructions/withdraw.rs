use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, TokenAccount, Transfer};
use spl_math::precise_number::PreciseNumber;

use std::convert::Into;
use std::convert::TryFrom;

use crate::errors::ErrorCode;
use crate::state::Vault;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        constraint = !vault.last_update.stale @ ErrorCode::VaultIsNotRefreshed,
        has_one = vault_authority,
        has_one = vault_reserve_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub vault_lp_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub user_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub user_reserve_token: Box<Account<'info, TokenAccount>>,

    pub user_authority: Signer<'info>,

    #[account(address = token::ID)]
    pub token_program: AccountInfo<'info>,
}

impl<'info> Withdraw<'info> {
    fn burn_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        CpiContext::new(
            self.token_program.clone(),
            Burn {
                mint: self.vault_lp_mint.to_account_info(),
                to: self.user_lp_token.to_account_info(),
                authority: self.user_authority.to_account_info(),
            },
        )
    }

    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.clone(),
            Transfer {
                from: self.vault_reserve_token.to_account_info().clone(),
                to: self.user_reserve_token.to_account_info().clone(),
                authority: self.vault_authority.clone(),
            },
        )
    }
}

pub fn handler(ctx: Context<Withdraw>, lp_token_amount: u64) -> ProgramResult {
    msg!("Withdrawing {} lp tokens", lp_token_amount);

    let vault = &ctx.accounts.vault;

    let reserve_tokens_to_transfer = calc_withdraw_from_vault(
        lp_token_amount,
        ctx.accounts.vault_lp_mint.supply,
        vault.total_value,
    )
    .ok_or(ErrorCode::MathError)?;

    token::burn(ctx.accounts.burn_context(), lp_token_amount)?;

    // Transfer reserve tokens to user
    token::transfer(
        ctx.accounts
            .transfer_context()
            .with_signer(&[&vault.authority_seeds()]),
        reserve_tokens_to_transfer,
    )?;

    ctx.accounts.vault.total_value -= reserve_tokens_to_transfer;

    Ok(())
}

pub fn calc_withdraw_from_vault(
    lp_token_amount: u64,
    lp_token_supply: u64,
    reserve_tokens_in_vault: u64,
) -> Option<u64> {
    let lp_token_amount = PreciseNumber::new(lp_token_amount as u128)?;
    let lp_token_supply = PreciseNumber::new(lp_token_supply as u128)?;
    let reserve_tokens_in_vault = PreciseNumber::new(reserve_tokens_in_vault as u128)?;

    let reserve_tokens_to_transfer = lp_token_amount
        .checked_mul(&reserve_tokens_in_vault)?
        .checked_div(&lp_token_supply)?
        .floor()?
        .to_imprecise()?;

    u64::try_from(reserve_tokens_to_transfer).ok()
}
