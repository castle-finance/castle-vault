use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, AssociatedToken, Create},
    token::{Mint, Token, TokenAccount},
};

use std::convert::Into;

use crate::state::*;

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct InitBumpSeeds {
    authority: u8,
    reserve: u8,
    lp_mint: u8,
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct VaultConfigArg {
    pub deposit_cap: u64,
    pub fee_carry_bps: u32,
    pub fee_mgmt_bps: u32,
    pub referral_fee_pct: u8,
    pub allocation_cap_pct: u8,
    pub rebalance_mode: RebalanceMode,
    pub strategy_type: StrategyType,
}

#[derive(Accounts)]
#[instruction(bumps: InitBumpSeeds)]
pub struct Initialize<'info> {
    /// Vault state account
    #[account(zero)]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"authority".as_ref()],
        bump = bumps.authority,
    )]
    pub vault_authority: AccountInfo<'info>,

    /// Mint for vault lp token
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"lp_mint".as_ref()],
        bump = bumps.lp_mint,
        mint::authority = vault_authority,
        mint::decimals = reserve_token_mint.decimals,
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// Token account for vault reserve tokens
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), reserve_token_mint.key().as_ref()],
        bump = bumps.reserve,
        token::authority = vault_authority,
        token::mint = reserve_token_mint,
    )]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Mint of the token that the vault accepts and stores
    pub reserve_token_mint: Box<Account<'info, Mint>>,

    /// Token account that receives the primary ratio of fees from the vault
    /// denominated in vault lp tokens
    #[account(mut)]
    pub fee_receiver: AccountInfo<'info>,

    /// Token account that receives the secondary ratio of fees from the vault
    /// denominated in vault lp tokens
    #[account(mut)]
    pub referral_fee_receiver: AccountInfo<'info>,

    /// Owner of the referral fee reciever token account
    pub referral_fee_owner: AccountInfo<'info>,

    /// Account that pays for above account inits
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Owner of the vault
    /// Only this account can call restricted instructions
    /// Acts as authority of the fee receiver account
    pub owner: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Initialize<'info> {
    fn init_fee_receiver_create_context(
        &self,
        fee_token_account: AccountInfo<'info>,
        token_authority: AccountInfo<'info>,
    ) -> CpiContext<'_, '_, '_, 'info, Create<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Create {
                payer: self.payer.to_account_info(),
                associated_token: fee_token_account,
                authority: token_authority,
                mint: self.lp_token_mint.to_account_info(),
                system_program: self.system_program.to_account_info(),
                token_program: self.token_program.to_account_info(),
                rent: self.rent.to_account_info(),
            },
        )
    }

    fn validate_referral_token(&self) -> ProgramResult {
        let referral_fee_receiver = associated_token::get_associated_token_address(
            &self.referral_fee_owner.key(),
            &self.lp_token_mint.key(),
        );

        if referral_fee_receiver.ne(&self.referral_fee_receiver.key()) {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }
}

pub fn handler(
    ctx: Context<Initialize>,
    bumps: InitBumpSeeds,
    config: VaultConfigArg,
) -> ProgramResult {
    let clock = Clock::get()?;

    // Validating referral token address
    ctx.accounts.validate_referral_token()?;

    let vault = &mut ctx.accounts.vault;
    vault.version = get_version_arr();
    vault.owner = ctx.accounts.owner.key();
    vault.vault_authority = ctx.accounts.vault_authority.key();
    vault.authority_seed = vault.key();
    vault.authority_bump = [bumps.authority];
    vault.vault_reserve_token = ctx.accounts.vault_reserve_token.key();
    vault.lp_token_mint = ctx.accounts.lp_token_mint.key();
    vault.reserve_token_mint = ctx.accounts.reserve_token_mint.key();
    vault.fee_receiver = ctx.accounts.fee_receiver.key();
    vault.referral_fee_receiver = ctx.accounts.referral_fee_receiver.key();
    vault.value = SlotTrackedValue {
        value: 0,
        last_update: LastUpdate::new(clock.slot),
    };
    vault.config = VaultConfig::new(config)?;
    vault.yield_source_flags = 0;

    // Initialize fee receiver account
    associated_token::create(ctx.accounts.init_fee_receiver_create_context(
        ctx.accounts.fee_receiver.to_account_info(),
        ctx.accounts.owner.to_account_info(),
    ))?;

    // Initialize referral fee receiver account
    associated_token::create(ctx.accounts.init_fee_receiver_create_context(
        ctx.accounts.referral_fee_receiver.to_account_info(),
        ctx.accounts.referral_fee_owner.to_account_info(),
    ))?;

    Ok(())
}

fn get_version_arr() -> [u8; 3] {
    [
        env!("CARGO_PKG_VERSION_MAJOR")
            .parse::<u8>()
            .expect("failed to parse major version"),
        env!("CARGO_PKG_VERSION_MINOR")
            .parse::<u8>()
            .expect("failed to parse minor version"),
        env!("CARGO_PKG_VERSION_PATCH")
            .parse::<u8>()
            .expect("failed to parse patch version"),
    ]
}
