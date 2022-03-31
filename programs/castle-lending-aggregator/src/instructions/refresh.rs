use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use port_anchor_adaptor::{get_lending_program_id, Cluster, PortReserve};

use crate::adapters::{solend, SolendReserve};
use crate::errors::ErrorCode;
use crate::state::Vault;

// NOTE: having all accounts for each lending market reserve here is not scalable
// since eventually we will hit into transaction size limits
#[derive(Accounts)]
pub struct Refresh<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        constraint = vault.fees.fee_receiver.eq(&fee_receiver.key()) @ ErrorCode::InvalidFeeReceiver,
        constraint = vault.fees.referral_fee_receiver.eq(&referral_fee_receiver.key()) @ ErrorCode::InvalidReferralFeeReceiver,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
        has_one = vault_port_lp_token,
        has_one = vault_jet_lp_token,
        has_one = lp_token_mint,
        has_one = solend_reserve,
        has_one = port_reserve,
        has_one = jet_reserve,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's reserve tokens
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's solend lp tokens
    pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's port lp tokens
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's jet lp tokens
    pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,

    /// Mint for the vault lp token
    #[account(mut)]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    #[account(
        executable,
        address = spl_token_lending::ID,
    )]
    pub solend_program: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve: Box<Account<'info, SolendReserve>>,

    //#[soteria(ignore)]
    pub solend_pyth: AccountInfo<'info>,

    //#[soteria(ignore)]
    pub solend_switchboard: AccountInfo<'info>,

    // NOTE address check is commented out because port has a different
    // ID in devnet than they do in mainnet
    #[account(
        executable,
        address = get_lending_program_id(Cluster::Devnet)
    )]
    pub port_program: AccountInfo<'info>,

    #[account(mut)]
    pub port_reserve: Box<Account<'info, PortReserve>>,

    //#[soteria(ignore)]
    pub port_oracle: AccountInfo<'info>,

    #[account(
        executable,
        address = jet::ID,
    )]
    pub jet_program: AccountInfo<'info>,

    #[account(mut)]
    //#[soteria(ignore)]
    pub jet_market: AccountInfo<'info>,

    //#[soteria(ignore)]
    pub jet_market_authority: AccountInfo<'info>,

    #[account(mut)]
    pub jet_reserve: AccountLoader<'info, jet::state::Reserve>,

    #[account(mut)]
    //#[soteria(ignore)]
    pub jet_fee_note_vault: AccountInfo<'info>,

    #[account(mut)]
    //#[soteria(ignore)]
    pub jet_deposit_note_mint: AccountInfo<'info>,

    //#[soteria(ignore)]
    pub jet_pyth: AccountInfo<'info>,

    /// Token account that collects primary fees from the vault
    /// denominated in vault lp tokens
    #[account(mut)]
    pub fee_receiver: Box<Account<'info, TokenAccount>>,

    /// Token account that collects referral fees from the vault
    /// denominated in vault lp tokens
    #[account(mut)]
    pub referral_fee_receiver: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,

    pub clock: Sysvar<'info, Clock>,
}

// TODO refactor refresh cpi calls into adapter pattern
impl<'info> Refresh<'info> {
    /// CpiContext for refreshing solend reserve
    pub fn solend_refresh_reserve_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::RefreshReserve<'info>> {
        CpiContext::new(
            self.solend_program.clone(),
            solend::RefreshReserve {
                lending_program: self.solend_program.clone(),
                reserve: self.solend_reserve.to_account_info(),
                pyth_reserve_liquidity_oracle: self.solend_pyth.clone(),
                switchboard_reserve_liquidity_oracle: self.solend_switchboard.clone(),
                clock: self.clock.to_account_info(),
            },
        )
    }

    /// CpiContext for refreshing port reserve
    pub fn port_refresh_reserve_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, port_anchor_adaptor::RefreshReserve<'info>> {
        CpiContext::new(
            self.port_program.clone(),
            port_anchor_adaptor::RefreshReserve {
                reserve: self.port_reserve.to_account_info(),
                clock: self.clock.to_account_info(),
            },
        )
        .with_remaining_accounts(vec![self.port_oracle.clone()])
    }

    /// CpiContext for refreshing jet reserve
    pub fn jet_refresh_reserve_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, jet::cpi::accounts::RefreshReserve<'info>> {
        CpiContext::new(
            self.jet_program.clone(),
            jet::cpi::accounts::RefreshReserve {
                market: self.jet_market.clone(),
                market_authority: self.jet_market_authority.clone(),
                reserve: self.jet_reserve.to_account_info(),
                fee_note_vault: self.jet_fee_note_vault.clone(),
                deposit_note_mint: self.jet_deposit_note_mint.clone(),
                pyth_oracle_price: self.jet_pyth.clone(),
                token_program: self.token_program.to_account_info(),
            },
        )
    }

    /// CpiContext for collecting fees by minting new vault lp tokens
    fn mint_to_context(
        &self,
        fee_receiver: &Account<'info, TokenAccount>,
    ) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            MintTo {
                mint: self.lp_token_mint.to_account_info(),
                to: fee_receiver.to_account_info(),
                authority: self.vault_authority.clone(),
            },
        )
    }
}

/// Refreshes the reserves of downstream lending markets,
/// updates the vault total value, and collects fees
pub fn handler(ctx: Context<Refresh>) -> ProgramResult {
    msg!("Refreshing");

    // Refresh lending market reserves
    solend::refresh_reserve(ctx.accounts.solend_refresh_reserve_context())?;
    port_anchor_adaptor::refresh_port_reserve(
        ctx.accounts.port_refresh_reserve_context(),
        port_anchor_adaptor::Cluster::Devnet,
    )?;
    jet::cpi::refresh_reserve(ctx.accounts.jet_refresh_reserve_context())?;

    // Calculate value of solend position
    let solend_exchange_rate = ctx.accounts.solend_reserve.collateral_exchange_rate()?;
    let solend_value =
        solend_exchange_rate.collateral_to_liquidity(ctx.accounts.vault_solend_lp_token.amount)?;
    // Calculate value of port position
    let port_exchange_rate = ctx.accounts.port_reserve.collateral_exchange_rate()?;
    let port_value =
        port_exchange_rate.collateral_to_liquidity(ctx.accounts.vault_port_lp_token.amount)?;
    // Calculate value of jet position
    let jet_reserve = ctx.accounts.jet_reserve.load()?;
    let jet_exchange_rate = jet_reserve.deposit_note_exchange_rate(
        ctx.accounts.clock.slot,
        jet_reserve.total_deposits(),
        jet_reserve.total_deposit_notes(),
    );
    let jet_value = jet_exchange_rate
        .as_u64(0)
        .checked_mul(ctx.accounts.vault_jet_lp_token.amount)
        .ok_or(ErrorCode::OverflowError)?;

    // Calculate new vault value
    let vault_reserve_token_amount = ctx.accounts.vault_reserve_token.amount;
    let vault_value = [solend_value, port_value, jet_value]
        .iter()
        .try_fold(vault_reserve_token_amount, |acc, &x| acc.checked_add(x))
        .ok_or(ErrorCode::OverflowError)?;

    msg!("Tokens value: {}", vault_reserve_token_amount);
    msg!("Solend value: {}", solend_value);
    msg!("Port value: {}", port_value);
    msg!("Jet value: {}", jet_value);
    msg!("Vault value: {}", vault_value);

    let vault = &ctx.accounts.vault;

    // Calculate fees
    let total_fees = vault.calculate_fees(vault_value, ctx.accounts.clock.slot)?;

    let total_fees_converted =
        crate::math::calc_reserve_to_lp(total_fees, ctx.accounts.lp_token_mint.supply, vault_value)
            .ok_or(ErrorCode::MathError)?;

    msg!(
        "Total fees: {} reserve tokens, {} lp tokens",
        total_fees,
        total_fees_converted
    );

    let primary_fees_converted = total_fees_converted
        .checked_mul(100 - ctx.accounts.vault.fees.referral_fee_pct as u64)
        .and_then(|val| val.checked_div(100))
        .ok_or(ErrorCode::MathError)?;

    let referral_fees_converted = total_fees_converted
        .checked_mul(ctx.accounts.vault.fees.referral_fee_pct as u64)
        .and_then(|val| val.checked_div(100))
        .ok_or(ErrorCode::MathError)?;

    // Mint new LP tokens to fee_receiver
    msg!(
        "Collecting primary fees: {} lp tokens",
        primary_fees_converted
    );

    token::mint_to(
        ctx.accounts
            .mint_to_context(&ctx.accounts.fee_receiver)
            .with_signer(&[&vault.authority_seeds()]),
        primary_fees_converted,
    )?;

    // Mint new LP tokens to referral_fee_receiver
    msg!(
        "Collecting referral fees: {} lp tokens",
        referral_fees_converted
    );

    token::mint_to(
        ctx.accounts
            .mint_to_context(&ctx.accounts.referral_fee_receiver)
            .with_signer(&[&vault.authority_seeds()]),
        referral_fees_converted,
    )?;

    // Update vault total value
    ctx.accounts
        .vault
        .update_value(vault_value, ctx.accounts.clock.slot);

    Ok(())
}
