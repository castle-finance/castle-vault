use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

use std::convert::Into;

use crate::state::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    pub vault_authority: AccountInfo<'info>,

    #[account(signer)]
    pub user_authority: AccountInfo<'info>,

    #[account(signer, zero)]
    pub vault: Box<Account<'info, Vault>>,

    // Mint address of pool LP token
    #[account(mut)]
    pub lp_token_mint: Account<'info, Mint>,

    // Account where tokens in pool are stored
    #[account(mut)]
    pub vault_reserve_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_reserve_token: Account<'info, TokenAccount>,

    // Account where pool LP tokens are minted to 
    #[account(mut)]
    pub user_lp_token: Account<'info, TokenAccount>,

    // SPL token program
    pub token_program: AccountInfo<'info>,    

    pub clock: Sysvar<'info, Clock>,
}

// Context for calling token mintTo
impl<'info> Initialize<'info> {
    fn mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        CpiContext::new(
            self.token_program.clone(),
            MintTo {
                mint: self.lp_token_mint.to_account_info().clone(),
                to: self.user_lp_token.to_account_info().clone(),
                authority: self.vault_authority.clone(),
            },
        )
    }

    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.clone(),
            Transfer {
                from: self.user_reserve_token.to_account_info().clone(),
                to: self.vault_reserve_token.to_account_info().clone(),
                authority: self.user_authority.clone(),
            },
        )
    }
}

pub fn handler(ctx: Context<Initialize>, initial_reserves: u64) -> ProgramResult {
    let (____pool_authority, bump_seed) = Pubkey::find_program_address(
        &[&ctx.accounts.vault.to_account_info().key.to_bytes()],
        ctx.program_id,
    );   
    let seeds = &[
        &ctx.accounts.vault.to_account_info().key.to_bytes(),
        &[bump_seed][..],
    ];

    // TODO safety checks

    token::transfer(
        ctx.accounts.transfer_context(),
        initial_reserves,
    )?;

    token::mint_to(
        ctx.accounts.mint_to_context().with_signer(&[&seeds[..]]),
        initial_reserves,
    )?;

    // Initialize reserve pool
    let vault = &mut ctx.accounts.vault;
    vault.bump_seed = bump_seed;
    vault.token_program = *ctx.accounts.token_program.key;
    vault.reserve_token_account = *ctx.accounts.vault_reserve_token.to_account_info().key;
    vault.lp_token_mint = *ctx.accounts.lp_token_mint.to_account_info().key;
    vault.reserve_token_mint = ctx.accounts.vault_reserve_token.mint;
    vault.last_update = LastUpdate::new(ctx.accounts.clock.slot);
    vault.total_value = initial_reserves;

    Ok(())
}