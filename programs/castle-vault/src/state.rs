use std::cmp::Ordering;

use core::convert::TryFrom;

use strum::IntoEnumIterator;
#[cfg(test)]
use type_layout::TypeLayout;

use anchor_lang::prelude::*;
use jet_proto_proc_macros::assert_size;

use crate::{
    asset_container::AssetContainer,
    errors::ErrorCode,
    impl_provider_index,
    instructions::VaultConfigArg,
    math::{calc_carry_fees, calc_mgmt_fees},
    reserves::Provider,
};

#[assert_size(768)]
#[account]
#[repr(C, align(8))]
#[derive(Debug)]
#[cfg_attr(test, derive(TypeLayout))]
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

    halt_flags: u16,
    yield_source_flags: u16,

    /// Total value of vault denominated in the reserve token
    pub value: SlotTrackedValue,

    /// Prospective allocations set by rebalance, executed by reconciles
    pub target_allocations: Allocations,

    pub config: VaultConfig,

    // Actual allocation retrieved by refresh
    pub actual_allocations: Allocations,

    // Supply of vault LP token
    pub lp_token_supply: u64,

    pub vault_port_additional_state_bump: u8,

    // /// Port staking account
    // pub vault_port_stake_account: Pubkey,

    // /// Account used to receive Port staking reward
    // pub vault_port_reward_token: Pubkey,

    // /// Account used for the port staking program
    // pub vault_port_obligation: Pubkey,
    /// Reserved space for future upgrades
    _reserved0: [u8; 3],
    _reserved1: [u32; 25],
}

impl Vault {
    pub fn get_halt_flags(&self) -> VaultFlags {
        VaultFlags::from_bits(self.halt_flags)
            .unwrap_or_else(|| panic!("{:?} does not resolve to valid VaultFlags", self.halt_flags))
    }

    pub fn set_halt_flags(&mut self, bits: u16) -> ProgramResult {
        VaultFlags::from_bits(bits)
            .ok_or_else::<ProgramError, _>(|| ErrorCode::InvalidVaultFlags.into())?;
        self.halt_flags = bits;
        Ok(())
    }

    pub fn get_yield_source_flags(&self) -> YieldSourceFlags {
        YieldSourceFlags::from_bits(self.yield_source_flags).unwrap_or_else(|| {
            panic!(
                "{:?} does not resolve to valid YieldSourceFlags",
                self.yield_source_flags
            )
        })
    }

    pub fn set_yield_source_flags(&mut self, flags: u16) -> ProgramResult {
        YieldSourceFlags::from_bits(flags)
            .ok_or_else::<ProgramError, _>(|| ErrorCode::InvalidVaultFlags.into())?;
        self.yield_source_flags = flags;
        Ok(())
    }

    // The lower bound of allocation cap is adjusted to 100 / N
    // Where N is the number of available yield sources according to yield_source_flags
    pub fn adjust_allocation_cap(&mut self) -> ProgramResult {
        let cnt: u8 =
            u8::try_from((0..16).fold(0, |sum, i| sum + ((self.yield_source_flags >> i) & 1)))
                .map_err::<ProgramError, _>(|_| ErrorCode::MathError.into())?;
        let new_allocation_cap = 100_u8
            .checked_div(cnt)
            .ok_or_else::<ProgramError, _>(|| ErrorCode::MathError.into())?
            .checked_add(1)
            .ok_or_else::<ProgramError, _>(|| ErrorCode::MathError.into())?
            .clamp(0, 100);
        self.config.allocation_cap_pct = self
            .config
            .allocation_cap_pct
            .clamp(new_allocation_cap, 100);

        #[cfg(feature = "debug")]
        {
            msg!("num of active pools: {}", cnt);
            msg!(" new allocation cap: {}", self.config.allocation_cap_pct);
        }

        Ok(())
    }

    pub fn get_yield_source_availability(&self, provider: Provider) -> bool {
        let flags = self.get_yield_source_flags();
        match provider {
            Provider::Solend => flags.contains(YieldSourceFlags::SOLEND),
            Provider::Port => flags.contains(YieldSourceFlags::PORT),
            Provider::Jet => flags.contains(YieldSourceFlags::JET),
        }
    }

    pub fn calculate_fees(&self, new_vault_value: u64, slot: u64) -> Result<u64, ProgramError> {
        let vault_value_diff = new_vault_value.saturating_sub(self.value.value);
        let slots_elapsed = self.value.last_update.slots_elapsed(slot)?;

        let carry = calc_carry_fees(vault_value_diff, self.config.fee_carry_bps as u64)?;
        let mgmt = calc_mgmt_fees(
            new_vault_value,
            self.config.fee_mgmt_bps as u64,
            slots_elapsed,
        )?;

        #[cfg(feature = "debug")]
        {
            msg!("Slots elapsed: {}", slots_elapsed);
            msg!("New vault value: {}", new_vault_value);
            msg!("Old vault value: {}", self.value.value);
            msg!("Carry fee: {}", carry);
            msg!("Mgmt fee: {}", mgmt);
        }

        carry
            .checked_add(mgmt)
            .ok_or_else(|| ErrorCode::OverflowError.into())
    }

    pub fn authority_seeds(&self) -> [&[u8]; 3] {
        [
            self.authority_seed.as_ref(),
            b"authority".as_ref(),
            &self.authority_bump,
        ]
    }
}

#[assert_size(504)]
#[account]
#[repr(C, align(8))]
#[derive(Debug, Default)]
#[cfg_attr(test, derive(TypeLayout))]
pub struct VaultPortAdditionalState {
    /// Port staking account
    pub vault_port_stake_account_bump: u8,

    /// Account used to receive Port staking reward
    pub vault_port_reward_token_bump: u8,

    /// Account used for the port staking program
    pub vault_port_obligation_bump: u8,

    _reserved0: [u8; 5],
    _reserved1: [u64; 30],
    _reserved2: [u64; 32],
}

#[assert_size(aligns, 32)]
#[repr(C, align(8))]
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
#[cfg_attr(test, derive(TypeLayout))]
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
    pub struct VaultFlags: u16 {
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

bitflags::bitflags! {
    pub struct YieldSourceFlags: u16 {
        const SOLEND = 1 << 0;
        const PORT = 1 << 1;
        const JET = 1 << 2;
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
    pub fn from_container(c: AssetContainer<u64>, slot: u64) -> Self {
        Provider::iter().fold(Self::default(), |mut acc, provider| {
            match c[provider] {
                Some(v) => acc[provider].update(v, slot),
                None => {}
            };
            acc
        })
    }

    pub fn to_container(&self, flags: YieldSourceFlags) -> AssetContainer<u64> {
        let mut retval = AssetContainer::<u64>::default();
        Provider::iter().for_each(|p| {
            retval[p] = flags
                .contains(match p {
                    Provider::Solend => YieldSourceFlags::SOLEND,
                    Provider::Port => YieldSourceFlags::PORT,
                    Provider::Jet => YieldSourceFlags::JET,
                })
                .then(|| self[p].value);
        });
        retval
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
pub const STALE_AFTER_SLOTS_ELAPSED: u64 = 1;

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
        slot.checked_sub(self.slot)
            .ok_or_else(|| ErrorCode::MathError.into())
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
