use std::ops::Deref;
use std::{io::Write, ops::DerefMut};

use anchor_lang::{prelude::*, solana_program};
use anchor_spl::token::{Token, TokenAccount};
use solana_maths::Rate;
use spl_token_lending::state::Reserve;

use crate::rebalance::assets::ReserveAccessor;
use crate::{impl_has_vault, rebalance::assets::Provider, reconcile::LendingMarket, state::Vault};

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

    //#[soteria(ignore)]
    pub solend_market_authority: AccountInfo<'info>,

    //#[soteria(ignore)]
    pub solend_market: AccountInfo<'info>,

    #[account(mut)]
    pub solend_reserve: Box<Account<'info, SolendReserve>>,

    #[account(mut)]
    //#[soteria(ignore)]
    pub solend_lp_mint: AccountInfo<'info>,

    #[account(mut)]
    //#[soteria(ignore)]
    pub solend_reserve_token: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,
}

impl_has_vault!(SolendAccounts<'_>);

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

    fn provider(&self) -> Provider {
        Provider::Solend
    }
}

impl ReserveAccessor for Reserve {
    fn utilization_rate(&self) -> Result<Rate, ProgramError> {
        Ok(Rate::from_scaled_val(
            self.liquidity.utilization_rate()?.to_scaled_val() as u64,
        ))
    }

    fn borrow_rate(&self) -> Result<Rate, ProgramError> {
        Ok(Rate::from_scaled_val(
            self.current_borrow_rate()?.to_scaled_val() as u64,
        ))
    }

    fn reserve_with_deposit(
        &self,
        allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>, ProgramError> {
        let mut reserve = Box::new(self.clone());
        reserve.liquidity.deposit(allocation)?;
        Ok(reserve)
    }
}

pub fn deposit_reserve_liquidity<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, DepositReserveLiquidity<'info>>,
    liquidity_amount: u64,
) -> ProgramResult {
    let ix = spl_token_lending::instruction::deposit_reserve_liquidity(
        *ctx.accounts.lending_program.key,
        liquidity_amount,
        *ctx.accounts.source_liquidity.key,
        *ctx.accounts.destination_collateral_account.key,
        *ctx.accounts.reserve.key,
        *ctx.accounts.reserve_liquidity_supply.key,
        *ctx.accounts.reserve_collateral_mint.key,
        *ctx.accounts.lending_market.key,
        *ctx.accounts.transfer_authority.key,
    );

    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;

    Ok(())
}

pub fn redeem_reserve_collateral<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, RedeemReserveCollateral<'info>>,
    collateral_amount: u64,
) -> ProgramResult {
    let ix = spl_token_lending::instruction::redeem_reserve_collateral(
        *ctx.accounts.lending_program.key,
        collateral_amount,
        *ctx.accounts.source_collateral.key,
        *ctx.accounts.destination_liquidity.key,
        *ctx.accounts.reserve.key,
        *ctx.accounts.reserve_collateral_mint.key,
        *ctx.accounts.reserve_liquidity_supply.key,
        *ctx.accounts.lending_market.key,
        *ctx.accounts.transfer_authority.key,
    );

    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;

    Ok(())
}

pub fn refresh_reserve<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, RefreshReserve<'info>>,
) -> ProgramResult {
    let ix = spl_token_lending::instruction::refresh_reserve(
        *ctx.accounts.lending_program.key,
        *ctx.accounts.reserve.key,
        *ctx.accounts.pyth_reserve_liquidity_oracle.key,
        *ctx.accounts.switchboard_reserve_liquidity_oracle.key,
    );

    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct DepositReserveLiquidity<'info> {
    // Lending program
    pub lending_program: AccountInfo<'info>,
    // Token account for asset to deposit into reserve
    pub source_liquidity: AccountInfo<'info>,
    // Token account for reserve collateral token
    pub destination_collateral_account: AccountInfo<'info>,
    // Reserve state account
    pub reserve: AccountInfo<'info>,
    // Token mint for reserve collateral token
    pub reserve_collateral_mint: AccountInfo<'info>,
    // Reserve liquidity supply SPL token account
    pub reserve_liquidity_supply: AccountInfo<'info>,
    // Lending market account
    pub lending_market: AccountInfo<'info>,
    // Lending market authority (PDA)
    pub lending_market_authority: AccountInfo<'info>,
    // Transfer authority for accounts 1 and 2
    pub transfer_authority: AccountInfo<'info>,
    // Clock
    pub clock: AccountInfo<'info>,
    // Token program ID
    pub token_program_id: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RedeemReserveCollateral<'info> {
    // Lending program
    pub lending_program: AccountInfo<'info>,
    // Source token account for reserve collateral token
    pub source_collateral: AccountInfo<'info>,
    // Destination liquidity token account
    pub destination_liquidity: AccountInfo<'info>,
    // Refreshed reserve account
    pub reserve: AccountInfo<'info>,
    // Reserve collateral mint account
    pub reserve_collateral_mint: AccountInfo<'info>,
    // Reserve liquidity supply SPL Token account.
    pub reserve_liquidity_supply: AccountInfo<'info>,
    // Lending market account
    pub lending_market: AccountInfo<'info>,
    // Lending market authority - PDA
    pub lending_market_authority: AccountInfo<'info>,
    // User transfer authority
    pub transfer_authority: AccountInfo<'info>,
    // Clock
    pub clock: AccountInfo<'info>,
    // Token program ID
    pub token_program_id: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RefreshReserve<'info> {
    // Lending program
    pub lending_program: AccountInfo<'info>,
    // Reserve account
    pub reserve: AccountInfo<'info>,
    // Pyth reserve liquidity oracle
    // Must be the pyth price account specified in InitReserve
    pub pyth_reserve_liquidity_oracle: AccountInfo<'info>,
    // Switchboard Reserve liquidity oracle account
    // Must be the switchboard price account specified in InitReserve
    pub switchboard_reserve_liquidity_oracle: AccountInfo<'info>,
    // Clock
    pub clock: AccountInfo<'info>,
}

#[derive(Clone)] //, BorshSerialize, BorshDeserialize)]
pub struct SolendReserve(Reserve);

impl anchor_lang::AccountDeserialize for SolendReserve {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        SolendReserve::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        <Reserve as solana_program::program_pack::Pack>::unpack(buf).map(SolendReserve)
    }
}

impl anchor_lang::AccountSerialize for SolendReserve {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> Result<(), ProgramError> {
        // no-op
        Ok(())
    }
}

impl anchor_lang::Owner for SolendReserve {
    fn owner() -> Pubkey {
        spl_token_lending::id()
    }
}

impl Deref for SolendReserve {
    type Target = Reserve;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}