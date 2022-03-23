use anchor_lang::solana_program::program_pack::Pack;
/// Modified from @RohanKapurDEV
/// https://github.com/RohanKapurDEV/anchor-lending
use anchor_lang::{prelude::*, solana_program};
use spl_token_lending::state::Reserve;
use std::io::Write;
use std::ops::Deref;

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

#[derive(Clone)]
pub struct SolendReserve(Reserve);

impl anchor_lang::AccountDeserialize for SolendReserve {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        SolendReserve::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Reserve::unpack(buf).map(SolendReserve)
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
