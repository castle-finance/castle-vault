use anchor_lang::prelude::*;

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
}