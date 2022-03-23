use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{cpi::solend_cpi::*, reconcile::LendingMarket, state::Vault};

// TODO move file to diff location?

#[derive(Accounts)]
pub struct SolendAccounts<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_solend_lp_token,
        has_one = solend_reserve,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's reserve tokens
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's solend lp tokens
    #[account(mut)]
    pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        executable,
        address = spl_token_lending::ID,
    )]
    pub solend_program: AccountInfo<'info>,

    pub solend_market_authority: AccountInfo<'info>,

    #[account(owner = solend_program.key())]
    pub solend_market: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve: Box<Account<'info, SolendReserve>>,

    #[account(mut)]
    pub solend_lp_mint: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve_token: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,
}

impl<'info> LendingMarket for SolendAccounts<'info> {
    fn deposit(&self, amount: u64) -> ProgramResult {
        let context = CpiContext::new(
            self.solend_program.clone(),
            DepositReserveLiquidity {
                lending_program: self.solend_program.clone(),
                source_liquidity: self.vault_reserve_token.to_account_info(),
                destination_collateral_account: self.vault_solend_lp_token.to_account_info(),
                reserve: self.solend_reserve.to_account_info(),
                reserve_collateral_mint: self.solend_lp_mint.clone(),
                reserve_liquidity_supply: self.solend_reserve_token.clone(),
                lending_market: self.solend_market.clone(),
                lending_market_authority: self.solend_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program_id: self.token_program.to_account_info(),
            },
        );
        match amount {
            0 => Ok(()),
            _ => deposit_reserve_liquidity(
                context.with_signer(&[&self.vault.authority_seeds()]),
                amount,
            ),
        }
    }
    fn redeem(&self, amount: u64) -> ProgramResult {
        let context = CpiContext::new(
            self.solend_program.clone(),
            RedeemReserveCollateral {
                lending_program: self.solend_program.clone(),
                source_collateral: self.vault_solend_lp_token.to_account_info(),
                destination_liquidity: self.vault_reserve_token.to_account_info(),
                reserve: self.solend_reserve.to_account_info(),
                reserve_collateral_mint: self.solend_lp_mint.clone(),
                reserve_liquidity_supply: self.solend_reserve_token.clone(),
                lending_market: self.solend_market.clone(),
                lending_market_authority: self.solend_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program_id: self.token_program.to_account_info(),
            },
        );
        match amount {
            0 => Ok(()),
            _ => redeem_reserve_collateral(
                context.with_signer(&[&self.vault.authority_seeds()]),
                amount,
            ),
        }
    }
    fn convert_amount_reserve_to_lp(&self, amount: u64) -> Result<u64, ProgramError> {
        let exchange_rate = self.solend_reserve.collateral_exchange_rate()?;
        exchange_rate.collateral_to_liquidity(amount)
    }
    fn convert_amount_lp_to_reserve(&self, amount: u64) -> Result<u64, ProgramError> {
        let exchange_rate = self.solend_reserve.collateral_exchange_rate()?;
        exchange_rate.liquidity_to_collateral(amount)
    }

    fn reserve_tokens_in_vault(&self) -> u64 {
        self.vault_reserve_token.amount
    }

    fn lp_tokens_in_vault(&self) -> u64 {
        self.vault_solend_lp_token.amount
    }

    fn get_allocation(&self) -> u64 {
        self.vault.allocations.solend.value
    }

    fn reset_allocations(&mut self) {
        self.vault.allocations.solend.reset();
    }
}
