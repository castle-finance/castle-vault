use std::{
    io::Write,
    iter,
    ops::{Deref, DerefMut},
};

use anchor_lang::{prelude::*, solana_program};
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount};
use solana_maths::{Rate, TryMul};

use crate::{
    errors::ErrorCode,
    impl_has_vault,
    init_yield_source::YieldSourceInitializer,
    reconcile::LendingMarket,
    refresh::Refresher,
    reserves::{Provider, ReserveAccessor, ReturnCalculator},
    state::{Vault, VaultMangoAdditionalState, YieldSourceFlags},
};

extern crate mango as mango_lib;

#[derive(Accounts)]
pub struct MangoAccounts<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_mango_lp_token,
        has_one = vault_mango_account,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    /// CHECK: safe
    pub vault_authority: AccountInfo<'info>,

    #[account(
        seeds = [vault.key().as_ref(), b"mango_additional_state".as_ref()],
        bump = vault.vault_mango_additional_state_bump,
        has_one = mango_lp_token_mint
    )]
    pub mango_additional_state: Box<Account<'info, VaultMangoAdditionalState>>,

    // TODO add seeds check
    /// CHECK: safe
    #[account(mut)]
    pub mango_lp_token_mint: Box<Account<'info, Mint>>,

    /// Token account for the vault's reserve tokens
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    // TODO add seeds check
    /// Token account for the vault's mango lp tokens
    #[account(mut)]
    pub vault_mango_lp_token: Box<Account<'info, TokenAccount>>,

    /// CHECK: safe
    #[account(mut)]
    pub vault_mango_account: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub mango_group: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub mango_group_signer: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub mango_cache: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub mango_root_bank: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub mango_node_bank: AccountInfo<'info>,

    /// CHECK: safe
    #[account(mut)]
    pub mango_vault: AccountInfo<'info>,

    /// CHECK: safe
    #[account(
        executable,
        // address = spl_token_lending::ID,
    )]
    pub mango_program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

impl_has_vault!(MangoAccounts<'_>);

pub fn deposit(ctx: Context<MangoAccounts>) -> Result<()> {
    let deposit_ix = mango_lib::instruction::deposit(
        &ctx.accounts.mango_program.key(),
        &ctx.accounts.mango_group.key(),
        &ctx.accounts.vault_mango_account.key(),
        &ctx.accounts.vault_authority.key(),
        &ctx.accounts.mango_cache.key(),
        &ctx.accounts.mango_root_bank.key(),
        &ctx.accounts.mango_node_bank.key(),
        &ctx.accounts.mango_vault.key(),
        &ctx.accounts.vault_reserve_token.key(),
        1000,
    )?;

    solana_program::program::invoke_signed(
        &deposit_ix,
        &[
            ctx.accounts.mango_program.to_account_info().clone(),
            ctx.accounts.mango_group.to_account_info().clone(),
            ctx.accounts.vault_mango_account.to_account_info().clone(),
            ctx.accounts.vault_authority.to_account_info().clone(),
            ctx.accounts.mango_cache.to_account_info().clone(),
            ctx.accounts.mango_root_bank.to_account_info().clone(),
            ctx.accounts.mango_node_bank.to_account_info().clone(),
            ctx.accounts.mango_vault.to_account_info().clone(),
            ctx.accounts.vault_reserve_token.to_account_info().clone(),
        ],
        &[&ctx.accounts.vault.authority_seeds()],
    )?;

    let mint_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            to: ctx.accounts.vault_mango_lp_token.to_account_info(),
            mint: ctx.accounts.mango_lp_token_mint.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        },
    );

    token::mint_to(
        mint_ctx.with_signer(&[&ctx.accounts.vault.authority_seeds()]),
        1000,
    )?;

    Ok(())
}

pub fn withdraw(ctx: Context<MangoAccounts>) -> Result<()> {
    let open_orders_iter = &mut iter::empty::<Pubkey>();

    let withdraw_ix = mango_lib::instruction::withdraw2(
        &ctx.accounts.mango_program.key(),
        &ctx.accounts.mango_group.key(),
        &ctx.accounts.vault_mango_account.key(),
        &ctx.accounts.vault_authority.key(),
        &ctx.accounts.mango_cache.key(),
        &ctx.accounts.mango_root_bank.key(),
        &ctx.accounts.mango_node_bank.key(),
        &ctx.accounts.mango_vault.key(),
        &ctx.accounts.vault_reserve_token.key(),
        &ctx.accounts.mango_group_signer.key(),
        open_orders_iter,
        u64::MAX, // withdraw all
        false,
    )?;

    solana_program::program::invoke_signed(
        &withdraw_ix,
        &[
            ctx.accounts.mango_program.to_account_info().clone(),
            ctx.accounts.mango_group.to_account_info().clone(),
            ctx.accounts.vault_mango_account.to_account_info().clone(),
            ctx.accounts.vault_authority.to_account_info().clone(),
            ctx.accounts.mango_cache.to_account_info().clone(),
            ctx.accounts.mango_root_bank.to_account_info().clone(),
            ctx.accounts.mango_node_bank.to_account_info().clone(),
            ctx.accounts.mango_vault.to_account_info().clone(),
            ctx.accounts.vault_reserve_token.to_account_info().clone(),
            ctx.accounts.mango_group_signer.to_account_info().clone(),
        ],
        &[&ctx.accounts.vault.authority_seeds()],
    )?;

    let burn_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Burn {
            mint: ctx.accounts.mango_lp_token_mint.to_account_info(),
            from: ctx.accounts.vault_mango_lp_token.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        },
    );

    token::burn(
        burn_ctx.with_signer(&[&ctx.accounts.vault.authority_seeds()]),
        1000,
    )?;

    Ok(())
}

impl<'info> LendingMarket for MangoAccounts<'info> {
    fn deposit(&mut self, amount: u64) -> Result<()> {
        let deposit_ix = mango_lib::instruction::deposit(
            &self.mango_program.key(),
            &self.mango_group.key(),
            &self.vault_mango_account.key(),
            &self.vault_authority.key(),
            &self.mango_cache.key(),
            &self.mango_root_bank.key(),
            &self.mango_node_bank.key(),
            &self.mango_vault.key(),
            &self.vault_reserve_token.key(),
            1000,
        )?;

        match amount {
            0 => Ok(()),
            _ => solana_program::program::invoke_signed(
                    &deposit_ix,
                    &[
                        self.mango_program.to_account_info().clone(),
                        self.mango_group.to_account_info().clone(),
                        self.vault_mango_account.to_account_info().clone(),
                        self.vault_authority.to_account_info().clone(),
                        self.mango_cache.to_account_info().clone(),
                        self.mango_root_bank.to_account_info().clone(),
                        self.mango_node_bank.to_account_info().clone(),
                        self.mango_vault.to_account_info().clone(),
                        self.vault_reserve_token.to_account_info().clone(),
                    ],
                    &[&self.vault.authority_seeds()],
                )
        }?;

        let mint_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            MintTo {
                to: self.vault_mango_lp_token.to_account_info(),
                mint: self.mango_lp_token_mint.to_account_info(),
                authority: self.vault_authority.to_account_info(),
            },
        );
    
        token::mint_to(
            mint_ctx.with_signer(&[&self.vault.authority_seeds()]),
            1000,
        )?;

        // let solend_value = self.vault.actual_allocations[Provider::Solend]
        //     .value
        //     .checked_add(amount)
        //     .ok_or(ErrorCode::MathError)?;
        // self.vault.actual_allocations[Provider::Solend].update(solend_value, self.clock.slot);
        Ok(())
    }

    fn redeem(&mut self, amount: u64) -> Result<()> {
        let open_orders_iter = &mut iter::empty::<Pubkey>();

        let withdraw_ix = mango_lib::instruction::withdraw2(
            &self.mango_program.key(),
            &self.mango_group.key(),
            &self.vault_mango_account.key(),
            &self.vault_authority.key(),
            &self.mango_cache.key(),
            &self.mango_root_bank.key(),
            &self.mango_node_bank.key(),
            &self.mango_vault.key(),
            &self.vault_reserve_token.key(),
            &self.mango_group_signer.key(),
            open_orders_iter,
            amount,
            false
        )?;

        match amount {
            0 => Ok(()),
            _ => solana_program::program::invoke_signed(
                    &withdraw_ix,
                    &[
                        self.mango_program.to_account_info().clone(),
                        self.mango_group.to_account_info().clone(),
                        self.vault_mango_account.to_account_info().clone(),
                        self.vault_authority.to_account_info().clone(),
                        self.mango_cache.to_account_info().clone(),
                        self.mango_root_bank.to_account_info().clone(),
                        self.mango_node_bank.to_account_info().clone(),
                        self.mango_vault.to_account_info().clone(),
                        self.vault_reserve_token.to_account_info().clone(),
                        self.mango_group_signer.to_account_info().clone(),
                    ],
                    &[&self.vault.authority_seeds()],
                )
        }?;

        let burn_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint: self.mango_lp_token_mint.to_account_info(),
                from: self.vault_mango_lp_token.to_account_info(),
                authority: self.vault_authority.to_account_info(),
            },
        );

        token::burn(
            burn_ctx.with_signer(&[&self.vault.authority_seeds()]),
            1000,
        )?;

        // let vault_reserve_vault_delta = self.convert_amount_lp_to_reserve(amount)?;
        // let solend_value = self.vault.actual_allocations[Provider::Solend]
        //     .value
        //     .checked_sub(vault_reserve_vault_delta)
        //     .ok_or(ErrorCode::MathError)?;
        // self.vault.actual_allocations[Provider::Solend].update(solend_value, self.clock.slot);
        Ok(())
    }

    fn convert_amount_reserve_to_lp(&self, amount: u64) -> Result<u64> {
        // let exchange_rate = self.solend_reserve.collateral_exchange_rate()?;
        // Ok(exchange_rate.liquidity_to_collateral(amount)?)
        Ok(0)
    }

    fn convert_amount_lp_to_reserve(&self, amount: u64) -> Result<u64> {
        // let exchange_rate = self.solend_reserve.collateral_exchange_rate()?;
        // Ok(exchange_rate.collateral_to_liquidity(amount)?)
        Ok(0)
    }

    fn reserve_tokens_in_vault(&self) -> u64 {
        self.vault_reserve_token.amount
    }

    fn lp_tokens_in_vault(&self) -> u64 {
        self.vault_mango_lp_token.amount
    }

    fn provider(&self) -> Provider {
        Provider::Solend
    }
}
