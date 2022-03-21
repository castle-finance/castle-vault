use anchor_lang::prelude::*;

// TODO is there a way to add another message as a parameter?
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
}
