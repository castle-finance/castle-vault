/// Modified from @RohanKapurDEV
/// https://github.com/RohanKapurDEV/anchor-lending
use anchor_lang::solana_program;
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_lang::{Accounts, CpiContext, ToAccountInfos};

pub fn deposit_reserve_liquidity<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, DepositReserveLiquidity<'info>>,
    liquidity_amount: u64,
) -> ProgramResult {
    let ix = spl_token_lending::instruction::deposit_reserve_liquidity(
        *ctx.accounts.lending_program.key,
        liquidity_amount,
        *ctx.accounts.source_liquidity.key,
        *ctx.accounts.destination_collateral_account.key,
        *ctx.accounts.reserve_account.key,
        *ctx.accounts.reserve_liquidity_supply.key,
        *ctx.accounts.reserve_collateral_mint.key,
        *ctx.accounts.lending_market_account.key,
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
        *ctx.accounts.refreshed_reserve_account.key,
        *ctx.accounts.reserve_collateral_mint.key,
        *ctx.accounts.reserve_liquidity.key,
        *ctx.accounts.lending_market.key,
        *ctx.accounts.user_transfer_authority.key,
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
    pub reserve_account: AccountInfo<'info>,
    // Token mint for reserve collateral token
    pub reserve_collateral_mint: AccountInfo<'info>,
    // Reserve liquidity supply SPL token account
    pub reserve_liquidity_supply: AccountInfo<'info>,
    // Lending market account
    pub lending_market_account: AccountInfo<'info>,
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
    pub refreshed_reserve_account: AccountInfo<'info>,
    // Reserve collateral mint account
    pub reserve_collateral_mint: AccountInfo<'info>,
    // Reserve liquidity supply SPL Token account.
    pub reserve_liquidity: AccountInfo<'info>,
    // Lending market account
    pub lending_market: AccountInfo<'info>,
    // Lending market authority - PDA
    pub lending_market_authority: AccountInfo<'info>,
    // User transfer authority
    pub user_transfer_authority: AccountInfo<'info>,
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

pub mod solend_accessor {
    use std::convert::TryFrom;

    use anchor_lang::prelude::*;
    use spl_token_lending::math::{Decimal, Rate, TryAdd, TryDiv, U128};
    use spl_token_lending::state::{CollateralExchangeRate, INITIAL_COLLATERAL_RATE};

    fn unpack_decimal(src: &[u8; 16]) -> Decimal {
        Decimal::from_scaled_val(u128::from_le_bytes(*src))
    }

    pub fn reserve_available_liquidity(account: &AccountInfo) -> Result<u64, ProgramError> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&bytes[171..179]);
        Ok(u64::from_le_bytes(amount_bytes))
    }

    pub fn reserve_borrowed_amount(account: &AccountInfo) -> Result<Decimal, ProgramError> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 16];
        amount_bytes.copy_from_slice(&bytes[179..195]);
        Ok(unpack_decimal(&amount_bytes))
    }

    pub fn reserve_total_liquidity(account: &AccountInfo) -> Result<Decimal, ProgramError> {
        let available_liquidity = reserve_available_liquidity(account)?;
        let borrowed_amount = reserve_borrowed_amount(account)?;
        borrowed_amount.try_add(Decimal::from(available_liquidity))
    }

    pub fn reserve_mint_total(account: &AccountInfo) -> Result<u64, ProgramError> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&bytes[259..267]);
        Ok(u64::from_le_bytes(amount_bytes))
    }

    pub fn exchange_rate(account: &AccountInfo) -> Result<CollateralExchangeRate, ProgramError> {
        let mint_total_supply = reserve_mint_total(account)?;
        let total_liquidity = reserve_total_liquidity(account)?;
        let rate = if mint_total_supply == 0 || total_liquidity == Decimal::zero() {
            Rate::from_scaled_val(INITIAL_COLLATERAL_RATE)
        } else {
            let mint_total_supply = Decimal::from(mint_total_supply);
            Rate::try_from(mint_total_supply.try_div(total_liquidity)?)?
        };
        let rate = Rate(U128::from(rate.to_scaled_val()));
        Ok(CollateralExchangeRate(rate))
    }
}