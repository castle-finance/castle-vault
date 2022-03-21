use anchor_lang::prelude::*;
use anchor_spl::token::{self, InitializeAccount, Mint, Token, TokenAccount};
use port_anchor_adaptor::PortReserve;

use std::convert::Into;

use crate::{cpi::SolendReserve, state::*};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitBumpSeeds {
    authority: u8,
    reserve: u8,
    lp_mint: u8,
    fee_receiver: u8,
    solend_lp: u8,
    port_lp: u8,
    jet_lp: u8,
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

    /// Token account that collects fees from the vault
    /// denominated in vault lp tokens
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), b"fee_receiver".as_ref()],
        bump = bumps.fee_receiver,
        owner = token::ID,
        space = TokenAccount::LEN
    )]
    pub fee_receiver: AccountInfo<'info>,

    /// Account that pays for above account inits
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Owner of the vault
    /// Only this account can call restricted instructions
    /// Acts as authority of the fee receiver account
    pub owner: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Initialize<'info> {
    fn init_fee_receiver_context(&self) -> CpiContext<'_, '_, '_, 'info, InitializeAccount<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            InitializeAccount {
                account: self.fee_receiver.clone(),
                authority: self.owner.to_account_info(),
                mint: self.lp_token_mint.to_account_info(),
                rent: self.rent.to_account_info(),
            },
        )
    }
}

/// Creates a new vault
///
/// # Arguments
///
/// * `bumps` - bump seeds for creating PDAs
/// * `strategy_type` - type of strategy that rebalance will execute
/// * `fee_carry_bps` - carry fee that the vault collects denominated in basis points
/// * `fee_mgmt_bps` - management fee that the vault collects denominated in basis points
pub fn handler(
    ctx: Context<Initialize>,
    bumps: InitBumpSeeds,
    strategy_type: StrategyType,
    fee_carry_bps: u16,
    fee_mgmt_bps: u16,
) -> ProgramResult {
    let clock = Clock::get()?;

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
    vault.fee_receiver = ctx.accounts.fee_receiver.key();
    vault.fee_carry_bps = fee_carry_bps;
    vault.fee_mgmt_bps = fee_mgmt_bps;
    vault.last_update = LastUpdate::new(clock.slot);
    vault.total_value = 0;
    vault.strategy_type = strategy_type;

    // Initialize fee receiver account
    // Needs to be manually done here instead of with anchor because the mint is initialized with anchor
    token::initialize_account(ctx.accounts.init_fee_receiver_context())?;

    Ok(())
}
