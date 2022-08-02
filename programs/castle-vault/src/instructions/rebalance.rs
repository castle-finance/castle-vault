use std::{convert::TryFrom, ops::Deref};

use boolinator::Boolinator;
use pyth_sdk_solana::PriceStatus;
use strum::IntoEnumIterator;

use anchor_lang::prelude::*;
use port_anchor_adaptor::PortReserve;
use port_anchor_adaptor::PortStakingPool;
use solana_maths::Rate;
use solana_program::pubkey::Pubkey;

use pyth_sdk_solana::load_price_feed_from_account_info;

use crate::{
    adapters::SolendReserve,
    asset_container::AssetContainer,
    errors::ErrorCode,
    impl_provider_index,
    math::SLOTS_PER_YEAR,
    reserves::{PortReserveWrapper, Provider, Reserves},
    state::*,
};

#[event]
pub struct RebalanceEvent {
    vault: Pubkey,
}

/// Used by the SDK to figure out the order in which reconcile TXs should be sent
#[event]
#[derive(Default)]
pub struct RebalanceDataEvent {
    solend: u64,
    port: u64,
}
impl_provider_index!(RebalanceDataEvent, u64);

impl From<&Allocations> for RebalanceDataEvent {
    fn from(allocations: &Allocations) -> Self {
        Provider::iter().fold(Self::default(), |mut acc, provider| {
            acc[provider] = allocations[provider].value;
            acc
        })
    }
}

#[derive(Accounts)]
pub struct Rebalance<'info> {
    /// Vault state account
    /// Checks that the refresh has been called in the same slot
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        constraint = !vault.value.last_update.is_stale(clock.slot)? @ ErrorCode::VaultIsNotRefreshed,
    )]
    pub vault: Box<Account<'info, Vault>>,

    // DANGER: make sure the owner is as expected (currently done using `try_from`)
    //         and the keys match the vault (currently explicitly checked before `try_from`)
    /// CHECK: safe
    //#[soteria(ignore)]
    pub solend_reserve: AccountInfo<'info>,

    /// CHECK: safe
    //#[soteria(ignore)]
    pub port_reserve: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,
}

impl TryFrom<&Context<'_, '_, '_, '_, Rebalance<'_>>> for AssetContainer<Reserves> {
    type Error = Error;

    fn try_from(ctx: &Context<'_, '_, '_, '_, Rebalance<'_>>) -> Result<AssetContainer<Reserves>> {
        let r = &ctx.accounts;
        let flags: YieldSourceFlags = r.vault.get_yield_source_flags();

        // NOTE: I tried pretty hard to get rid of these clones and only use the references.
        // The problem is that these references originate from a deref() (or as_ref())
        // and end up sharing lifetimes with the Context<Rebalance>.accounts lifetime,
        // which means that the lifetimes are shared, preventing any other borrows
        // (in particular the mutable borrow required at the end to save state)

        // TODO is there a way to eliminate duplicate code here?
        let solend = flags
            .contains(YieldSourceFlags::SOLEND)
            .as_option()
            .map(|()| {
                r.solend_reserve.key.eq(&r.vault.solend_reserve).as_result(
                    Ok::<_, Error>(Reserves::Solend(Box::new(
                        Account::<SolendReserve>::try_from(&r.solend_reserve)?
                            .deref()
                            .clone(),
                    ))),
                    ErrorCode::InvalidAccount,
                )?
            })
            .transpose()?;

        let mut port: Option<Reserves> = None;
        if flags.contains(YieldSourceFlags::PORT) {
            if ctx.remaining_accounts.len() != 3 {
                return Err(ErrorCode::InvalidAccount.into());
            }

            let port_additional_states = &ctx.remaining_accounts[0];
            let port_reward_token_oracle = &ctx.remaining_accounts[1];
            let port_staking_pool = &ctx.remaining_accounts[2];

            let (port_additional_states_key, _) = Pubkey::find_program_address(
                &[r.vault.key().as_ref(), b"port_additional_state".as_ref()],
                ctx.program_id,
            );

            let port_additional_states_data = Box::new(
                Account::<VaultPortAdditionalState>::try_from(port_additional_states)?,
            );

            if r.port_reserve.key.ne(&r.vault.port_reserve)
                || port_additional_states_key.ne(port_additional_states.key)
                || port_additional_states_data
                    .port_reward_token_oracle
                    .key()
                    .ne(port_reward_token_oracle.key)
                || port_additional_states_data
                    .port_staking_pool
                    .key()
                    .ne(port_staking_pool.key)
            {
                return Err(ErrorCode::InvalidAccount.into());
            }

            let port_reserve = Box::new(
                Account::<PortReserve>::try_from(&r.port_reserve)?
                    .deref()
                    .clone(),
            );

            let pool_data = Box::new(Account::<PortStakingPool>::try_from(port_staking_pool)?);

            let port_exchange_rate = port_reserve.collateral_exchange_rate()?;
            let pool_size = port_exchange_rate.collateral_to_liquidity(pool_data.pool_size)?;
            let rate_per_slot = pool_data.rate_per_slot.try_floor_u64()?;
            let price_feed = load_price_feed_from_account_info(port_reward_token_oracle)
                .map_err(|_| ErrorCode::PriceFeedError)?;
            if price_feed.status != PriceStatus::Trading {
                return Err(ErrorCode::PriceFeedError.into());
            }
            let current_price = price_feed
                .get_current_price()
                .ok_or(ErrorCode::PriceFeedError)?;
            let price_raw = current_price.price as u64;
            let oracle_factor = (10_u64).pow(current_price.expo.unsigned_abs());
            let port_reward_per_year = rate_per_slot
                .checked_mul(price_raw)
                .ok_or(ErrorCode::MathError)?
                .checked_mul(SLOTS_PER_YEAR)
                .ok_or(ErrorCode::MathError)?
                .checked_div(oracle_factor)
                .ok_or(ErrorCode::MathError)?;

            #[cfg(feature = "debug")]
            {
                msg!("price_raw: {}", price_raw);
                msg!("rate_per_slot: {}", rate_per_slot);
                msg!("oracle_factor: {}", oracle_factor);
                msg!("SLOTS_PER_YEAR: {}", SLOTS_PER_YEAR);
                msg!("pool_size: {}", pool_size);
                msg!("pool_size_lp: {}", pool_data.pool_size);
                msg!("Expo: {}", current_price.expo);
                msg!("Reward per year: {}", port_reward_per_year);
            }

            port = Some(Reserves::Port(PortReserveWrapper {
                reserve: port_reserve,
                reward_per_year: port_reward_per_year,
                pool_size,
            }));
        }

        Ok(AssetContainer {
            inner: [solend, port],
        })
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub struct StrategyWeightsArg {
    solend: u16,
    port: u16,
}
impl_provider_index!(StrategyWeightsArg, u16);

// TODO use existing From impl
impl From<StrategyWeightsArg> for AssetContainer<Rate> {
    fn from(s: StrategyWeightsArg) -> Self {
        Provider::iter().fold(Self::default(), |mut acc, provider| {
            acc[provider] = Some(Rate::from_bips(s[provider] as u64));
            acc
        })
    }
}

/// Calculate and store optimal allocations to downstream lending markets
pub fn handler(ctx: Context<Rebalance>, proposed_weights_arg: StrategyWeightsArg) -> Result<()> {
    #[cfg(feature = "debug")]
    msg!("Rebalancing");

    let vault_value = ctx.accounts.vault.value.value;
    let slot = Clock::get()?.slot;

    let assets = Box::new(AssetContainer::<Reserves>::try_from(&ctx)?);
    let strategy_weights = assets.calculate_weights(
        ctx.accounts.vault.config.strategy_type,
        ctx.accounts.vault.config.allocation_cap_pct,
    )?;

    AssetContainer::<u64>::try_from_weights(&strategy_weights, vault_value)
        .and_then(
            |strategy_allocations| match ctx.accounts.vault.config.rebalance_mode {
                RebalanceMode::ProofChecker => {
                    let proposed_weights = AssetContainer::<Rate>::from(proposed_weights_arg);
                    let proposed_allocations =
                        AssetContainer::<u64>::try_from_weights(&strategy_weights, vault_value)?;

                    #[cfg(feature = "debug")]
                    msg!(
                        "Running as proof checker with proposed weights: {:?}",
                        proposed_weights.inner
                    );

                    // Check that proposed weights meet necessary constraints
                    proposed_weights
                        .verify_weights(ctx.accounts.vault.config.allocation_cap_pct)?;

                    let actual_allocations = ctx
                        .accounts
                        .vault
                        .actual_allocations
                        .to_container(ctx.accounts.vault.get_yield_source_flags());
                    let proposed_apr = assets.get_apr(
                        &proposed_weights,
                        &proposed_allocations,
                        &actual_allocations,
                    )?;
                    let proof_apr = assets.get_apr(
                        &strategy_weights,
                        &strategy_allocations,
                        &actual_allocations,
                    )?;

                    #[cfg(feature = "debug")]
                    msg!(
                        "Proposed APR: {:?}\nProof APR: {:?}",
                        proposed_apr,
                        proof_apr
                    );

                    // Return error if APR from proposed weights is not higher than proof weights
                    (proposed_apr >= proof_apr).as_result(
                        proposed_allocations,
                        ErrorCode::RebalanceProofCheckFailed.into(),
                    )
                }
                RebalanceMode::Calculator => {
                    #[cfg(feature = "debug")]
                    msg!("Running as calculator");
                    Ok(strategy_allocations)
                }
            },
        )
        .map(|final_allocations_container| {
            let final_allocations = Allocations::from_container(final_allocations_container, slot);

            #[cfg(feature = "debug")]
            msg!("Final allocations: {:?}", final_allocations);

            emit!(RebalanceEvent {
                vault: ctx.accounts.vault.key()
            });
            emit!(RebalanceDataEvent::from(&final_allocations));

            ctx.accounts.vault.target_allocations = final_allocations;
        })
}
