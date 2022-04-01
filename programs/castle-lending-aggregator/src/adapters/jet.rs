use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use jet::{state::CachedReserveInfo, Amount, Rounding};

use crate::{
    impl_has_vault,
    reconcile::LendingMarket,
    state::{Provider, Vault},
};

#[derive(Accounts)]
pub struct JetAccounts<'info> {
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

    //#[soteria(ignore)]
    pub jet_market: AccountLoader<'info, jet::state::Market>,

    //#[soteria(ignore)]
    pub jet_market_authority: AccountInfo<'info>,

    #[account(mut)]
    pub jet_reserve: AccountLoader<'info, jet::state::Reserve>,

    #[account(mut)]
    //#[soteria(ignore)]
    pub jet_reserve_token: AccountInfo<'info>,

    #[account(mut)]
    //#[soteria(ignore)]
    pub jet_lp_mint: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> JetAccounts<'info> {
    fn get_reserve_info(&self) -> Result<CachedReserveInfo, ProgramError> {
        let market = self.jet_market.load()?;
        let reserve = self.jet_reserve.load()?;
        let clock = Clock::get()?;
        Ok(*market.reserves().get_cached(reserve.index, clock.slot))
    }
}

impl_has_vault!(JetAccounts<'_>);

impl<'info> LendingMarket for JetAccounts<'info> {
    fn deposit(&self, amount: u64) -> ProgramResult {
        let context = CpiContext::new(
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
        );

        match amount {
            0 => Ok(()),
            _ => jet::cpi::deposit_tokens(
                context.with_signer(&[&self.vault.authority_seeds()]),
                Amount::from_tokens(amount),
            ),
        }
    }

    fn redeem(&self, amount: u64) -> ProgramResult {
        let context = CpiContext::new(
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
        );
        match amount {
            0 => Ok(()),
            _ => jet::cpi::withdraw_tokens(
                context.with_signer(&[&self.vault.authority_seeds()]),
                Amount::from_deposit_notes(amount),
            ),
        }
    }

    fn convert_amount_reserve_to_lp(&self, amount: u64) -> Result<u64, ProgramError> {
        let reserve_info = self.get_reserve_info()?;
        Ok(Amount::from_tokens(amount).as_deposit_notes(&reserve_info, Rounding::Down)?)
    }

    fn convert_amount_lp_to_reserve(&self, amount: u64) -> Result<u64, ProgramError> {
        let reserve_info = self.get_reserve_info()?;
        Ok(Amount::from_deposit_notes(amount).as_tokens(&reserve_info, Rounding::Down))
    }

    fn reserve_tokens_in_vault(&self) -> u64 {
        self.vault_reserve_token.amount
    }

    fn lp_tokens_in_vault(&self) -> u64 {
        self.vault_jet_lp_token.amount
    }

    fn provider(&self) -> Provider {
        Provider::Jet
    }
}
