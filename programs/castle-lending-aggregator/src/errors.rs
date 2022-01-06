use anchor_lang::prelude::*;

#[error]
pub enum ErrorCode {
    #[msg("failed to perform some math operation safely")]
    MathError,

    #[msg("Vault is not refreshed")]
    VaultIsNotRefreshed,
}