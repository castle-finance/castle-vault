use std::convert::TryFrom;
use anchor_lang::prelude::*;

use anchor_lang::{
    solana_program::clock::{DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY},
};
use spl_math::precise_number::PreciseNumber;

use crate::errors::ErrorCode;

pub const INITIAL_COLLATERAL_RATIO: u64 = 1;

pub fn calc_reserve_to_lp(
    reserve_token_amount: u64,
    lp_token_supply: u64,
    reserve_tokens_in_vault: u64,
) -> Option<u64> {
    match reserve_tokens_in_vault {
        // Assert that lp supply is 0
        0 => Some(INITIAL_COLLATERAL_RATIO.checked_mul(reserve_token_amount)?),
        _ => {
            let reserve_token_amount = PreciseNumber::new(reserve_token_amount as u128)?;
            let lp_token_supply = PreciseNumber::new(lp_token_supply as u128)?;
            let reserve_tokens_in_vault = PreciseNumber::new(reserve_tokens_in_vault as u128)?;

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

/// Number of slots per year
/// 63072000
pub const SLOTS_PER_YEAR: u64 =
    DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;

pub const ONE_AS_BPS: u64 = 10000;

pub fn calc_carry_fees(profit: u64, fee_bps: u64) -> Result<u64> {
    profit
        .checked_mul(fee_bps)
        .map(|n| n / ONE_AS_BPS)
        .ok_or_else(|| ErrorCode::OverflowError.into())
}

pub fn calc_mgmt_fees(aum: u64, fee_bps: u64, slots_elapsed: u64) -> Result<u64> {
    [fee_bps, slots_elapsed]
        .iter()
        .try_fold(aum, |acc, r| acc.checked_mul(*r))
        .map(|n| n / ONE_AS_BPS / SLOTS_PER_YEAR)
        .ok_or_else(|| ErrorCode::OverflowError.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reserve_to_lp_initial() {
        assert_eq!(calc_reserve_to_lp(20, 0, 0), Some(20));
    }

    #[test]
    fn test_reserve_to_lp() {
        assert_eq!(calc_reserve_to_lp(100, 100, 100), Some(100));
        assert_eq!(calc_reserve_to_lp(10, 100, 200), Some(5));
        assert_eq!(calc_reserve_to_lp(10, 100, 201), Some(4));
    }

    #[test]
    fn test_lp_to_reserve() {
        assert_eq!(calc_lp_to_reserve(100, 100, 100), Some(100));
        assert_eq!(calc_lp_to_reserve(10, 100, 200), Some(20));
        assert_eq!(calc_lp_to_reserve(10, 101, 200), Some(19));
    }

    #[test]
    fn test_carry_fees() {
        assert_eq!(calc_carry_fees(50000, 10).unwrap(), 50)
    }

    #[test]
    fn test_mgmt_fees() {
        assert_eq!(calc_mgmt_fees(1261440000, 1000, 100).unwrap(), 200)
    }
}
