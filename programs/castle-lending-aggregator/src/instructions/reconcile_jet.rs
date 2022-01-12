use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount};
use jet::Amount;

use crate::state::Vault;

#[derive(Accounts)]
pub struct ReconcileJet<'info> {
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        executable,
        address = jet::ID,
    )]
    pub jet_program: AccountInfo<'info>,

    /// The relevant market this deposit is for
    pub jet_market: AccountInfo<'info>,

    /// The market's authority account
    pub jet_market_authority: AccountInfo<'info>,

    /// The reserve being deposited into
    #[account(mut)]
    pub jet_reserve_state: AccountInfo<'info>,

    /// The reserve's vault where the deposited tokens will be transferred to
    #[account(mut)]
    pub jet_reserve_token: AccountInfo<'info>,

    /// The mint for the deposit notes
    #[account(mut)]
    pub jet_lp_mint: AccountInfo<'info>,

    #[account(address = token::ID)]
    pub token_program: AccountInfo<'info>,
}

impl<'info> ReconcileJet<'info> {
    pub fn jet_deposit_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, jet::cpi::accounts::DepositTokens<'info>> {
        CpiContext::new(
            self.jet_program.clone(),
            jet::cpi::accounts::DepositTokens {
                market: self.jet_market.clone(),
                market_authority: self.jet_market_authority.clone(),
                reserve: self.jet_reserve_state.clone(),
                vault: self.jet_reserve_token.clone(),
                deposit_note_mint: self.jet_lp_mint.clone(),
                depositor: self.vault_authority.clone(),
                deposit_note_account: self.vault_jet_lp_token.to_account_info(),
                deposit_source: self.vault_reserve_token.to_account_info(),
                token_program: self.token_program.clone(),
            },
        )
    }

    pub fn jet_withdraw_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, jet::cpi::accounts::WithdrawTokens<'info>> {
        CpiContext::new(
            self.jet_program.clone(),
            jet::cpi::accounts::WithdrawTokens {
                market: self.jet_market.clone(),
                market_authority: self.jet_market_authority.clone(),
                reserve: self.jet_reserve_state.clone(),
                vault: self.jet_reserve_token.clone(),
                deposit_note_mint: self.jet_lp_mint.clone(),
                depositor: self.vault_authority.clone(),
                deposit_note_account: self.vault_jet_lp_token.to_account_info(),
                withdraw_account: self.vault_reserve_token.to_account_info(),
                token_program: self.token_program.clone(),
            },
        )
    }
}

pub fn handler(ctx: Context<ReconcileJet>) -> ProgramResult {
    let vault = &ctx.accounts.vault;

    let deposit_amount = vault.to_reconcile[2].deposit;
    let redeem_amount = vault.to_reconcile[2].redeem;

    if deposit_amount > 0 {
        jet::cpi::deposit_tokens(
            ctx.accounts
                .jet_deposit_context()
                .with_signer(&[&vault.authority_seeds()]),
            Amount::from_tokens(deposit_amount),
        )?;
    }
    if redeem_amount > 0 {
        jet::cpi::withdraw_tokens(
            ctx.accounts
                .jet_withdraw_context()
                .with_signer(&[&vault.authority_seeds()]),
            Amount::from_deposit_notes(redeem_amount),
        )?;
    }

    ctx.accounts.vault.to_reconcile[2].reset();

    Ok(())
}
