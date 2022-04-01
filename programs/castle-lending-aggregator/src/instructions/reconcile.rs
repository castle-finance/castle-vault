use std::cmp;

use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    state::{Allocation, Provider, Vault},
};

// move this somewhere else?
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

    fn provider(&self) -> Provider;
    fn vault(&self) -> &Vault;
    fn vault_mut(&mut self) -> &mut Vault;

    fn get_allocation(&self) -> Allocation {
        self.vault().allocations[self.provider()]
    }

    fn reset_allocation(&mut self) {
        let provider = self.provider();
        self.vault_mut().allocations[provider] = self.vault().allocations[provider].reset();
    }
}

pub fn handler(ctx: Context<impl LendingMarket>, withdraw_option: u64) -> ProgramResult {
    match withdraw_option {
        // Normal case where reconcile is being called after rebalance
        0 => {
            let lp_tokens_in_vault = ctx.accounts.lp_tokens_in_vault();
            let current_value = ctx
                .accounts
                .convert_amount_lp_to_reserve(lp_tokens_in_vault)?;
            let allocation = ctx.accounts.get_allocation();
            //msg!("Desired allocation: {}", allocation.value);
            //msg!("Current allocation: {}", current_value);

            // Make sure that rebalance was called recently
            let clock = Clock::get()?;
            if allocation.last_update.slots_elapsed(clock.slot)? > 10 {
                return Err(ErrorCode::AllocationIsNotUpdated.into());
            }

            match allocation.value.checked_sub(current_value) {
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
                        .checked_sub(
                            ctx.accounts
                                .convert_amount_reserve_to_lp(allocation.value)?,
                        )
                        .ok_or(ErrorCode::MathError)?;

                    msg!("Redeeming {}", tokens_to_redeem);

                    ctx.accounts.redeem(tokens_to_redeem)?;
                }
            }
            ctx.accounts.reset_allocation();
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
