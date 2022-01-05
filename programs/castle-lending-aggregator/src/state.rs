use anchor_lang::prelude::*;

use std::cmp::Ordering;

use crate::errors::ErrorCode;

#[account]
pub struct Vault {
    // Bump seed used to generate PDA
    pub bump_seed: u8,

    // SPL token program
    pub token_program: Pubkey,

    // Account where reserve tokens are stored
    pub reserve_token_account: Pubkey,

    // Mint address of pool LP tokens
    pub lp_token_mint: Pubkey,

    // Mint address of the tokens that are stored in pool
    pub reserve_token_mint: Pubkey,

    // Last slot when vault was updated
    pub last_update: LastUpdate,

    // Total value of vault denominated in the reserve token
    pub total_value: u64,
}

/// Number of slots to consider stale after
pub const STALE_AFTER_SLOTS_ELAPSED: u64 = 1;

#[derive(Clone, Copy, AnchorDeserialize, AnchorSerialize)]
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
        let slots_elapsed = slot
            .checked_sub(self.slot)
            .ok_or(ErrorCode::MathError)?;
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
