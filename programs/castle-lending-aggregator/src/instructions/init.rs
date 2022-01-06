use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use std::convert::Into;

use crate::state::*;

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitBumpSeeds {
    authority: u8, 
    reserve: u8, 
    lp_mint: u8,
}

#[derive(Accounts)]
#[instruction(bumps: InitBumpSeeds)]
pub struct Initialize<'info> {
    #[account(seeds = [vault.key().as_ref(), b"authority"], bump = bumps.authority)]
    pub vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(zero)]
    pub vault: Box<Account<'info, Vault>>,

    pub reserve_token_mint: Account<'info, Mint>,

    // Mint address of pool LP token
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"lp_mint"],
        bump = bumps.lp_mint,
        mint::authority = vault_authority,
        mint::decimals = reserve_token_mint.decimals,
    )]
    pub lp_token_mint: Account<'info, Mint>,

    // Account where tokens in pool are stored
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), reserve_token_mint.key().as_ref()],
        bump = bumps.reserve,
        token::authority = vault_authority,
        token::mint = reserve_token_mint,
    )]
    pub vault_reserve_token: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,

    // SPL token program
    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,    

    pub rent: Sysvar<'info, Rent>,

    pub clock: Sysvar<'info, Clock>,
}

pub fn handler(ctx: Context<Initialize>, _bumps: InitBumpSeeds) -> ProgramResult {
    let (____pool_authority, vault_bump) = Pubkey::find_program_address(
        &[&ctx.accounts.vault.to_account_info().key.to_bytes()],
        ctx.program_id,
    );
    // TODO safety checks
    let vault = &mut ctx.accounts.vault;
    vault.bump_seed = vault_bump; 
    vault.token_program = *ctx.accounts.token_program.key;
    vault.reserve_token_account = *ctx.accounts.vault_reserve_token.to_account_info().key;
    vault.lp_token_mint = *ctx.accounts.lp_token_mint.to_account_info().key;
    vault.reserve_token_mint = ctx.accounts.vault_reserve_token.mint;
    vault.last_update = LastUpdate::new(ctx.accounts.clock.slot);
    vault.total_value = 0;

    Ok(())
}