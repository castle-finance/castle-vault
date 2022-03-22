use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use port_anchor_adaptor::PortReserve;

use crate::{errors::ErrorCode, state::Vault};

use std::cmp;

#[derive(Accounts)]
pub struct ReconcilePort<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_port_lp_token,
        has_one = port_reserve,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's reserve tokens
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's port lp tokens
    #[account(mut)]
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    // NOTE address check is commented out because port has a different
    // ID in devnet than they do in mainnet
    #[account(
        executable,
        //address = port_variable_rate_lending_instructions::ID,
    )]
    pub port_program: AccountInfo<'info>,

    pub port_market_authority: AccountInfo<'info>,

    #[account(owner = port_program.key())]
    pub port_market: AccountInfo<'info>,

    #[account(mut)]
    pub port_reserve: Box<Account<'info, PortReserve>>,

    #[account(mut)]
    pub port_lp_mint: AccountInfo<'info>,

    #[account(mut)]
    pub port_reserve_token: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,
}

impl<'info> ReconcilePort<'info> {
    /// CpiContext for depositing to port
    pub fn port_deposit_reserve_liquidity_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, port_anchor_adaptor::Deposit<'info>> {
        CpiContext::new(
            self.port_program.clone(),
            port_anchor_adaptor::Deposit {
                source_liquidity: self.vault_reserve_token.to_account_info(),
                destination_collateral: self.vault_port_lp_token.to_account_info(),
                reserve: self.port_reserve.to_account_info(),
                reserve_collateral_mint: self.port_lp_mint.clone(),
                reserve_liquidity_supply: self.port_reserve_token.clone(),
                lending_market: self.port_market.clone(),
                lending_market_authority: self.port_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        )
    }

    /// CpiContext for redeeming from port
    fn port_redeem_reserve_collateral_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, port_anchor_adaptor::Redeem<'info>> {
        CpiContext::new(
            self.port_program.clone(),
            port_anchor_adaptor::Redeem {
                source_collateral: self.vault_port_lp_token.to_account_info(),
                destination_liquidity: self.vault_reserve_token.to_account_info(),
                reserve: self.port_reserve.to_account_info(),
                reserve_collateral_mint: self.port_lp_mint.clone(),
                reserve_liquidity_supply: self.port_reserve_token.clone(),
                lending_market: self.port_market.clone(),
                lending_market_authority: self.port_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        )
    }
}

// TODO eliminate duplication of redeem logic
/// Deposit or withdraw from port to match the stored allocation or to process a withdrawal
pub fn handler(ctx: Context<ReconcilePort>, withdraw_option: Option<u64>) -> ProgramResult {
    msg!("Reconciling Port");

    let vault = &ctx.accounts.vault;
    let port_exchange_rate = ctx.accounts.port_reserve.collateral_exchange_rate()?;

    match withdraw_option {
        // Normal case where reconcile is being called after rebalance
        None => {
            // TODO check !vault.allocations.port.stale
            let current_port_value = port_exchange_rate
                .collateral_to_liquidity(ctx.accounts.vault_port_lp_token.amount)?;
            let allocation = ctx.accounts.vault.allocations.port;

            match allocation.value.checked_sub(current_port_value) {
                Some(tokens_to_deposit) => {
                    // Make sure that the amount deposited is not more than the vault has in reserves
                    let tokens_to_deposit_checked =
                        cmp::min(tokens_to_deposit, ctx.accounts.vault_reserve_token.amount);

                    msg!("Depositing {}", tokens_to_deposit_checked);

                    if tokens_to_deposit_checked != 0 {
                        port_anchor_adaptor::deposit_reserve(
                            ctx.accounts
                                .port_deposit_reserve_liquidity_context()
                                .with_signer(&[&vault.authority_seeds()]),
                            port_anchor_adaptor::Cluster::Devnet,
                            tokens_to_deposit_checked,
                        )?;
                    }
                }
                None => {
                    let tokens_to_redeem = ctx
                        .accounts
                        .vault_port_lp_token
                        .amount
                        .checked_sub(port_exchange_rate.liquidity_to_collateral(allocation.value)?)
                        .ok_or(ErrorCode::MathError)?;

                    msg!("Redeeming {}", tokens_to_redeem);

                    port_anchor_adaptor::redeem(
                        ctx.accounts
                            .port_redeem_reserve_collateral_context()
                            .with_signer(&[&vault.authority_seeds()]),
                        port_anchor_adaptor::Cluster::Devnet,
                        tokens_to_redeem,
                    )?;
                }
            }
            ctx.accounts.vault.allocations.port.reset();
        }
        // Extra case where reconcile is being called in same tx as a withdraw or by vault owner to emergency brake
        Some(withdraw_amount) => {
            // TODO check that tx is signed by owner OR there is a withdraw tx later with the withdraw_option <= withdraw_amount

            msg!("Redeeming {}", withdraw_amount);

            let tokens_to_redeem = port_exchange_rate.liquidity_to_collateral(withdraw_amount)?;

            port_anchor_adaptor::redeem(
                ctx.accounts
                    .port_redeem_reserve_collateral_context()
                    .with_signer(&[&vault.authority_seeds()]),
                port_anchor_adaptor::Cluster::Devnet,
                tokens_to_redeem,
            )?;
        }
    }
    Ok(())
}
