use anchor_lang::prelude::*;

use std::cmp::Ordering;

use crate::errors::ErrorCode;

#[account]
#[derive(Debug)]
pub struct Vault {
    pub vault_authority: Pubkey,

    /// Account which is allowed to call restricted instructions
    /// Also the authority of the fee receiver account
    pub owner: Pubkey,

    pub authority_seed: Pubkey,

    pub authority_bump: [u8; 1],

    // Account where reserve tokens are stored
    pub vault_reserve_token: Pubkey,

    // Account where solend LP tokens are stored
    pub vault_solend_lp_token: Pubkey,

    pub vault_port_lp_token: Pubkey,

    pub vault_jet_lp_token: Pubkey,

    // Mint address of vault LP tokens
    pub lp_token_mint: Pubkey,

    // Mint address of the tokens that are stored in vault
    pub reserve_token_mint: Pubkey,

    pub fee_receiver: Pubkey,

    pub fee_bps: u8,

    // Last slot when vault was updated
    pub last_update: LastUpdate,

    // Total value of vault denominated in the reserve token
    pub total_value: u64,

    pub allocations: Allocations,

    pub strategy_type: StrategyType,
}

impl Vault {
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
