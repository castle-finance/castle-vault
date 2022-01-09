use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount};

use crate::state::Vault;

#[derive(Accounts)]
pub struct ReconcilePort<'info> {
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_port_lp_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        executable,
        address = port_variable_rate_lending_instructions::ID,
    )]
    pub port_program: AccountInfo<'info>,

    pub port_market_authority: AccountInfo<'info>,

    #[account(owner = port_program.key())]
    pub port_market: AccountInfo<'info>,

    #[account(mut, owner = port_program.key())]
    pub port_reserve_state: AccountInfo<'info>,

    #[account(mut)]
    pub port_lp_mint: AccountInfo<'info>,

    #[account(mut)]
    pub port_reserve_token: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    #[account(address = token::ID)]
    pub token_program: AccountInfo<'info>,
}

impl<'info> ReconcilePort<'info> {
    pub fn port_deposit_reserve_liquidity_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, port_anchor_adaptor::Deposit<'info>> {
        CpiContext::new(
            self.port_program.clone(),
            port_anchor_adaptor::Deposit {
                source_liquidity: self.vault_reserve_token.to_account_info(),
                destination_collateral: self.vault_port_lp_token.to_account_info(),
                reserve: self.port_reserve_state.clone(),
                reserve_collateral_mint: self.port_lp_mint.clone(),
                reserve_liquidity_supply: self.port_reserve_token.clone(),
                lending_market: self.port_market.clone(),
                lending_market_authority: self.port_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program: self.token_program.clone(),
            },
        )
    }

    fn port_redeem_reserve_collateral_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, port_anchor_adaptor::Redeem<'info>> {
        CpiContext::new(
            self.port_program.clone(),
            port_anchor_adaptor::Redeem {
                source_collateral: self.vault_port_lp_token.to_account_info(),
                destination_liquidity: self.vault_reserve_token.to_account_info(),
                reserve: self.port_reserve_state.clone(),
                reserve_collateral_mint: self.port_lp_mint.clone(),
                reserve_liquidity_supply: self.port_reserve_token.clone(),
                lending_market: self.port_market.clone(),
                lending_market_authority: self.port_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program: self.token_program.clone(),
            },
        )
    }
}

pub fn handler(ctx: Context<ReconcilePort>) -> ProgramResult {
    let vault = &ctx.accounts.vault;

    let deposit_amount = vault.to_reconcile[1].deposit;
    let redeem_amount = vault.to_reconcile[1].redeem;

    if deposit_amount > 0 {
        port_anchor_adaptor::deposit_reserve(
            ctx.accounts
                .port_deposit_reserve_liquidity_context()
                .with_signer(&[&vault.authority_seeds()]),
            deposit_amount,
        )?;
    }
    if redeem_amount > 0 {
        port_anchor_adaptor::redeem(
            ctx.accounts
                .port_redeem_reserve_collateral_context()
                .with_signer(&[&vault.authority_seeds()]),
            redeem_amount,
        )?;
    }

    ctx.accounts.vault.to_reconcile[1].reset();

    Ok(())
}
