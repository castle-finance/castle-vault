use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use jet::{Amount, Rounding};

use crate::{errors::ErrorCode, state::Vault};

use std::cmp;

#[derive(Accounts)]
pub struct ReconcileJet<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_jet_lp_token,
        has_one = jet_reserve,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's reserve tokens
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's jet lp tokens
    #[account(mut)]
    pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        executable,
        address = jet::ID,
    )]
    pub jet_program: AccountInfo<'info>,

    /// The relevant market this deposit is for
    pub jet_market: AccountLoader<'info, jet::state::Market>,

    /// The market's authority account
    pub jet_market_authority: AccountInfo<'info>,

    /// The reserve being deposited into
    #[account(mut)]
    pub jet_reserve: AccountLoader<'info, jet::state::Reserve>,

    /// The reserve's vault where the deposited tokens will be transferred to
    #[account(mut)]
    pub jet_reserve_token: AccountInfo<'info>,

    /// The mint for the deposit notes
    #[account(mut)]
    pub jet_lp_mint: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> ReconcileJet<'info> {
    /// CpiContext for depositing to jet
    pub fn jet_deposit_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, jet::cpi::accounts::DepositTokens<'info>> {
        CpiContext::new(
            self.jet_program.clone(),
            jet::cpi::accounts::DepositTokens {
                market: self.jet_market.to_account_info(),
                market_authority: self.jet_market_authority.clone(),
                reserve: self.jet_reserve.to_account_info(),
                vault: self.jet_reserve_token.clone(),
                deposit_note_mint: self.jet_lp_mint.clone(),
                depositor: self.vault_authority.clone(),
                deposit_note_account: self.vault_jet_lp_token.to_account_info(),
                deposit_source: self.vault_reserve_token.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        )
    }

    /// CpiContext for withdrawing from jet
    pub fn jet_withdraw_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, jet::cpi::accounts::WithdrawTokens<'info>> {
        CpiContext::new(
            self.jet_program.clone(),
            jet::cpi::accounts::WithdrawTokens {
                market: self.jet_market.to_account_info(),
                market_authority: self.jet_market_authority.clone(),
                reserve: self.jet_reserve.to_account_info(),
                vault: self.jet_reserve_token.clone(),
                deposit_note_mint: self.jet_lp_mint.clone(),
                depositor: self.vault_authority.clone(),
                deposit_note_account: self.vault_jet_lp_token.to_account_info(),
                withdraw_account: self.vault_reserve_token.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        )
    }
}

// TODO eliminate duplication of redeem logic
/// Deposit or withdraw from jet to match the stored allocation or to process a withdrawal
pub fn handler(ctx: Context<ReconcileJet>, withdraw_option: Option<u64>) -> ProgramResult {
    msg!("Reconciling Jet");

    let vault = &ctx.accounts.vault;

    match withdraw_option {
        // Normal case where reconcile is being called after rebalance
        None => {
            let reserve_info = {
                let market = ctx.accounts.jet_market.load()?;
                let reserve = ctx.accounts.jet_reserve.load()?;
                let clock = Clock::get()?;
                *market.reserves().get_cached(reserve.index, clock.slot)
            };

            let current_jet_value =
                Amount::from_deposit_notes(ctx.accounts.vault_jet_lp_token.amount)
                    .as_tokens(&reserve_info, Rounding::Down);
            let allocation = ctx.accounts.vault.allocations.jet;

            match allocation.value.checked_sub(current_jet_value) {
                Some(tokens_to_deposit) => {
                    // Make sure that the amount deposited is not more than the vault has in reserves
                    let tokens_to_deposit_checked =
                        cmp::min(tokens_to_deposit, ctx.accounts.vault_reserve_token.amount);

                    msg!("Depositing {}", tokens_to_deposit_checked);

                    if tokens_to_deposit != 0 {
                        jet::cpi::deposit_tokens(
                            ctx.accounts
                                .jet_deposit_context()
                                .with_signer(&[&vault.authority_seeds()]),
                            Amount::from_tokens(tokens_to_deposit_checked),
                        )?;
                    }
                }
                None => {
                    let tokens_to_redeem = Amount::from_tokens(
                        current_jet_value
                            .checked_sub(allocation.value)
                            .ok_or(ErrorCode::MathError)?,
                    )
                    .as_deposit_notes(&reserve_info, Rounding::Down)?;

                    msg!("Redeeming {}", tokens_to_redeem);

                    jet::cpi::withdraw_tokens(
                        ctx.accounts
                            .jet_withdraw_context()
                            .with_signer(&[&vault.authority_seeds()]),
                        Amount::from_deposit_notes(tokens_to_redeem),
                    )?;
                }
            }
            ctx.accounts.vault.allocations.jet.reset();
        }
        // Extra case where reconcile is being called in same tx as a withdraw or by vault owner to emergency brake
        Some(withdraw_amount) => {
            // TODO check that tx is signed by owner OR there is a withdraw tx later with the withdraw_option <= withdraw_amount

            let reserve_info = {
                let market = ctx.accounts.jet_market.load()?;
                let reserve = ctx.accounts.jet_reserve.load()?;
                let clock = Clock::get()?;
                *market.reserves().get_cached(reserve.index, clock.slot)
            };

            let tokens_to_redeem = Amount::from_tokens(withdraw_amount)
                .as_deposit_notes(&reserve_info, Rounding::Down)?;

            msg!("Redeeming {}", tokens_to_redeem);

            jet::cpi::withdraw_tokens(
                ctx.accounts
                    .jet_withdraw_context()
                    .with_signer(&[&vault.authority_seeds()]),
                Amount::from_deposit_notes(tokens_to_redeem),
            )?;
        }
    }

    Ok(())
}
