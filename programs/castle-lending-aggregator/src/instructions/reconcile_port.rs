use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount};
use port_anchor_adaptor::PortReserve;

use crate::{errors::ErrorCode, state::Vault};

#[derive(Accounts)]
pub struct ReconcilePort<'info> {
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_port_lp_token,
        constraint = !vault.allocations.port.last_update.stale @ ErrorCode::AllocationIsNotUpdated,
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

    #[account(mut)]
    pub port_reserve_state: Box<Account<'info, PortReserve>>,

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
                reserve: self.port_reserve_state.to_account_info(),
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
                reserve: self.port_reserve_state.to_account_info(),
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

    let port_exchange_rate = ctx.accounts.port_reserve_state.collateral_exchange_rate()?;
    let current_port_value =
        port_exchange_rate.collateral_to_liquidity(ctx.accounts.vault_port_lp_token.amount)?;
    let allocation = ctx.accounts.vault.allocations.port;

    match allocation.value.checked_sub(current_port_value) {
        Some(tokens_to_deposit) => {
            port_anchor_adaptor::deposit_reserve(
                ctx.accounts
                    .port_deposit_reserve_liquidity_context()
                    .with_signer(&[&vault.authority_seeds()]),
                tokens_to_deposit,
            )?;
        }
        None => {
            let tokens_to_redeem = ctx
                .accounts
                .vault_port_lp_token
                .amount
                .checked_sub(port_exchange_rate.liquidity_to_collateral(allocation.value)?)
                .ok_or(ErrorCode::MathError)?;

            port_anchor_adaptor::redeem(
                ctx.accounts
                    .port_redeem_reserve_collateral_context()
                    .with_signer(&[&vault.authority_seeds()]),
                tokens_to_redeem,
            )?;
        }
    }

    ctx.accounts.vault.allocations.port.reset();

    Ok(())
}
