use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Burn, TokenAccount, Transfer};

use std::convert::Into; 

use crate::errors::ErrorCode;
use crate::math::{calc_withdraw_from_vault, get_vault_value};
use crate::state::Vault;


#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(signer)]
    pub user_authority: AccountInfo<'info>,

    // Account from which pool tokens are burned
    #[account(mut)]
    pub user_lp_token_account: Account<'info, TokenAccount>,

    // Account where tokens in pool are stored
    #[account(mut)]
    pub vault_reserve_token_account: Account<'info, TokenAccount>,

    // Account where tokens are transferred to
    #[account(mut)]
    pub user_reserve_token_account: Account<'info, TokenAccount>,

    // Mint address of pool LP token
    #[account(mut)]
    pub lp_token_mint: Account<'info, Mint>,

    // SPL token program
    pub token_program: AccountInfo<'info>,
}

impl<'info> Withdraw<'info> {
    fn burn_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        let cpi_accounts = Burn {
            mint: self.lp_token_mint.to_account_info().clone(),
            to: self.user_lp_token_account.to_account_info().clone(),
            authority: self.user_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_reserve_token_account.to_account_info().clone(),
            to: self.user_reserve_token_account.to_account_info().clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

pub fn handler(ctx: Context<Withdraw>, lp_token_amount: u64) -> ProgramResult {
    let vault = &ctx.accounts.vault;

    // TODO check accounts

    let reserve_tokens_in_vault = get_vault_value(ctx.accounts.vault_reserve_token_account.amount);

    let reserve_tokens_to_transfer = calc_withdraw_from_vault(
        lp_token_amount, 
        ctx.accounts.lp_token_mint.supply, 
        reserve_tokens_in_vault,
    ).ok_or(ErrorCode::MathError)?;

    let seeds = &[
        &vault.to_account_info().key.to_bytes(), 
        &[vault.bump_seed][..],
    ];

    // TODO redeem collateral

    // Transfer reserve tokens to user
    token::transfer(
        ctx.accounts.transfer_context().with_signer(&[&seeds[..]]),
            reserve_tokens_to_transfer,
    )?;

    // Burn LP tokens
    token::burn(
        ctx.accounts.burn_context(),
        lp_token_amount,
    )?;
    
    Ok(())
}