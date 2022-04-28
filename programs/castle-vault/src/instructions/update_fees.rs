use anchor_lang::prelude::*;

use crate::state::Vault;
use std::convert::Into;

use crate::init::{validate_fees, FeeArgs};

#[derive(Accounts)]
pub struct UpdateFees<'info, const N: usize> {
    #[account(
        mut,
        has_one = owner,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub owner: Signer<'info>,
}

pub fn handler<const N: usize>(ctx: Context<UpdateFees<N>>, new_fees: FeeArgs) -> ProgramResult {
    validate_fees(&new_fees)?;

    ctx.accounts.vault.fees.fee_carry_bps = new_fees.fee_carry_bps;
    ctx.accounts.vault.fees.fee_mgmt_bps = new_fees.fee_mgmt_bps;
    ctx.accounts.vault.fees.referral_fee_pct = new_fees.referral_fee_pct;

    Ok(())
}
