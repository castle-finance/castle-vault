use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount};
use jet_proto_math::Number;
use port_anchor_adaptor::PortReserve;

use crate::cpi::solend::{self, SolendReserve};
use crate::errors::ErrorCode;
use crate::state::Vault;

#[derive(Accounts)]
pub struct Refresh<'info> {
    #[account(
        mut,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
        has_one = vault_port_lp_token,
        has_one = vault_jet_lp_token,
        has_one = fee_receiver,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,

    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    #[account(
        executable,
        address = spl_token_lending::ID,
    )]
    pub solend_program: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve_state: Box<Account<'info, SolendReserve>>,

    pub solend_pyth: AccountInfo<'info>,

    pub solend_switchboard: AccountInfo<'info>,

    #[account(
        executable,
        //address = port_variable_rate_lending_instructions::ID,
    )]
    pub port_program: AccountInfo<'info>,

    #[account(mut)]
    pub port_reserve_state: Box<Account<'info, PortReserve>>,

    pub port_oracle: AccountInfo<'info>,

    #[account(
        executable,
        address = jet::ID,
    )]
    pub jet_program: AccountInfo<'info>,

    #[account(mut)]
    pub jet_market: AccountInfo<'info>,

    pub jet_market_authority: AccountInfo<'info>,

    #[account(mut)]
    pub jet_reserve_state: AccountLoader<'info, jet::state::Reserve>,

    #[account(mut)]
    pub jet_fee_note_vault: AccountInfo<'info>,

    #[account(mut)]
    pub jet_deposit_note_mint: AccountInfo<'info>,

    pub jet_pyth: AccountInfo<'info>,

    #[account(mut)]
    pub fee_receiver: Box<Account<'info, TokenAccount>>,

    #[account(address = token::ID)]
    pub token_program: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info> Refresh<'info> {
    pub fn solend_refresh_reserve_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, solend::RefreshReserve<'info>> {
        CpiContext::new(
            self.solend_program.clone(),
            solend::RefreshReserve {
                lending_program: self.solend_program.clone(),
                reserve: self.solend_reserve_state.to_account_info(),
                pyth_reserve_liquidity_oracle: self.solend_pyth.clone(),
                switchboard_reserve_liquidity_oracle: self.solend_switchboard.clone(),
                clock: self.clock.to_account_info(),
            },
        )
    }

    pub fn port_refresh_reserve_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, port_anchor_adaptor::RefreshReserve<'info>> {
        CpiContext::new(
            self.port_program.clone(),
            port_anchor_adaptor::RefreshReserve {
                reserve: self.port_reserve_state.to_account_info(),
                clock: self.clock.to_account_info(),
            },
        )
        .with_remaining_accounts(vec![self.port_oracle.clone()])
    }

    pub fn jet_refresh_reserve_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, jet::cpi::accounts::RefreshReserve<'info>> {
        CpiContext::new(
            self.jet_program.clone(),
            jet::cpi::accounts::RefreshReserve {
                market: self.jet_market.clone(),
                market_authority: self.jet_market_authority.clone(),
                reserve: self.jet_reserve_state.to_account_info(),
                fee_note_vault: self.jet_fee_note_vault.clone(),
                deposit_note_mint: self.jet_deposit_note_mint.clone(),
                pyth_oracle_price: self.jet_pyth.clone(),
                token_program: self.token_program.clone(),
            },
        )
    }

    fn mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        CpiContext::new(
            self.token_program.clone(),
            MintTo {
                mint: self.lp_token_mint.to_account_info(),
                to: self.fee_receiver.to_account_info(),
                authority: self.vault_authority.clone(),
            },
        )
    }
}

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
    let solend_exchange_rate = ctx
        .accounts
        .solend_reserve_state
        .collateral_exchange_rate()?;
    let solend_value =
        solend_exchange_rate.collateral_to_liquidity(ctx.accounts.vault_solend_lp_token.amount)?;
    // Calculate value of port position
    let port_exchange_rate = ctx.accounts.port_reserve_state.collateral_exchange_rate()?;
    let port_value =
        port_exchange_rate.collateral_to_liquidity(ctx.accounts.vault_port_lp_token.amount)?;
    // Calculate value of jet position
    let jet_reserve = ctx.accounts.jet_reserve_state.load()?;
    let jet_exchange_rate = jet_reserve.deposit_note_exchange_rate(
        ctx.accounts.clock.slot,
        jet_reserve.total_deposits(),
        jet_reserve.total_deposit_notes(),
    );
    let jet_value =
        (jet_exchange_rate * Number::from(ctx.accounts.vault_jet_lp_token.amount)).as_u64(0);

    // Calculate new vault value
    let vault_reserve_token_amount = ctx.accounts.vault_reserve_token.amount;
    let vault_value = vault_reserve_token_amount + solend_value + port_value + jet_value;
    msg!("Tokens value: {}", vault_reserve_token_amount);
    msg!("Solend value: {}", solend_value);
    msg!("Port value: {}", port_value);
    msg!("Jet value: {}", jet_value);
    msg!("Vault value: {}", vault_value);

    let vault = &ctx.accounts.vault;

    // Calculate fees
    let fees = vault.calculate_fees(vault_value, ctx.accounts.clock.slot)?;
    let fees_converted =
        crate::math::calc_reserve_to_lp(fees, ctx.accounts.lp_token_mint.supply, vault_value)
            .ok_or(ErrorCode::MathError)?;

    // Mint new LP tokens to fee_receiver
    msg!(
        "Collecting fees: {} reserve tokens, {} lp tokens",
        fees,
        fees_converted
    );
    token::mint_to(
        ctx.accounts
            .mint_to_context()
            .with_signer(&[&vault.authority_seeds()]),
        fees_converted,
    )?;

    // Update vault total value
    ctx.accounts
        .vault
        .update_value(vault_value, ctx.accounts.clock.slot);

    Ok(())
}
