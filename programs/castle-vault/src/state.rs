use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::{
    DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY,
};
use jet_proto_proc_macros::assert_size;
use solana_maths::{Decimal, TryMul};
use std::cmp::Ordering;
use strum::IntoEnumIterator;
use type_layout::TypeLayout;

use crate::errors::ErrorCode;
use crate::impl_provider_index;
use crate::instructions::VaultConfigArg;
use crate::rebalance::assets::Provider;
use crate::rebalance::strategies::StrategyWeights;

/// Number of slots per year
pub const SLOTS_PER_YEAR: u64 =
    DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;

pub const ONE_AS_BPS: u64 = 10000;

#[assert_size(768)]
#[account]
#[repr(C, align(8))]
#[derive(Debug, TypeLayout)]
pub struct Vault {
    /// Program version when initialized: [major, minor, patch]
    pub version: [u8; 3],

    /// Account which is allowed to call restricted instructions
    /// Also the authority of the fee receiver account
    pub owner: Pubkey,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: Pubkey,

    pub authority_seed: Pubkey,

    pub authority_bump: [u8; 1],

    pub solend_reserve: Pubkey,

    pub port_reserve: Pubkey,

    pub jet_reserve: Pubkey,

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

    pub fee_receiver: Pubkey,

    pub referral_fee_receiver: Pubkey,

    bitflags: u32,

    /// Total value of vault denominated in the reserve token
    pub value: SlotTrackedValue,

    /// Prospective allocations set by rebalance, executed by reconciles
    pub allocations: Allocations,

    pub config: VaultConfig,

    // 8 * 23 = 184
    /// Reserved space for future upgrades
    _reserved: [u64; 23],
}

impl Vault {
    pub fn flags(&self) -> VaultFlags {
        VaultFlags::from_bits(self.bitflags)
            .unwrap_or_else(|| panic!("{:?} does not resolve to valid VaultFlags", self.bitflags))
    }

    pub fn set_flags(&mut self, bits: u32) -> ProgramResult {
        VaultFlags::from_bits(bits).ok_or::<ProgramError>(ErrorCode::InvalidVaultFlags.into())?;
        self.bitflags = bits;
        Ok(())
    }

    pub fn calculate_fees(&self, new_vault_value: u64, slot: u64) -> Result<u64, ProgramError> {
        let vault_value_diff = new_vault_value.saturating_sub(self.value.value);
        let slots_elapsed = self.value.last_update.slots_elapsed(slot)?;

        let carry = vault_value_diff
            .checked_mul(self.config.fee_carry_bps as u64)
            .ok_or(ErrorCode::OverflowError)?
            / ONE_AS_BPS;

        let mgmt = [self.config.fee_mgmt_bps as u64, slots_elapsed]
            .iter()
            .try_fold(new_vault_value, |acc, r| acc.checked_mul(*r))
            .ok_or(ErrorCode::OverflowError)?
            / ONE_AS_BPS
            / SLOTS_PER_YEAR;

        #[cfg(feature = "debug")]
        {
            msg!("Slots elapsed: {}", slots_elapsed);
            msg!("New vault value: {}", new_vault_value);
            msg!("Old vault value: {}", self.value.value);
            msg!("Carry fee: {}", carry);
            msg!("Mgmt fee: {}", mgmt);
        }

        let fees = carry.checked_add(mgmt).ok_or(ErrorCode::OverflowError)?;
        Ok(fees)
    }

    pub fn authority_seeds(&self) -> [&[u8]; 3] {
        [
            self.authority_seed.as_ref(),
            b"authority".as_ref(),
            &self.authority_bump,
        ]
    }
}

#[assert_size(aligns, 32)]
#[repr(C, align(8))]
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug, TypeLayout)]
pub struct VaultConfig {
    /// Max num of reserve tokens. If total_value grows higher than this, will stop accepting deposits.
    pub deposit_cap: u64,

    /// Basis points of the accrued interest that gets sent to the fee_receiver
    pub fee_carry_bps: u32,

    /// Basis points of the AUM that gets sent to the fee_receiver
    pub fee_mgmt_bps: u32,

    /// Referral fee share for fee splitting
    pub referral_fee_pct: u8,

    /// Max percentage to allocate to each pool
    pub allocation_cap_pct: u8,

    /// Whether to run rebalance as a proof check or a calculation
    pub rebalance_mode: RebalanceMode,

    /// Strategy type that is executed during rebalance
    pub strategy_type: StrategyType,

    // 4 * 3 = 12
    _padding: [u32; 3],
}

impl VaultConfig {
    pub fn new(config: VaultConfigArg) -> Result<Self, ProgramError> {
        // Fee cannot be over 100%
        if config.fee_carry_bps > 10000 {
            return Err(ErrorCode::InvalidFeeConfig.into());
        }

        // Fee cannot be over 100%
        if config.fee_mgmt_bps > 10000 {
            return Err(ErrorCode::InvalidFeeConfig.into());
        }

        // Referral percentage cannot be over 50%
        if config.referral_fee_pct > 50 {
            return Err(ErrorCode::InvalidReferralFeeConfig.into());
        }

        // compute the lower limit of the cap using number of yield sources
        // TODO get this from MAX const in Chris's changes
        if !(34..=100).contains(&config.allocation_cap_pct) {
            return Err(ErrorCode::InvalidAllocationCap.into());
        }

        Ok(Self {
            deposit_cap: config.deposit_cap,
            fee_carry_bps: config.fee_carry_bps,
            fee_mgmt_bps: config.fee_mgmt_bps,
            referral_fee_pct: config.referral_fee_pct,
            allocation_cap_pct: config.allocation_cap_pct,
            rebalance_mode: config.rebalance_mode,
            strategy_type: config.strategy_type,
            _padding: [0; 3],
        })
    }
}

#[repr(u8)]
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub enum RebalanceMode {
    Calculator,
    ProofChecker,
}

#[repr(u8)]
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub enum StrategyType {
    MaxYield,
    EqualAllocation,
}

bitflags::bitflags! {
    pub struct VaultFlags: u32 {
        /// Disable reconciles
        const HALT_RECONCILES = 1 << 0;

        /// Disable refreshes
        const HALT_REFRESHES = 1 << 1;

        /// Disable deposits + withdrawals
        const HALT_DEPOSITS_WITHDRAWS = 1 << 2;

        /// Disable all operations
        const HALT_ALL = Self::HALT_RECONCILES.bits
                       | Self::HALT_REFRESHES.bits
                       | Self::HALT_DEPOSITS_WITHDRAWS.bits;

    }
}

#[assert_size(aligns, 72)]
#[repr(C, align(8))]
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug, Default)]
pub struct Allocations {
    pub solend: SlotTrackedValue,
    pub port: SlotTrackedValue,
    pub jet: SlotTrackedValue,
}
impl_provider_index!(Allocations, SlotTrackedValue);

impl Allocations {
    pub fn try_from_weights(
        weights: StrategyWeights,
        total_value: u64,
        slot: u64,
    ) -> Result<Self, ProgramError> {
        let mut allocations = Self::default();
        for p in Provider::iter() {
            let allocation = weights[p]
                .try_mul(total_value)
                .and_then(|product| Decimal::from(product).try_floor_u64())?;
            allocations[p].update(allocation, slot);
        }
        Ok(allocations)
    }
}

// This should be a generic, but anchor doesn't support that yet
// https://github.com/project-serum/anchor/issues/1849
#[repr(C, align(8))]
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug, Default)]
pub struct SlotTrackedValue {
    pub value: u64,
    pub last_update: LastUpdate,
}

impl SlotTrackedValue {
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
pub const STALE_AFTER_SLOTS_ELAPSED: u64 = 2;

#[assert_size(aligns, 16)]
#[repr(C, align(8))]
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug, Default)]
pub struct LastUpdate {
    pub slot: u64,
    pub stale: bool,
    _padding: [u8; 7],
}

impl LastUpdate {
    /// Create new last update
    pub fn new(slot: u64) -> Self {
        Self {
            slot,
            stale: true,
            _padding: [0_u8; 7],
        }
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

    /// Check if marked stale or last update slot is too long ago
    pub fn is_stale(&self, slot: u64) -> Result<bool, ProgramError> {
        #[cfg(feature = "debug")]
        {
            msg!("Last updated slot: {}", self.slot);
            msg!("Current slot: {}", slot);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_vault_layout() {
        println!("{}", Vault::type_layout());
        println!("{}", VaultConfig::type_layout());
    }
}
