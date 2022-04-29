use spl_math::precise_number::PreciseNumber;

use std::convert::TryFrom;

pub const INITIAL_COLLATERAL_RATIO: u64 = 1;

// TODO move to state.rs as a Calculator?
// TODO return Results

pub fn calc_reserve_to_lp(
    reserve_token_amount: u64,
    lp_token_supply: u64,
    reserve_tokens_in_vault: u64,
) -> Option<u64> {
    match reserve_tokens_in_vault {
        0 => Some(INITIAL_COLLATERAL_RATIO.checked_mul(reserve_token_amount)?),
        _ => {
            let reserve_token_amount = PreciseNumber::new(reserve_token_amount as u128)?;
            let lp_token_supply = PreciseNumber::new(lp_token_supply as u128)?;
            let reserve_tokens_in_vault = PreciseNumber::new(reserve_tokens_in_vault as u128)?;

            // at what values of PreciseNumber will this fail?
            let lp_tokens_to_mint = lp_token_supply
                .checked_mul(&reserve_token_amount)?
                .checked_div(&reserve_tokens_in_vault)?
                .floor()?
                .to_imprecise()?;

            u64::try_from(lp_tokens_to_mint).ok()
        }
    }
}

pub fn calc_lp_to_reserve(
    lp_token_amount: u64,
    lp_token_supply: u64,
    reserve_tokens_in_vault: u64,
) -> Option<u64> {
    let lp_token_amount = PreciseNumber::new(lp_token_amount as u128)?;
    let lp_token_supply = PreciseNumber::new(lp_token_supply as u128)?;
    let reserve_tokens_in_vault = PreciseNumber::new(reserve_tokens_in_vault as u128)?;

    let reserve_tokens_to_transfer = lp_token_amount
        .checked_mul(&reserve_tokens_in_vault)?
        .checked_div(&lp_token_supply)?
        .floor()?
        .to_imprecise()?;

    u64::try_from(reserve_tokens_to_transfer).ok()
}

// TODO add unit tests
