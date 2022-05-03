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

    #[msg("Referral fee split cannot set to be over 50%")]
    InvalidReferralFeeConfig,

    #[msg("Fees cannot be set to over 100%")]
    InvalidFeeConfig,

    #[msg("Proposed weights do not meet the required constraints")]
    InvalidProposedWeights,

    #[msg("Proposed weights failed proof check")]
    RebalanceProofCheckFailed,

    #[msg("Vault size limit is reached")]
    DepositCapError,

    #[msg("Account passed in is not valid")]
    InvalidAccount,

    #[msg("Insufficient number of accounts for a given operation")]
    InsufficientAccounts,

    #[msg("Allocation cap cannot set to under 1/(number of assets) or over 100%")]
    InvalidAllocationCap,
}
