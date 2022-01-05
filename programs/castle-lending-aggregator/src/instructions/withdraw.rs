use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Burn, TokenAccount, Transfer};

use std::convert::Into; 

use crate::cpi::solend;
use crate::errors::ErrorCode;
use crate::math::calc_withdraw_from_vault;
use crate::state::Vault;


#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(signer)]
    pub user_authority: AccountInfo<'info>,

    // Account from which pool tokens are burned
    #[account(mut)]
    pub user_lp_token: Box<Account<'info, TokenAccount>>,

    // Account where tokens are transferred to
    #[account(mut)]
    pub user_reserve_token: Box<Account<'info, TokenAccount>>,

    // Account where tokens in pool are stored
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    // Mint address of pool LP token
    #[account(mut)]
    pub vault_lp_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,

    pub solend_program: AccountInfo<'info>,

    pub solend_market_authority: AccountInfo<'info>,

    pub solend_market: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve_state: AccountInfo<'info>,

    #[account(mut)]
    pub solend_lp_mint: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve_token: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    // SPL token program
    pub token_program: AccountInfo<'info>,
}

impl<'info> Withdraw<'info> {
    fn burn_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        CpiContext::new(
            self.token_program.clone(),
            Burn {
                mint: self.vault_lp_mint.to_account_info().clone(),
                to: self.user_lp_token.to_account_info().clone(),
                authority: self.user_authority.clone(),
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

    fn solend_redeem_reserve_collateral_context(&self) -> CpiContext<'_, '_, '_, 'info, solend::RedeemReserveCollateral<'info>> {
        CpiContext::new(
            self.solend_program.clone(),
            solend::RedeemReserveCollateral {
                lending_program: self.solend_program.clone(),
                source_collateral: self.vault_solend_lp_token.to_account_info().clone(),
                destination_liquidity: self.vault_reserve_token.to_account_info().clone(),
                refreshed_reserve_account: self.solend_reserve_state.clone(),
                reserve_collateral_mint: self.solend_lp_mint.clone(),
                reserve_liquidity: self.solend_reserve_token.clone(),
                lending_market: self.solend_market.clone(),
                lending_market_authority: self.solend_market_authority.clone(),
                user_transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info().clone(),
                token_program_id: self.token_program.clone(),
            },
        )
    }
}

pub fn handler(ctx: Context<Withdraw>, lp_token_amount: u64) -> ProgramResult {
    let vault = &ctx.accounts.vault;

    // TODO check accounts

    // TODO check last update slot

    let reserve_tokens_to_transfer = calc_withdraw_from_vault(
        lp_token_amount, 
        ctx.accounts.vault_lp_mint.supply, 
        vault.total_value,
    ).ok_or(ErrorCode::MathError)?;

    let seeds = &[
        &vault.to_account_info().key.to_bytes(), 
        &[vault.bump_seed][..],
    ];

    let solend_exchange_rate = solend::solend_accessor::exchange_rate(&ctx.accounts.solend_reserve_state)?;
    let solend_collateral_amount = solend_exchange_rate.liquidity_to_collateral(
        reserve_tokens_to_transfer - ctx.accounts.vault_reserve_token.amount
    )?;
    solend::redeem_reserve_collateral(
        ctx.accounts.solend_redeem_reserve_collateral_context().with_signer(&[&seeds[..]]),
        solend_collateral_amount,
    )?;

    token::burn(
        ctx.accounts.burn_context(),
        lp_token_amount,
    )?;

    // Transfer reserve tokens to user
    token::transfer(
        ctx.accounts.transfer_context().with_signer(&[&seeds[..]]),
        reserve_tokens_to_transfer,
    )?;

    Ok(())
}