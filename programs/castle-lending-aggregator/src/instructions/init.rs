use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, AssociatedToken, Create},
    token::{Mint, Token, TokenAccount},
};
use port_anchor_adaptor::PortReserve;

use std::convert::Into;

use crate::{adapters::SolendReserve, errors::ErrorCode, state::*};

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct InitBumpSeeds {
    authority: u8,
    reserve: u8,
    lp_mint: u8,
    solend_lp: u8,
    port_lp: u8,
    jet_lp: u8,
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct FeeArgs {
    pub fee_carry_bps: u32,
    pub fee_mgmt_bps: u32,
    pub referral_fee_pct: u8,
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

    /// Token account for the vault's solend lp tokens
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), solend_lp_token_mint.key().as_ref()],
        bump = bumps.solend_lp,
        token::authority = vault_authority,
        token::mint = solend_lp_token_mint,
    )]
    pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's port lp tokens
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), port_lp_token_mint.key().as_ref()],
        bump = bumps.port_lp,
        token::authority = vault_authority,
        token::mint = port_lp_token_mint,
    )]
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's jet lp tokens
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), jet_lp_token_mint.key().as_ref()],
        bump = bumps.jet_lp,
        token::authority = vault_authority,
        token::mint = jet_lp_token_mint,
    )]
    pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,

    /// Mint of the token that the vault accepts and stores
    pub reserve_token_mint: Box<Account<'info, Mint>>,

    /// Mint of the solend lp token
    pub solend_lp_token_mint: AccountInfo<'info>,

    /// Mint of the port lp token
    pub port_lp_token_mint: AccountInfo<'info>,

    /// Mint of the jet lp token
    pub jet_lp_token_mint: AccountInfo<'info>,

    pub solend_reserve: Box<Account<'info, SolendReserve>>,

    pub port_reserve: Box<Account<'info, PortReserve>>,

    pub jet_reserve: AccountLoader<'info, jet::state::Reserve>,

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

pub fn validate_fees(fees: &FeeArgs) -> ProgramResult {
    if fees.fee_carry_bps > 10000 {
        return Err(ErrorCode::FeeBpsError.into());
    }

    if fees.fee_mgmt_bps > 10000 {
        return Err(ErrorCode::FeeBpsError.into());
    }

    if fees.referral_fee_pct > 50 {
        return Err(ErrorCode::ReferralFeeError.into());
    }

    Ok(())
}

/// Creates a new vault
///
/// # Arguments
///
/// * `bumps` - bump seeds for creating PDAs
/// * `strategy_type` - type of strategy that rebalance will execute
/// * `fees` - carry and management fee that the vault collects denominated in basis points on behalf of primary and supplementary fee receivers
pub fn handler(
    ctx: Context<Initialize>,
    bumps: InitBumpSeeds,
    strategy_type: StrategyType,
    rebalance_mode: RebalanceMode,
    fees: FeeArgs,
    vault_deposit_cap: Option<u64>,
    allocation_cap_pct: Option<u8>,
) -> ProgramResult {
    let clock = Clock::get()?;

    // Validating referral token address
    ctx.accounts.validate_referral_token()?;

    // Validating referral token account's mint
    validate_fees(&fees)?;

    let vault = &mut ctx.accounts.vault;
    vault.vault_authority = ctx.accounts.vault_authority.key();
    vault.owner = ctx.accounts.owner.key();
    vault.authority_seed = vault.key();
    vault.authority_bump = [bumps.authority];
    vault.solend_reserve = ctx.accounts.solend_reserve.key();
    vault.port_reserve = ctx.accounts.port_reserve.key();
    vault.jet_reserve = ctx.accounts.jet_reserve.key();
    vault.vault_reserve_token = ctx.accounts.vault_reserve_token.key();
    vault.vault_solend_lp_token = ctx.accounts.vault_solend_lp_token.key();
    vault.vault_port_lp_token = ctx.accounts.vault_port_lp_token.key();
    vault.vault_jet_lp_token = ctx.accounts.vault_jet_lp_token.key();
    vault.lp_token_mint = ctx.accounts.lp_token_mint.key();
    vault.reserve_token_mint = ctx.accounts.reserve_token_mint.key();
    vault.last_update = LastUpdate::new(clock.slot);
    vault.total_value = 0;
    vault.strategy_type = strategy_type;
    vault.rebalance_mode = rebalance_mode;
    vault.deposit_cap = match vault_deposit_cap {
        Some(value) => value,
        None => u64::MAX,
    };
    vault.allocation_cap_pct = match allocation_cap_pct {
        Some(value) => {
            // compute the lower limit of the cap using number of yield sources
            // TODO Get this number from Chris's branch: MAX for const generic
            if !(34..=100).contains(&value) {
                return Err(ErrorCode::AllocationCapError.into());
            }
            value
        }
        None => 100,
    };

    vault.fees = VaultFees {
        fee_receiver: ctx.accounts.fee_receiver.key(),
        referral_fee_receiver: ctx.accounts.referral_fee_receiver.key(),
        fee_carry_bps: fees.fee_carry_bps,
        fee_mgmt_bps: fees.fee_mgmt_bps,
        referral_fee_pct: fees.referral_fee_pct,
    };

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
