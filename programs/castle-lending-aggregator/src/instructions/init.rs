use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use std::convert::Into;

use crate::state::*;

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitBumpSeeds {
    authority: u8,
    reserve: u8,
    solend_lp: u8,
    port_lp: u8,
    jet_lp: u8,
    lp_mint: u8,
}

#[derive(Accounts)]
#[instruction(bumps: InitBumpSeeds)]
pub struct Initialize<'info> {
    #[account(zero)]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"authority".as_ref()], 
        bump = bumps.authority,
    )]
    pub vault_authority: AccountInfo<'info>,

    // Mint address of pool LP token
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"lp_mint".as_ref()],
        bump = bumps.lp_mint,
        mint::authority = vault_authority,
        mint::decimals = reserve_token_mint.decimals,
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    // Account where tokens in pool are stored
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), reserve_token_mint.key().as_ref()],
        bump = bumps.reserve,
        token::authority = vault_authority,
        token::mint = reserve_token_mint,
    )]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), solend_lp_token_mint.key().as_ref()],
        bump = bumps.solend_lp,
        token::authority = vault_authority,
        token::mint = solend_lp_token_mint,
    )]
    pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), port_lp_token_mint.key().as_ref()],
        bump = bumps.port_lp,
        token::authority = vault_authority,
        token::mint = port_lp_token_mint,
    )]
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), jet_lp_token_mint.key().as_ref()],
        bump = bumps.jet_lp,
        token::authority = vault_authority,
        token::mint = jet_lp_token_mint,
    )]
    pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,

    pub jet_program: AccountInfo<'info>,

    pub jet_market: AccountInfo<'info>,

    pub jet_market_authority: AccountInfo<'info>,

    pub jet_reserve_state: AccountInfo<'info>,

    pub reserve_token_mint: Box<Account<'info, Mint>>,

    pub solend_lp_token_mint: AccountInfo<'info>,

    pub port_lp_token_mint: AccountInfo<'info>,

    pub jet_lp_token_mint: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,

    pub clock: Sysvar<'info, Clock>,
}

pub fn handler(
    ctx: Context<Initialize>,
    bumps: InitBumpSeeds,
    strategy_type: StrategyType,
) -> ProgramResult {
    // TODO also store lending market reserve account addresses in vault?

    let vault = &mut ctx.accounts.vault;
    vault.vault_authority = ctx.accounts.vault_authority.key();
    vault.authority_seed = vault.key();
    vault.authority_bump = [bumps.authority];
    vault.vault_reserve_token = ctx.accounts.vault_reserve_token.key();
    vault.vault_solend_lp_token = ctx.accounts.vault_solend_lp_token.key();
    vault.vault_port_lp_token = ctx.accounts.vault_port_lp_token.key();
    vault.vault_jet_lp_token = ctx.accounts.vault_jet_lp_token.key();
    vault.lp_token_mint = ctx.accounts.lp_token_mint.key();
    vault.reserve_token_mint = ctx.accounts.reserve_token_mint.key();
    vault.last_update = LastUpdate::new(ctx.accounts.clock.slot);
    vault.total_value = 0;
    vault.strategy_type = strategy_type;

    Ok(())
}
