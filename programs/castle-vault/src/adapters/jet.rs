use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use jet::{
    state::{CachedReserveInfo, Reserve},
    Amount, Rounding,
};
use solana_maths::Rate;

use crate::{
    errors::ErrorCode,
    impl_has_vault,
    init_yield_source::YieldSourceInitializer,
    reconcile::LendingMarket,
    refresh::Refresher,
    reserves::{Provider, ReserveAccessor},
    state::{Vault, YieldSourceFlags},
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
    fn deposit(&mut self, amount: u64) -> ProgramResult {
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
        }?;

        let jet_value = self.vault.actual_allocations[Provider::Jet]
            .value
            .checked_add(amount)
            .ok_or(ErrorCode::MathError)?;
        self.vault.actual_allocations[Provider::Jet].update(jet_value, Clock::get()?.slot);
        Ok(())
    }

    fn redeem(&mut self, amount: u64) -> ProgramResult {
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
        }?;

        let vault_reserve_value_delta = self.convert_amount_lp_to_reserve(amount)?;
        let jet_value = self.vault.actual_allocations[Provider::Jet]
            .value
            .checked_sub(vault_reserve_value_delta)
            .ok_or(ErrorCode::MathError)?;
        self.vault.actual_allocations[Provider::Jet].update(jet_value, Clock::get()?.slot);
        Ok(())
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

impl ReserveAccessor for Reserve {
    fn utilization_rate(&self) -> Result<Rate, ProgramError> {
        let vault_amount = self.total_deposits();
        let outstanding_debt = *self.unwrap_outstanding_debt(Clock::get()?.slot);

        Ok(Rate::from_bips(
            jet::state::utilization_rate(outstanding_debt, vault_amount).as_u64(-4),
        ))
    }

    fn borrow_rate(&self) -> Result<Rate, ProgramError> {
        let vault_amount = self.total_deposits();
        let outstanding_debt = *self.unwrap_outstanding_debt(Clock::get()?.slot);

        Ok(Rate::from_bips(
            self.interest_rate(outstanding_debt, vault_amount)
                .as_u64(-4),
        ))
    }

    fn reserve_with_deposit(
        &self,
        new_allocation: u64,
        old_allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>, ProgramError> {
        let mut reserve = Box::new(*self);
        // We only care about the token amount here
        reserve.deposit(new_allocation, 0);
        reserve.withdraw(old_allocation, 0);
        Ok(reserve)
    }
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializeJet<'info> {
    #[account(
        mut,
        has_one = owner,
        has_one = vault_authority,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's jet lp tokens
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), jet_lp_token_mint.key().as_ref()],
        bump = bump,
        token::authority = vault_authority,
        token::mint = jet_lp_token_mint,
    )]
    pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,

    /// Mint of the jet lp token
    pub jet_lp_token_mint: AccountInfo<'info>,

    pub jet_reserve: AccountLoader<'info, jet::state::Reserve>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}

impl<'info> YieldSourceInitializer<'info> for InitializeJet<'info> {
    fn initialize_yield_source(&mut self) -> ProgramResult {
        self.vault.jet_reserve = self.jet_reserve.key();
        self.vault.vault_jet_lp_token = self.vault_jet_lp_token.key();

        let mut new_flags = self.vault.get_yield_source_flags();
        new_flags.set(YieldSourceFlags::JET, true);
        self.vault.set_yield_source_flags(new_flags.bits())
    }
}

#[derive(Accounts)]
pub struct RefreshJet<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_jet_lp_token,
        has_one = jet_reserve,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Token account for the vault's jet lp tokens
    pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,

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

    pub token_program: Program<'info, Token>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info> RefreshJet<'info> {
    fn jet_refresh_reserve_context(
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
}

impl<'info> Refresher<'info> for RefreshJet<'info> {
    fn update_actual_allocation(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
    ) -> ProgramResult {
        #[cfg(feature = "debug")]
        msg!("Refreshing jet");

        jet::cpi::refresh_reserve(self.jet_refresh_reserve_context())?;

        let jet_reserve = self.jet_reserve.load()?;
        let jet_exchange_rate = jet_reserve.deposit_note_exchange_rate(
            self.clock.slot,
            jet_reserve.total_deposits(),
            jet_reserve.total_deposit_notes(),
        );
        let jet_value = (jet_exchange_rate * self.vault_jet_lp_token.amount).as_u64(0);

        #[cfg(feature = "debug")]
        msg!("Value: {}", jet_value);

        self.vault.actual_allocations[Provider::Jet].update(jet_value, self.clock.slot);

        Ok(())
    }
}
