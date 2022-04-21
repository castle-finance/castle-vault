use std::cmp;

use anchor_lang::prelude::*;

use crate::{errors::ErrorCode, rebalance::assets::Provider, state::Vault};

const MAX_SLOTS_SINCE_ALLOC_UPDATE: u64 = 20;

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
}

pub trait HasVault {
    fn vault(&self) -> &Vault;
    fn vault_mut(&mut self) -> &mut Vault;
}

// TODO make this a custom derive procmacro
#[macro_export]
macro_rules! impl_has_vault {
    ($($t:ty),+ $(,)?) => ($(
        impl $crate::instructions::reconcile::HasVault for $t {
            fn vault(&self) -> &Vault {
                self.vault.deref()
            }

            fn vault_mut(&mut self) -> &mut Vault {
                self.vault.deref_mut()
            }
        }
    )+)
}

pub fn handler<T: LendingMarket + HasVault>(
    ctx: Context<T>,
    withdraw_option: u64,
) -> ProgramResult {
    let provider = ctx.accounts.provider();
    match withdraw_option {
        // Normal case where reconcile is being called after rebalance
        0 => {
            let lp_tokens_in_vault = ctx.accounts.lp_tokens_in_vault();
            let current_value = ctx
                .accounts
                .convert_amount_lp_to_reserve(lp_tokens_in_vault)?;
            let allocation = ctx.accounts.vault().allocations[provider];

            #[cfg(feature = "debug")]
            {
                msg!("Desired allocation: {}", allocation.value);
                msg!("Current allocation: {}", current_value);
            }

            // Make sure that rebalance was called recently
            let clock = Clock::get()?;
            if allocation.last_update.slots_elapsed(clock.slot)? > MAX_SLOTS_SINCE_ALLOC_UPDATE {
                return Err(ErrorCode::AllocationIsNotUpdated.into());
            }

            match allocation.value.checked_sub(current_value) {
                Some(tokens_to_deposit) => {
                    // Make sure that the amount deposited is not more than the vault has in reserves
                    let tokens_to_deposit_checked =
                        cmp::min(tokens_to_deposit, ctx.accounts.reserve_tokens_in_vault());

                    #[cfg(feature = "debug")]
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

                    #[cfg(feature = "debug")]
                    msg!("Redeeming {}", tokens_to_redeem);

                    ctx.accounts.redeem(tokens_to_redeem)?;
                }
            }
            ctx.accounts.vault_mut().allocations[provider].reset();
        }
        // Extra case where reconcile is being called in same tx as a withdraw or by vault owner to emergency brake
        _ => {
            // TODO check that tx is signed by owner OR there is a withdraw tx later with the withdraw_option <= withdraw_amount

            let tokens_to_redeem = ctx.accounts.convert_amount_lp_to_reserve(withdraw_option)?;

            #[cfg(feature = "debug")]
            msg!("Redeeming {}", tokens_to_redeem);

            ctx.accounts.redeem(tokens_to_redeem)?;
        }
    }
    Ok(())
}
