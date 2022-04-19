use anchor_lang::prelude::*;

#[error]
pub enum ErrorCode {
    #[msg("failed to perform some math operation safely")]
    MathError,

    #[msg("Failed to run the strategy")]
    StrategyError,

    #[msg("Vault is not refreshed")]
    VaultIsNotRefreshed,

    #[msg("Allocation is not updated")]
    AllocationIsNotUpdated,

    #[msg("Failed to convert from Reserve")]
    TryFromReserveError,

    #[msg("Failed to perform a math operation without an overflow")]
    OverflowError,

    #[msg("Failed to set referral fee share which is greater than 50%")]
    ReferralFeeError,

    #[msg("Failed to set fee BPS which is greater than 10000")]
    FeeBpsError,

    #[msg("Proposed weights don't add up to 100%")]
    InvalidProposedWeights,

    #[msg("Proposed weights failed proof check")]
    RebalanceProofCheckFailed,

    #[msg("Vault size limit is reached")]
    DepositCapError,
}
