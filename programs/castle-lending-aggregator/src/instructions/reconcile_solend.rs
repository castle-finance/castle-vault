use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use solend::SolendReserve;

use crate::{cpi::solend, errors::ErrorCode, state::Vault};

use std::cmp;

#[derive(Accounts)]
pub struct ReconcileSolend<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
        has_one = solend_reserve,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's reserve tokens
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's solend lp tokens
    #[account(mut)]
    pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        executable,
        address = spl_token_lending::ID,
    )]
    pub solend_program: AccountInfo<'info>,

    pub solend_market_authority: AccountInfo<'info>,

    #[account(owner = solend_program.key())]
    pub solend_market: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve: Box<Account<'info, SolendReserve>>,

    #[account(mut)]
    pub solend_lp_mint: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve_token: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,
}

impl<'info> ReconcileSolend<'info> {
    /// CpiContext for depositing to solend
    pub fn solend_deposit_reserve_liquidity_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::DepositReserveLiquidity<'info>> {
        CpiContext::new(
            self.solend_program.clone(),
            solend::DepositReserveLiquidity {
                lending_program: self.solend_program.clone(),
                source_liquidity: self.vault_reserve_token.to_account_info(),
                destination_collateral_account: self.vault_solend_lp_token.to_account_info(),
                reserve: self.solend_reserve.to_account_info(),
                reserve_collateral_mint: self.solend_lp_mint.clone(),
                reserve_liquidity_supply: self.solend_reserve_token.clone(),
                lending_market: self.solend_market.clone(),
                lending_market_authority: self.solend_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program_id: self.token_program.to_account_info(),
            },
        )
    }

    /// CpiContext for redeeming from solend
    fn solend_redeem_reserve_collateral_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::RedeemReserveCollateral<'info>> {
        CpiContext::new(
            self.solend_program.clone(),
            solend::RedeemReserveCollateral {
                lending_program: self.solend_program.clone(),
                source_collateral: self.vault_solend_lp_token.to_account_info(),
                destination_liquidity: self.vault_reserve_token.to_account_info(),
                reserve: self.solend_reserve.to_account_info(),
                reserve_collateral_mint: self.solend_lp_mint.clone(),
                reserve_liquidity_supply: self.solend_reserve_token.clone(),
                lending_market: self.solend_market.clone(),
                lending_market_authority: self.solend_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program_id: self.token_program.to_account_info(),
            },
        )
    }
}

// TODO eliminate duplication of redeem logic
/// Deposit or withdraw from solend to match the stored allocation or to process a withdrawal
pub fn handler(ctx: Context<ReconcileSolend>, withdraw_option: Option<u64>) -> ProgramResult {
    msg!("Reconciling solend");

    let vault = &ctx.accounts.vault;
    let solend_exchange_rate = ctx.accounts.solend_reserve.collateral_exchange_rate()?;

    match withdraw_option {
        // Normal case where reconcile is being called after rebalance
        None => {
            let current_solend_value = solend_exchange_rate
                .collateral_to_liquidity(ctx.accounts.vault_solend_lp_token.amount)?;
            let allocation = ctx.accounts.vault.allocations.solend;

            match allocation.value.checked_sub(current_solend_value) {
                Some(tokens_to_deposit) => {
                    // Make sure that the amount deposited is not more than the vault has in reserves
                    let tokens_to_deposit_checked =
                        cmp::min(tokens_to_deposit, ctx.accounts.vault_reserve_token.amount);

                    msg!("Depositing {}", tokens_to_deposit_checked);

                    if tokens_to_deposit != 0 {
                        solend::deposit_reserve_liquidity(
                            ctx.accounts
                                .solend_deposit_reserve_liquidity_context()
                                .with_signer(&[&vault.authority_seeds()]),
                            tokens_to_deposit_checked,
                        )?;
                    }
                }
                None => {
                    let tokens_to_redeem = ctx
                        .accounts
                        .vault_solend_lp_token
                        .amount
                        .checked_sub(
                            solend_exchange_rate.liquidity_to_collateral(allocation.value)?,
                        )
                        .ok_or(ErrorCode::MathError)?;

                    msg!("Redeeming {}", tokens_to_redeem);

                    solend::redeem_reserve_collateral(
                        ctx.accounts
                            .solend_redeem_reserve_collateral_context()
                            .with_signer(&[&vault.authority_seeds()]),
                        tokens_to_redeem,
                    )?;
                }
            }
            ctx.accounts.vault.allocations.solend.reset();
        }
        // Extra case where reconcile is being called in same tx as a withdraw or by vault owner to emergency brake
        Some(withdraw_amount) => {
            // TODO check that tx is signed by owner OR there is a withdraw tx later with the withdraw_option <= withdraw_amount

            let tokens_to_redeem = solend_exchange_rate.liquidity_to_collateral(withdraw_amount)?;

            msg!("Redeeming {}", tokens_to_redeem);

            solend::redeem_reserve_collateral(
                ctx.accounts
                    .solend_redeem_reserve_collateral_context()
                    .with_signer(&[&vault.authority_seeds()]),
                tokens_to_redeem,
            )?;
        }
    }
    Ok(())
}
