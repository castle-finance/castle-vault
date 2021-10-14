use anchor_lang::prelude::*;

#[account]
pub struct ReservePool {
    pub initializer_key: Pubkey,
    pub initializer_deposit_token_amount: Pubkey,
    pub initializer_receive_token_amount: Pubkey,
    pub initializer_amount: u64,

    // Bump seed used to generate PDA
    pub bump_seed: u8,

    // SPL token program
    pub token_program_id: Pubkey,

    // Account where tokens are stored
    pub token_account: Pubkey,

    // Mint address of pool LP tokens
    pub pool_mint: Pubkey,

    // Mint address of the tokens that are stored in pool
    pub token_mint: Pubkey,
}