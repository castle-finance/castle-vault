use spl_math::{precise_number::PreciseNumber};

use std::convert::TryFrom;


pub const INITIAL_COLLATERAL_RATIO: u64 = 1;

// TODO move to state.rs as a Calculator?
pub fn calc_deposit_to_vault(
    reserve_token_amount: u64, lp_token_supply: u64, reserve_tokens_in_vault: u64
) -> Option<u64> {
    match reserve_tokens_in_vault {
        0 => return Some(INITIAL_COLLATERAL_RATIO * reserve_token_amount),
        _ => {
            let lp_token_amount = PreciseNumber::new(reserve_token_amount as u128)?;
            let lp_token_supply = PreciseNumber::new(lp_token_supply as u128)?;
            let reserve_tokens_in_vault = PreciseNumber::new(reserve_tokens_in_vault as u128)?;

            let lp_tokens_to_mint = lp_token_supply.checked_mul(
                &lp_token_amount.checked_div(&reserve_tokens_in_vault)?
            )?.to_imprecise()?;

            u64::try_from(lp_tokens_to_mint).ok()
        }
    }
}

pub fn calc_withdraw_from_vault(
    lp_token_amount: u64, lp_token_supply: u64, reserve_tokens_in_vault: u64
) -> Option<u64> {
    let lp_token_amount = PreciseNumber::new(lp_token_amount as u128)?;
    let lp_token_supply = PreciseNumber::new(lp_token_supply as u128)?;
    let reserve_tokens_in_vault = PreciseNumber::new(reserve_tokens_in_vault as u128)?;

    let reserve_tokens_to_transfer = reserve_tokens_in_vault.checked_mul(
        &lp_token_amount.checked_div(&lp_token_supply)?
    )?.to_imprecise()?;

    u64::try_from(reserve_tokens_to_transfer).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO add tests
}