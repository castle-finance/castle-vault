use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use port_anchor_adaptor::{
    port_lending_id, port_staking_id, PortLendingMarket, PortObligation, PortReserve,
    PortStakeAccount, PortStakingPool,
};
use port_variable_rate_lending_instructions::{
    instruction::withdraw_obligation_collateral, state::Reserve,
};
use solana_maths::Rate;

use crate::{
    errors::ErrorCode,
    impl_has_vault,
    init_yield_source::YieldSourceInitializer,
    reconcile::LendingMarket,
    refresh::Refresher,
    reserves::{Provider, ReserveAccessor},
    state::{Vault, VaultPortAdditionalState, YieldSourceFlags},
};

#[derive(Accounts)]
pub struct PortAccounts<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_port_lp_token,
        has_one = port_reserve
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    /// CHECK: safe
    pub vault_authority: AccountInfo<'info>,

    #[account(
        seeds = [vault.key().as_ref(), b"port_additional_state".as_ref()],
        bump = vault.vault_port_additional_state_bump,
        has_one = port_staking_pool,
        has_one = port_lp_token_account
    )]
    pub port_additional_states: Box<Account<'info, VaultPortAdditionalState>>,

    /// Token account for the vault's reserve tokens
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's port lp tokens
    #[account(mut)]
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"port_obligation".as_ref()],
        bump = port_additional_states.vault_port_obligation_bump
    )]
    pub vault_port_obligation: Box<Account<'info, PortObligation>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"port_stake".as_ref()],
        bump = port_additional_states.vault_port_stake_account_bump
    )]
    pub vault_port_stake_account: Box<Account<'info, PortStakeAccount>>,

    #[account(
        mut,
        seeds = [vault.key().as_ref(), b"port_reward".as_ref()],
        bump = port_additional_states.vault_port_reward_token_bump
    )]
    pub vault_port_reward_token: Box<Account<'info, TokenAccount>>,

    /// ID of the staking pool
    #[account(mut)]
    pub port_staking_pool: Box<Account<'info, PortStakingPool>>,

    /// CHECK: safe
    #[account(
        executable,
        address = port_lending_id(),
    )]
    pub port_lend_program: AccountInfo<'info>,

    /// CHECK: safe
    #[account(
        executable,
        address = port_staking_id(),
    )]
    pub port_stake_program: AccountInfo<'info>,

    /// CHECK: safe
    //#[soteria(ignore)]
    pub port_staking_authority: AccountInfo<'info>,

    // Account to which the token should be transfered for the purpose of staking
    #[account(mut)]
    pub port_lp_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: safe
    //#[soteria(ignore)]
    pub port_market_authority: AccountInfo<'info>,

    //#[soteria(ignore)]
    pub port_market: Box<Account<'info, PortLendingMarket>>,

    #[account(mut)]
    pub port_reserve: Box<Account<'info, PortReserve>>,

    #[account(mut)]
    //#[soteria(ignore)]
    pub port_lp_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    //#[soteria(ignore)]
    pub port_reserve_token: Box<Account<'info, TokenAccount>>,

    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,
}

impl_has_vault!(PortAccounts<'_>);

impl<'info> LendingMarket for PortAccounts<'info> {
    fn deposit(&mut self, amount: u64) -> Result<()> {
        let context = CpiContext::new(
            self.port_lend_program.to_account_info(),
            port_anchor_adaptor::DepositAndCollateralize {
                source_liquidity: self.vault_reserve_token.to_account_info(),
                user_collateral: self.vault_port_lp_token.to_account_info(),
                reserve: self.port_reserve.to_account_info(),
                reserve_liquidity_supply: self.port_reserve_token.to_account_info(),
                reserve_collateral_mint: self.port_lp_mint.to_account_info(),
                lending_market: self.port_market.to_account_info(),
                lending_market_authority: self.port_market_authority.to_account_info(),
                destination_collateral: self.port_lp_token_account.to_account_info(),
                obligation: self.vault_port_obligation.to_account_info(),
                obligation_owner: self.vault_authority.to_account_info(),
                stake_account: self.vault_port_stake_account.to_account_info(),
                staking_pool: self.port_staking_pool.to_account_info(),
                transfer_authority: self.vault_authority.to_account_info(),
                clock: self.clock.to_account_info(),
                token_program: self.token_program.to_account_info(),
                port_staking_program: self.port_stake_program.to_account_info(),
            },
        );
        match amount {
            0 => Ok(()),
            _ => port_anchor_adaptor::deposit_and_collateralize(
                context.with_signer(&[&self.vault.authority_seeds()]),
                amount,
            ),
        }?;

        let port_value = self.vault.actual_allocations[Provider::Port]
            .value
            .checked_add(amount)
            .ok_or(ErrorCode::MathError)?;
        self.vault.actual_allocations[Provider::Port].update(port_value, self.clock.slot);

        Ok(())
    }

    fn redeem(&mut self, amount: u64) -> Result<()> {
        let refresh_obligation_context = CpiContext::new(
            self.port_lend_program.clone(),
            port_anchor_adaptor::RefreshObligation {
                obligation: self.vault_port_obligation.to_account_info(),
                clock: self.clock.to_account_info(),
            },
        );

        let port_withdraw_accounts = PortWithdrawAccounts {
            source_collateral: self.port_lp_token_account.to_account_info(),
            destination_collateral: self.vault_port_lp_token.to_account_info(),
            reserve: self.port_reserve.to_account_info(),
            obligation: self.vault_port_obligation.to_account_info(),
            lending_market: self.port_market.to_account_info(),
            lending_market_authority: self.port_market_authority.to_account_info(),
            stake_account: self.vault_port_stake_account.to_account_info(),
            staking_pool: self.port_staking_pool.to_account_info(),
            obligation_owner: self.vault_authority.to_account_info(),
            clock: self.clock.to_account_info(),
            token_program: self.token_program.to_account_info(),
            port_stake_program: self.port_stake_program.to_account_info(),
            port_lend_program: self.port_lend_program.to_account_info(),
        };

        let redeem_context = CpiContext::new(
            self.port_lend_program.clone(),
            port_anchor_adaptor::Redeem {
                source_collateral: self.vault_port_lp_token.to_account_info(),
                destination_liquidity: self.vault_reserve_token.to_account_info(),
                reserve: self.port_reserve.to_account_info(),
                reserve_collateral_mint: self.port_lp_mint.to_account_info(),
                reserve_liquidity_supply: self.port_reserve_token.to_account_info(),
                lending_market: self.port_market.to_account_info(),
                lending_market_authority: self.port_market_authority.to_account_info(),
                transfer_authority: self.vault_authority.to_account_info(),
                clock: self.clock.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        );

        if amount == 0 {
            return Ok(());
        }

        port_anchor_adaptor::refresh_port_obligation(
            refresh_obligation_context
                .with_remaining_accounts(vec![self.port_reserve.to_account_info()])
                .with_signer(&[&self.vault.authority_seeds()]),
        )?;

        port_withdraw_obligation_collateral(
            amount,
            &port_withdraw_accounts,
            &[&self.vault.authority_seeds()],
        )?;

        port_anchor_adaptor::redeem(
            redeem_context.with_signer(&[&self.vault.authority_seeds()]),
            amount,
        )?;

        let vault_reserve_value_delta = self.convert_amount_lp_to_reserve(amount)?;
        let port_value = self.vault.actual_allocations[Provider::Port]
            .value
            .checked_sub(vault_reserve_value_delta)
            .ok_or(ErrorCode::MathError)?;
        self.vault.actual_allocations[Provider::Port].update(port_value, self.clock.slot);

        Ok(())
    }

    fn convert_amount_reserve_to_lp(&self, amount: u64) -> Result<u64> {
        let exchange_rate = self.port_reserve.collateral_exchange_rate()?;
        exchange_rate
            .liquidity_to_collateral(amount)
            .map_err(|e| e.into())
    }

    fn convert_amount_lp_to_reserve(&self, amount: u64) -> Result<u64> {
        let exchange_rate = self.port_reserve.collateral_exchange_rate()?;
        exchange_rate
            .collateral_to_liquidity(amount)
            .map_err(|e| e.into())
    }

    fn reserve_tokens_in_vault(&self) -> u64 {
        self.vault_reserve_token.amount
    }

    fn lp_tokens_in_vault(&self) -> u64 {
        self.vault_port_lp_token.amount + self.vault_port_stake_account.deposited_amount
    }

    fn provider(&self) -> Provider {
        Provider::Port
    }
}

pub struct PortWithdrawAccounts<'info> {
    /// CHECK: safe
    source_collateral: AccountInfo<'info>,
    /// CHECK: safe
    destination_collateral: AccountInfo<'info>,
    /// CHECK: safe
    reserve: AccountInfo<'info>,
    /// CHECK: safe
    obligation: AccountInfo<'info>,
    /// CHECK: safe
    lending_market: AccountInfo<'info>,
    /// CHECK: safe
    lending_market_authority: AccountInfo<'info>,
    /// CHECK: safe
    stake_account: AccountInfo<'info>,
    /// CHECK: safe
    staking_pool: AccountInfo<'info>,
    /// CHECK: safe
    obligation_owner: AccountInfo<'info>,
    /// CHECK: safe
    clock: AccountInfo<'info>,
    /// CHECK: safe
    token_program: AccountInfo<'info>,
    /// CHECK: safe
    port_stake_program: AccountInfo<'info>,
    /// CHECK: safe
    port_lend_program: AccountInfo<'info>,
}

fn port_withdraw_obligation_collateral<'info>(
    amount: u64,
    port_withdraw_accounts: &PortWithdrawAccounts<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let ix = withdraw_obligation_collateral(
        port_withdraw_accounts.port_lend_program.key(),
        amount,
        port_withdraw_accounts.source_collateral.key(),
        port_withdraw_accounts.destination_collateral.key(),
        port_withdraw_accounts.reserve.key(),
        port_withdraw_accounts.obligation.key(),
        port_withdraw_accounts.lending_market.key(),
        port_withdraw_accounts.obligation_owner.key(),
        Some(port_withdraw_accounts.stake_account.key()),
        Some(port_withdraw_accounts.staking_pool.key()),
    );

    solana_program::program::invoke_signed(
        &ix,
        &[
            port_withdraw_accounts.source_collateral.clone(),
            port_withdraw_accounts.destination_collateral.clone(),
            port_withdraw_accounts.reserve.clone(),
            port_withdraw_accounts.obligation.clone(),
            port_withdraw_accounts.lending_market.clone(),
            port_withdraw_accounts.lending_market_authority.clone(),
            port_withdraw_accounts.obligation_owner.clone(),
            port_withdraw_accounts.clock.clone(),
            port_withdraw_accounts.token_program.clone(),
            port_withdraw_accounts.stake_account.clone(),
            port_withdraw_accounts.staking_pool.clone(),
            port_withdraw_accounts.port_stake_program.clone(),
            port_withdraw_accounts.port_lend_program.clone(),
        ],
        signer_seeds,
    )
    .map_err(Into::into)
}

impl ReserveAccessor for Reserve {
    fn utilization_rate(&self) -> Result<Rate> {
        Ok(Rate::from_scaled_val(
            self.liquidity.utilization_rate()?.to_scaled_val() as u64,
        ))
    }

    fn borrow_rate(&self) -> Result<Rate> {
        Ok(Rate::from_scaled_val(
            self.current_borrow_rate()?.to_scaled_val() as u64,
        ))
    }

    fn reserve_with_deposit(
        &self,
        new_allocation: u64,
        old_allocation: u64,
    ) -> Result<Box<dyn ReserveAccessor>> {
        let mut reserve = Box::new(self.clone());
        reserve.liquidity.available_amount = reserve
            .liquidity
            .available_amount
            .checked_add(new_allocation)
            .ok_or(ErrorCode::OverflowError)?
            .checked_sub(old_allocation)
            .ok_or(ErrorCode::OverflowError)?;
        Ok(reserve)
    }
}

#[derive(Accounts)]
pub struct InitializePort<'info> {
    #[account(
        mut,
        has_one = owner,
        has_one = vault_authority,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// CHECK: safe
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's port lp tokens
    #[account(
        init,
        payer = payer,
        seeds = [vault.key().as_ref(), port_lp_token_mint.key().as_ref()],
        bump,
        token::authority = vault_authority,
        token::mint = port_lp_token_mint,
    )]
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    /// Mint of the port lp token
    pub port_lp_token_mint: Box<Account<'info, Mint>>,

    pub port_reserve: Box<Account<'info, PortReserve>>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}

impl<'info> YieldSourceInitializer<'info> for InitializePort<'info> {
    fn initialize_yield_source(&mut self) -> Result<()> {
        self.vault.port_reserve = self.port_reserve.key();
        self.vault.vault_port_lp_token = self.vault_port_lp_token.key();

        let mut new_flags = self.vault.get_yield_source_flags();
        new_flags.set(YieldSourceFlags::PORT, true);
        self.vault.set_yield_source_flags(new_flags.bits())
    }
}

#[derive(Accounts)]
pub struct RefreshPort<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_port_lp_token,
        has_one = port_reserve,
    )]
    pub vault: Box<Account<'info, Vault>>,

    #[account(
        seeds = [vault.key().as_ref(), b"port_additional_state".as_ref()],
        bump = vault.vault_port_additional_state_bump
    )]
    pub port_additional_states: Box<Account<'info, VaultPortAdditionalState>>,

    /// Token account for the vault's port lp tokens
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [vault.key().as_ref(), b"port_stake".as_ref()],
        bump = port_additional_states.vault_port_stake_account_bump
    )]
    pub vault_port_stake_account: Box<Account<'info, PortStakeAccount>>,

    /// CHECK: safe
    #[account(
        executable,
        address = port_lending_id(),
    )]
    pub port_lend_program: AccountInfo<'info>,

    #[account(mut)]
    pub port_reserve: Box<Account<'info, PortReserve>>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info> RefreshPort<'info> {
    fn port_refresh_reserve_context(
        &self,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> CpiContext<'_, '_, '_, 'info, port_anchor_adaptor::RefreshReserve<'info>> {
        CpiContext::new(
            self.port_lend_program.clone(),
            port_anchor_adaptor::RefreshReserve {
                reserve: self.port_reserve.to_account_info(),
                clock: self.clock.to_account_info(),
            },
        )
        .with_remaining_accounts(remaining_accounts.to_vec())
    }
}

impl<'info> Refresher<'info> for RefreshPort<'info> {
    fn update_actual_allocation(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> Result<()> {
        if self
            .vault
            .get_yield_source_flags()
            .contains(YieldSourceFlags::PORT)
        {
            port_anchor_adaptor::refresh_port_reserve(
                self.port_refresh_reserve_context(remaining_accounts),
            )?;

            #[cfg(feature = "debug")]
            msg!("Refreshing port");

            let port_exchange_rate = self.port_reserve.collateral_exchange_rate()?;
            let port_value = port_exchange_rate.collateral_to_liquidity(
                self.vault_port_lp_token
                    .amount
                    .checked_add(self.vault_port_stake_account.deposited_amount)
                    .ok_or(ErrorCode::OverflowError)?,
            )?;

            #[cfg(feature = "debug")]
            msg!("Refresh port reserve token value: {}", port_value);

            self.vault.actual_allocations[Provider::Port].update(port_value, self.clock.slot);
        }

        Ok(())
    }
}
