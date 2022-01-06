use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

use std::convert::Into;

use crate::errors::ErrorCode;
use crate::math::calc_deposit_to_vault;
use crate::state::Vault;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        constraint = !vault.last_update.stale @ ErrorCode::VaultIsNotRefreshed,
        has_one = lp_token_mint,
        has_one = vault_authority,
        has_one = vault_reserve_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    // Account where tokens in vault are stored
    #[account(mut)]
    pub vault_reserve_token: Account<'info, TokenAccount>,

    // Mint address of vault LP token
    #[account(mut)]
    pub lp_token_mint: Account<'info, Mint>,

    // Account from which tokens are transferred
    #[account(mut)]
    pub user_reserve_token: Account<'info, TokenAccount>,

    // Account where vault LP tokens are minted to 
    #[account(mut)]
    pub user_lp_token: Account<'info, TokenAccount>,

    pub user_authority: Signer<'info>,

    // SPL token program
    #[account(address = token::ID)]
    pub token_program: AccountInfo<'info>,
}

impl<'info> Deposit<'info> {
    fn mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        CpiContext::new(
            self.token_program.clone(),
            MintTo {
                mint: self.lp_token_mint.to_account_info(),
                to: self.user_lp_token.to_account_info(),
                authority: self.vault_authority.clone(),
            },
        )
    }

    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.clone(),
            Transfer {
                from: self.user_reserve_token.to_account_info(),
                to: self.vault_reserve_token.to_account_info(),
                authority: self.user_authority.to_account_info(),
            },
        )
    }
}

pub fn handler(ctx: Context<Deposit>, reserve_token_amount: u64) -> ProgramResult {
    let vault = &ctx.accounts.vault;

    let lp_tokens_to_mint = calc_deposit_to_vault(
        reserve_token_amount, 
        ctx.accounts.lp_token_mint.supply, 
        vault.total_value,
    ).ok_or(ErrorCode::MathError)?;

    token::transfer(
        ctx.accounts.transfer_context(),
        reserve_token_amount,
    )?;

    token::mint_to(
        ctx.accounts.mint_to_context().with_signer(
            &[&vault.authority_seeds()]
        ),
        lp_tokens_to_mint,
    )?;

    Ok(())
}