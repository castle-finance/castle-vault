use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount};

use std::convert::Into;

use crate::state::Vault;

#[derive(Accounts)]
pub struct InitializePool<'info> {
    pub vault_authority: AccountInfo<'info>,

    #[account(signer, zero)]
    pub vault: Box<Account<'info, Vault>>,

    // Mint address of pool LP token
    #[account(mut)]
    pub lp_token_mint: Account<'info, Mint>,

    // Account where tokens in pool are stored
    #[account(mut)]
    pub vault_reserve_token_account: Account<'info, TokenAccount>,

    // Account where pool LP tokens are minted to 
    #[account(mut)]
    pub owner_lp_token_account: Account<'info, TokenAccount>,

    // SPL token program
    pub token_program: AccountInfo<'info>,    
}

// Context for calling token mintTo
impl<'info> InitializePool<'info> {
    fn mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.lp_token_mint.to_account_info().clone(),
            to: self.owner_lp_token_account.to_account_info().clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

pub fn handler(ctx: Context<InitializePool>) -> ProgramResult {
    let (____pool_authority, bump_seed) = Pubkey::find_program_address(
        &[&ctx.accounts.vault.to_account_info().key.to_bytes()],
        ctx.program_id,
    );   
    let seeds = &[
        &ctx.accounts.vault.to_account_info().key.to_bytes(),
        &[bump_seed][..],
    ];

    // TODO safety checks

    // TODO remove this logic and add an init check to deposit
    // Mint initial LP tokens
    // TODO make smaller as to not int overflow with more $ in pool
    let initial_amount:u64 = 1000000;
    token::mint_to(
        ctx.accounts.mint_to_context().with_signer(&[&seeds[..]]),
        initial_amount,
    )?;

    // Initialize reserve pool
    let vault = &mut ctx.accounts.vault;
    vault.bump_seed = bump_seed;
    vault.token_program = *ctx.accounts.token_program.key;
    vault.reserve_token_account = *ctx.accounts.vault_reserve_token_account.to_account_info().key;
    vault.lp_token_mint = *ctx.accounts.lp_token_mint.to_account_info().key;
    vault.reserve_token_mint = ctx.accounts.vault_reserve_token_account.mint;

    Ok(())
}