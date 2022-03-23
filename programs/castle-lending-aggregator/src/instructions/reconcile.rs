use std::cmp;

use anchor_lang::prelude::*;

use crate::errors::ErrorCode;

// Split into CPI, Data, Vault traits?
pub trait LendingMarket {
    fn deposit(&self, amount: u64) -> ProgramResult;
    fn redeem(&self, amount: u64) -> ProgramResult;

    // TODO separate these fns into ExchangeRate struct
    // OR Amount struct like Jet does which handles conversions implicitly
    fn convert_amount_reserve_to_lp(&self, amount: u64) -> Result<u64, ProgramError>;
    fn convert_amount_lp_to_reserve(&self, amount: u64) -> Result<u64, ProgramError>;

    fn reserve_tokens_in_vault(&self) -> u64;
    fn lp_tokens_in_vault(&self) -> u64;
    fn get_allocation(&self) -> u64;
    fn reset_allocations(&mut self);
}

pub fn handler(ctx: Context<impl LendingMarket>, withdraw_option: u64) -> ProgramResult {
    match withdraw_option {
        // Normal case where reconcile is being called after rebalance
        0 => {
            // TODO check stale allocation

            let lp_tokens_in_vault = ctx.accounts.lp_tokens_in_vault();
            let current_solend_value = ctx
                .accounts
                .convert_amount_lp_to_reserve(lp_tokens_in_vault)?;
            let allocation = ctx.accounts.get_allocation();

            match allocation.checked_sub(current_solend_value) {
                Some(tokens_to_deposit) => {
                    // Make sure that the amount deposited is not more than the vault has in reserves
                    let tokens_to_deposit_checked =
                        cmp::min(tokens_to_deposit, ctx.accounts.reserve_tokens_in_vault());

                    msg!("Depositing {}", tokens_to_deposit_checked);
                    ctx.accounts.deposit(tokens_to_deposit_checked)?;
                }
                None => {
                    let tokens_to_redeem = ctx
                        .accounts
                        .lp_tokens_in_vault()
                        .checked_sub(ctx.accounts.convert_amount_reserve_to_lp(allocation)?)
                        .ok_or(ErrorCode::MathError)?;

                    msg!("Redeeming {}", tokens_to_redeem);

                    ctx.accounts.redeem(tokens_to_redeem)?;
                }
            }
            ctx.accounts.reset_allocations();
        }
        // Extra case where reconcile is being called in same tx as a withdraw or by vault owner to emergency brake
        _ => {
            // TODO check that tx is signed by owner OR there is a withdraw tx later with the withdraw_option <= withdraw_amount

            let tokens_to_redeem = ctx.accounts.convert_amount_lp_to_reserve(withdraw_option)?;
            msg!("Redeeming {}", tokens_to_redeem);
            ctx.accounts.redeem(tokens_to_redeem)?;
        }
    }
    Ok(())
}
