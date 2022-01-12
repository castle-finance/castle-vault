use anchor_lang::prelude::*;

// TODO is there a way to add another message as a parameter?
#[error]
pub enum ErrorCode {
    #[msg("failed to perform some math operation safely")]
    MathError,

    #[msg("Vault is not refreshed")]
    VaultIsNotRefreshed,
}
