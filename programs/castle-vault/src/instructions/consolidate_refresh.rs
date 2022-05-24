#![allow(dead_code)]
#![allow(unused_imports)]

use boolinator::Boolinator;

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use port_anchor_adaptor::{port_lending_id, PortReserve};

use crate::adapters::{solend, SolendReserve};
use crate::errors::ErrorCode;
use crate::reserves::Provider;
use crate::state::{Vault, VaultFlags};
use strum::IntoEnumIterator;

#[derive(Accounts)]
pub struct ConsolidateRefresh<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = lp_token_mint,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's reserve tokens
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Mint for the vault lp token
    #[account(mut)]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    pub token_program: Program<'info, Token>,
}

impl<'info> ConsolidateRefresh<'info> {
    /// CpiContext for collecting fees by minting new vault lp tokens
    #[cfg(feature = "fees")]
    fn mint_to_context(
        &self,
        fee_receiver: &AccountInfo<'info>,
    ) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            MintTo {
                mint: self.lp_token_mint.to_account_info(),
                to: fee_receiver.clone(),
                authority: self.vault_authority.clone(),
            },
        )
    }
}

/// updates the vault total value, and collects fees
pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, ConsolidateRefresh<'info>>) -> ProgramResult {
    #[cfg(feature = "debug")]
    msg!("Consolidate vault refreshing");

    // Check that refreshes are not halted
    (!ctx
        .accounts
        .vault
        .get_halt_flags()
        .contains(VaultFlags::HALT_REFRESHES))
    .ok_or::<ProgramError>(ErrorCode::HaltedVault.into())?;

    let clock_slot = Clock::get()?.slot;

    // Calculate new vault value
    let vault_reserve_token_amount = ctx.accounts.vault_reserve_token.amount;
    let vault_value =
        Provider::iter().try_fold(ctx.accounts.vault_reserve_token.amount, |acc, p| {
            let allocation = ctx.accounts.vault.actual_allocations[p];
            if ctx.accounts.vault.get_yield_source_availability(p) {
                (allocation.last_update.slots_elapsed(clock_slot)? == 0)
                    .as_result::<u64, ProgramError>(
                        acc.checked_add(allocation.value)
                            .ok_or(ErrorCode::OverflowError)?,
                        ErrorCode::AllocationIsNotUpdated.into(),
                    )
            } else {
                Ok(acc)
            }
        })?;

    #[cfg(feature = "debug")]
    {
        msg!("Tokens value: {}", vault_reserve_token_amount);
        msg!("Vault value: {}", vault_value);
    }

    #[cfg(not(feature = "fees"))]
    if ctx.accounts.vault.config.fee_carry_bps > 0 || ctx.accounts.vault.config.fee_mgmt_bps > 0 {
        msg!("WARNING: Fees are non-zero but the fee feature is deactivated");
    }

    #[cfg(feature = "fees")]
    {
        let vault = &ctx.accounts.vault;

        // Calculate fees
        let total_fees = vault.calculate_fees(vault_value, clock_slot)?;

        let total_fees_converted = crate::math::calc_reserve_to_lp(
            total_fees,
            ctx.accounts.lp_token_mint.supply,
            vault_value,
        )
        .ok_or(ErrorCode::MathError)?;

        #[cfg(feature = "debug")]
        msg!(
            "Total fees: {} reserve tokens, {} lp tokens",
            total_fees,
            total_fees_converted
        );

        let primary_fees_converted = total_fees_converted
            .checked_mul(100 - ctx.accounts.vault.config.referral_fee_pct as u64)
            .and_then(|val| val.checked_div(100))
            .ok_or(ErrorCode::MathError)?;

        let referral_fees_converted = total_fees_converted
            .checked_mul(ctx.accounts.vault.config.referral_fee_pct as u64)
            .and_then(|val| val.checked_div(100))
            .ok_or(ErrorCode::MathError)?;

        #[cfg(feature = "debug")]
        msg!(
            "Collecting primary fees: {} lp tokens",
            primary_fees_converted
        );

        if ctx.remaining_accounts.len() < 2 {
            msg!("Not enough accounts passed in to collect fees");
            return Err(ErrorCode::InsufficientAccounts.into());
        }

        let primary_fee_receiver = &ctx.remaining_accounts[0];
        if primary_fee_receiver.key() != ctx.accounts.vault.fee_receiver {
            msg!("Fee receivers do not match");
            return Err(ErrorCode::InvalidAccount.into());
        }

        token::mint_to(
            ctx.accounts
                .mint_to_context(primary_fee_receiver)
                .with_signer(&[&vault.authority_seeds()]),
            primary_fees_converted,
        )?;

        #[cfg(feature = "debug")]
        msg!(
            "Collecting referral fees: {} lp tokens",
            referral_fees_converted
        );

        let referral_fee_receiver = &ctx.remaining_accounts[1];
        if referral_fee_receiver.key() != ctx.accounts.vault.referral_fee_receiver {
            msg!("Referral fee receivers do not match");
            return Err(ErrorCode::InvalidAccount.into());
        }

        token::mint_to(
            ctx.accounts
                .mint_to_context(referral_fee_receiver)
                .with_signer(&[&vault.authority_seeds()]),
            referral_fees_converted,
        )?;
    }

    // Update vault total value
    ctx.accounts.vault.value.update(vault_value, clock_slot);

    Ok(())
}
