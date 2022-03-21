use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::{
    DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY,
};

use std::cmp::Ordering;

use crate::errors::ErrorCode;

/// Number of slots per year
pub const SLOTS_PER_YEAR: u64 =
    DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;

pub const ONE_AS_BPS: u64 = 10000;

#[account]
#[derive(Debug)]
pub struct Vault {
    /// Account which is allowed to call restricted instructions
    /// Also the authority of the fee receiver account
    pub owner: Pubkey,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: Pubkey,

    pub authority_seed: Pubkey,

    pub authority_bump: [u8; 1],

    /// Account where reserve tokens are stored
    pub vault_reserve_token: Pubkey,

    /// Account where solend LP tokens are stored
    pub vault_solend_lp_token: Pubkey,

    /// Account where port LP tokens are stored
    pub vault_port_lp_token: Pubkey,

    /// Account where jet LP tokens are stored
    pub vault_jet_lp_token: Pubkey,

    /// Mint address of vault LP tokens
    pub lp_token_mint: Pubkey,

    /// Mint address of the tokens that are stored in vault
    pub reserve_token_mint: Pubkey,

    /// Account that fees from this vault are sent to
    pub fee_receiver: Pubkey,

    /// Basis points of the accrued interest that gets sent to the fee_receiver
    pub fee_carry_bps: u16,

    /// Basis points of the AUM that gets sent to the fee_receiver
    pub fee_mgmt_bps: u16,

    /// Last slot when vault was refreshed
    pub last_update: LastUpdate,

    /// Total value of vault denominated in the reserve token
    pub total_value: u64,

    /// Prospective allocations set by rebalance, executed by reconciles
    pub allocations: Allocations,

    /// Strategy type that is executed during rebalance
    pub strategy_type: StrategyType,
}

impl Vault {
    // TODO use a more specific error type
    pub fn calculate_fees(&self, new_vault_value: u64, slot: u64) -> Result<u64, ProgramError> {
        let vault_value_diff = new_vault_value.saturating_sub(self.total_value);
        let slots_elapsed = self.last_update.slots_elapsed(slot)?;
        // Return early to avoid divide by zero
        if slots_elapsed == 0 {
            return Ok(0);
        }
        //msg!("Slots elapsed: {}", slots_elapsed);
        //msg!("New vault value: {}", new_vault_value);
        //msg!("Old vault value: {}", self.total_value);

        let carry = vault_value_diff
            .checked_mul(self.fee_carry_bps as u64)
            .ok_or(ErrorCode::OverflowError)?
            .checked_div(ONE_AS_BPS)
            .ok_or(ErrorCode::OverflowError)?;

        let mgmt = new_vault_value
            .checked_mul(self.fee_carry_bps as u64)
            .ok_or(ErrorCode::OverflowError)?
            / ONE_AS_BPS
            / SLOTS_PER_YEAR
            / slots_elapsed;

        //msg!("Carry: {}", carry);
        //msg!("Mgmt: {}", mgmt);

        let fees = carry.checked_add(mgmt).ok_or(ErrorCode::OverflowError)?;
        Ok(fees)
    }

    pub fn update_value(&mut self, new_value: u64, slot: u64) {
        self.total_value = new_value;
        self.last_update.update_slot(slot);
    }

    pub fn authority_seeds(&self) -> [&[u8]; 3] {
        [
            self.authority_seed.as_ref(),
            b"authority".as_ref(),
            &self.authority_bump,
        ]
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub enum StrategyType {
    MaxYield,
    EqualAllocation,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug, Default)]
pub struct Allocations {
    pub solend: Allocation,
    pub port: Allocation,
    pub jet: Allocation,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug, Default)]
pub struct Allocation {
    pub value: u64,
    pub last_update: LastUpdate,
}

impl Allocation {
    pub fn update(&mut self, value: u64, slot: u64) {
        self.value = value;
        self.last_update.update_slot(slot);
    }

    pub fn reset(&mut self) {
        self.value = 0;
        self.last_update.mark_stale();
    }
}

/// Number of slots to consider stale after
pub const STALE_AFTER_SLOTS_ELAPSED: u64 = 1;

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug, Default)]
pub struct LastUpdate {
    pub slot: u64,
    pub stale: bool,
}

impl LastUpdate {
    /// Create new last update
    pub fn new(slot: u64) -> Self {
        Self { slot, stale: true }
    }

    /// Return slots elapsed since given slot
    pub fn slots_elapsed(&self, slot: u64) -> Result<u64, ProgramError> {
        let slots_elapsed = slot.checked_sub(self.slot).ok_or(ErrorCode::MathError)?;
        Ok(slots_elapsed)
    }

    /// Set last update slot
    pub fn update_slot(&mut self, slot: u64) {
        self.slot = slot;
        self.stale = false;
    }

    /// Set stale to true
    pub fn mark_stale(&mut self) {
        self.stale = true;
    }

    // TODO mark stale if slots elapsed and update checks to use is_stale
    /// Check if marked stale or last update slot is too long ago
    pub fn is_stale(&self, slot: u64) -> Result<bool, ProgramError> {
        Ok(self.stale || self.slots_elapsed(slot)? >= STALE_AFTER_SLOTS_ELAPSED)
    }
}

impl PartialEq for LastUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot
    }
}

impl PartialOrd for LastUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.slot.partial_cmp(&other.slot)
    }
}
