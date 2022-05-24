#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2018::*;
#[macro_use]
extern crate std;
use anchor_lang::prelude::*;
pub mod adapters {
    pub mod jet {
        use std::ops::{Deref, DerefMut};
        use anchor_lang::prelude::*;
        use anchor_spl::token::{Token, TokenAccount};
        use jet::{
            state::{CachedReserveInfo, Reserve},
            Amount, Rounding,
        };
        use solana_maths::Rate;
        use crate::{
            impl_has_vault,
            init_yield_source::YieldSourceInitializer,
            reconcile::LendingMarket,
            refresh::Refresher,
            reserves::{Provider, ReserveAccessor},
            state::{Vault, YieldSourceFlags},
        };
        pub struct JetAccounts<'info> {
            /// Vault state account
            /// Checks that the accounts passed in are correct
            # [account (mut , has_one = vault_authority , has_one = vault_reserve_token , has_one = vault_jet_lp_token , has_one = jet_reserve ,)]
            pub vault: Box<Account<'info, Vault>>,
            /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
            pub vault_authority: AccountInfo<'info>,
            /// Token account for the vault's reserve tokens
            #[account(mut)]
            pub vault_reserve_token: Box<Account<'info, TokenAccount>>,
            /// Token account for the vault's jet lp tokens
            #[account(mut)]
            pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,
            # [account (executable , address = jet :: ID ,)]
            pub jet_program: AccountInfo<'info>,
            pub jet_market: AccountLoader<'info, jet::state::Market>,
            pub jet_market_authority: AccountInfo<'info>,
            #[account(mut)]
            pub jet_reserve: AccountLoader<'info, jet::state::Reserve>,
            #[account(mut)]
            pub jet_reserve_token: AccountInfo<'info>,
            #[account(mut)]
            pub jet_lp_mint: AccountInfo<'info>,
            pub token_program: Program<'info, Token>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for JetAccounts<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_reserve_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_jet_lp_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_program: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_market: anchor_lang::AccountLoader<jet::state::Market> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_market_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_reserve: anchor_lang::AccountLoader<jet::state::Reserve> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_reserve_token: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_lp_mint: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.vault_authority != vault_authority.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_reserve_token != vault_reserve_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_jet_lp_token != vault_jet_lp_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.jet_reserve != jet_reserve.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !vault_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !vault_jet_lp_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !jet_program.to_account_info().executable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintExecutable.into());
                }
                if jet_program.to_account_info().key != &jet::ID {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintAddress.into());
                }
                if !jet_reserve.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !jet_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !jet_lp_mint.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(JetAccounts {
                    vault,
                    vault_authority,
                    vault_reserve_token,
                    vault_jet_lp_token,
                    jet_program,
                    jet_market,
                    jet_market_authority,
                    jet_reserve,
                    jet_reserve_token,
                    jet_lp_mint,
                    token_program,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for JetAccounts<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_authority.to_account_infos());
                account_infos.extend(self.vault_reserve_token.to_account_infos());
                account_infos.extend(self.vault_jet_lp_token.to_account_infos());
                account_infos.extend(self.jet_program.to_account_infos());
                account_infos.extend(self.jet_market.to_account_infos());
                account_infos.extend(self.jet_market_authority.to_account_infos());
                account_infos.extend(self.jet_reserve.to_account_infos());
                account_infos.extend(self.jet_reserve_token.to_account_infos());
                account_infos.extend(self.jet_lp_mint.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for JetAccounts<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_authority.to_account_metas(None));
                account_metas.extend(self.vault_reserve_token.to_account_metas(None));
                account_metas.extend(self.vault_jet_lp_token.to_account_metas(None));
                account_metas.extend(self.jet_program.to_account_metas(None));
                account_metas.extend(self.jet_market.to_account_metas(None));
                account_metas.extend(self.jet_market_authority.to_account_metas(None));
                account_metas.extend(self.jet_reserve.to_account_metas(None));
                account_metas.extend(self.jet_reserve_token.to_account_metas(None));
                account_metas.extend(self.jet_lp_mint.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for JetAccounts<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_reserve_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_jet_lp_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.jet_reserve, program_id)?;
                anchor_lang::AccountsExit::exit(&self.jet_reserve_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.jet_lp_mint, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_jet_accounts {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct JetAccounts {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_jet_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_market: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_market_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_lp_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for JetAccounts
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_jet_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_market, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_market_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_lp_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for JetAccounts {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_authority,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_reserve_token,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_jet_lp_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.jet_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.jet_market,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.jet_market_authority,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.jet_reserve,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.jet_reserve_token,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.jet_lp_mint,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_jet_accounts {
            use super::*;
            pub struct JetAccounts<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_jet_lp_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_market: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_market_authority:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_lp_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for JetAccounts<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_authority),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_reserve_token),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_jet_lp_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.jet_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.jet_market),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.jet_market_authority),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.jet_reserve),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.jet_reserve_token),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.jet_lp_mint),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for JetAccounts<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_jet_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_market,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_market_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_lp_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos
                }
            }
        }
        impl<'info> JetAccounts<'info> {
            fn get_reserve_info(&self) -> Result<CachedReserveInfo, ProgramError> {
                let market = self.jet_market.load()?;
                let reserve = self.jet_reserve.load()?;
                let clock = Clock::get()?;
                Ok(*market.reserves().get_cached(reserve.index, clock.slot))
            }
        }
        impl crate::instructions::reconcile::HasVault for JetAccounts<'_> {
            fn vault(&self) -> &Vault {
                self.vault.deref()
            }
            fn vault_mut(&mut self) -> &mut Vault {
                self.vault.deref_mut()
            }
        }
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
                allocation: u64,
            ) -> Result<Box<dyn ReserveAccessor>, ProgramError> {
                let mut reserve = Box::new(*self);
                reserve.deposit(allocation, 0);
                Ok(reserve)
            }
        }
        # [instruction (bump : u8)]
        pub struct InitializeJet<'info> {
            # [account (mut , has_one = owner , has_one = vault_authority ,)]
            pub vault: Box<Account<'info, Vault>>,
            pub vault_authority: AccountInfo<'info>,
            /// Token account for the vault's jet lp tokens
            # [account (init , payer = payer , seeds = [vault . key () . as_ref () , jet_lp_token_mint . key () . as_ref ()] , bump = bump , token :: authority = vault_authority , token :: mint = jet_lp_token_mint ,)]
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
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for InitializeJet<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let mut ix_data = ix_data;
                struct __Args {
                    bump: u8,
                }
                impl borsh::ser::BorshSerialize for __Args
                where
                    u8: borsh::ser::BorshSerialize,
                {
                    fn serialize<W: borsh::maybestd::io::Write>(
                        &self,
                        writer: &mut W,
                    ) -> ::core::result::Result<(), borsh::maybestd::io::Error>
                    {
                        borsh::BorshSerialize::serialize(&self.bump, writer)?;
                        Ok(())
                    }
                }
                impl borsh::de::BorshDeserialize for __Args
                where
                    u8: borsh::BorshDeserialize,
                {
                    fn deserialize(
                        buf: &mut &[u8],
                    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error>
                    {
                        Ok(Self {
                            bump: borsh::BorshDeserialize::deserialize(buf)?,
                        })
                    }
                }
                let __Args { bump } = __Args::deserialize(&mut ix_data)
                    .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_jet_lp_token = &accounts[0];
                *accounts = &accounts[1..];
                let jet_lp_token_mint: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_reserve: anchor_lang::AccountLoader<jet::state::Reserve> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let owner: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let payer: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let system_program: anchor_lang::Program<System> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let rent: Sysvar<Rent> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let __anchor_rent = Rent::get()?;
                let vault_jet_lp_token: Box<anchor_lang::Account<TokenAccount>> = {
                    if !false
                        || vault_jet_lp_token.to_account_info().owner
                            == &anchor_lang::solana_program::system_program::ID
                    {
                        let payer = payer.to_account_info();
                        let __current_lamports = vault_jet_lp_token.to_account_info().lamports();
                        if __current_lamports == 0 {
                            let lamports =
                                __anchor_rent.minimum_balance(anchor_spl::token::TokenAccount::LEN);
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::create_account(
                                    payer.to_account_info().key,
                                    vault_jet_lp_token.to_account_info().key,
                                    lamports,
                                    anchor_spl::token::TokenAccount::LEN as u64,
                                    token_program.to_account_info().key,
                                ),
                                &[
                                    payer.to_account_info(),
                                    vault_jet_lp_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    jet_lp_token_mint.key().as_ref(),
                                    &[bump][..],
                                ][..]],
                            )?;
                        } else {
                            let required_lamports = __anchor_rent
                                .minimum_balance(anchor_spl::token::TokenAccount::LEN)
                                .max(1)
                                .saturating_sub(__current_lamports);
                            if required_lamports > 0 {
                                anchor_lang::solana_program::program::invoke(
                                    &anchor_lang::solana_program::system_instruction::transfer(
                                        payer.to_account_info().key,
                                        vault_jet_lp_token.to_account_info().key,
                                        required_lamports,
                                    ),
                                    &[
                                        payer.to_account_info(),
                                        vault_jet_lp_token.to_account_info(),
                                        system_program.to_account_info(),
                                    ],
                                )?;
                            }
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::allocate(
                                    vault_jet_lp_token.to_account_info().key,
                                    anchor_spl::token::TokenAccount::LEN as u64,
                                ),
                                &[
                                    vault_jet_lp_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    jet_lp_token_mint.key().as_ref(),
                                    &[bump][..],
                                ][..]],
                            )?;
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::assign(
                                    vault_jet_lp_token.to_account_info().key,
                                    token_program.to_account_info().key,
                                ),
                                &[
                                    vault_jet_lp_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    jet_lp_token_mint.key().as_ref(),
                                    &[bump][..],
                                ][..]],
                            )?;
                        }
                        let cpi_program = token_program.to_account_info();
                        let accounts = anchor_spl::token::InitializeAccount {
                            account: vault_jet_lp_token.to_account_info(),
                            mint: jet_lp_token_mint.to_account_info(),
                            authority: vault_authority.to_account_info(),
                            rent: rent.to_account_info(),
                        };
                        let cpi_ctx = CpiContext::new(cpi_program, accounts);
                        anchor_spl::token::initialize_account(cpi_ctx)?;
                    }
                    let pa: Box<anchor_lang::Account<TokenAccount>> = Box::new(
                        anchor_lang::Account::try_from_unchecked(&vault_jet_lp_token)?,
                    );
                    pa
                };
                let (__program_signer, __bump) =
                    anchor_lang::solana_program::pubkey::Pubkey::find_program_address(
                        &[vault.key().as_ref(), jet_lp_token_mint.key().as_ref()],
                        program_id,
                    );
                if vault_jet_lp_token.to_account_info().key != &__program_signer {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if __bump != bump {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if !vault_jet_lp_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !__anchor_rent.is_exempt(
                    vault_jet_lp_token.to_account_info().lamports(),
                    vault_jet_lp_token.to_account_info().try_data_len()?,
                ) {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
                }
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.owner != owner.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_authority != vault_authority.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !payer.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(InitializeJet {
                    vault,
                    vault_authority,
                    vault_jet_lp_token,
                    jet_lp_token_mint,
                    jet_reserve,
                    owner,
                    payer,
                    token_program,
                    system_program,
                    rent,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for InitializeJet<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_authority.to_account_infos());
                account_infos.extend(self.vault_jet_lp_token.to_account_infos());
                account_infos.extend(self.jet_lp_token_mint.to_account_infos());
                account_infos.extend(self.jet_reserve.to_account_infos());
                account_infos.extend(self.owner.to_account_infos());
                account_infos.extend(self.payer.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos.extend(self.system_program.to_account_infos());
                account_infos.extend(self.rent.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for InitializeJet<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_authority.to_account_metas(None));
                account_metas.extend(self.vault_jet_lp_token.to_account_metas(None));
                account_metas.extend(self.jet_lp_token_mint.to_account_metas(None));
                account_metas.extend(self.jet_reserve.to_account_metas(None));
                account_metas.extend(self.owner.to_account_metas(None));
                account_metas.extend(self.payer.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas.extend(self.system_program.to_account_metas(None));
                account_metas.extend(self.rent.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for InitializeJet<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_jet_lp_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.payer, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_initialize_jet {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct InitializeJet {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_jet_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_lp_token_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub owner: anchor_lang::solana_program::pubkey::Pubkey,
                pub payer: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub rent: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for InitializeJet
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_jet_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_lp_token_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.owner, writer)?;
                    borsh::BorshSerialize::serialize(&self.payer, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.rent, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for InitializeJet {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_authority,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_jet_lp_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.jet_lp_token_mint,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.jet_reserve,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.owner, true,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.payer, true,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.system_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.rent, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_initialize_jet {
            use super::*;
            pub struct InitializeJet<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_jet_lp_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_lp_token_mint:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub rent: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for InitializeJet<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_authority),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_jet_lp_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.jet_lp_token_mint),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.jet_reserve),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.owner),
                            true,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.payer),
                        true,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.system_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.rent),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for InitializeJet<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_jet_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_lp_token_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.owner));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.payer));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.system_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.rent));
                    account_infos
                }
            }
        }
        impl<'info> YieldSourceInitializer<'info> for InitializeJet<'info> {
            fn initialize_yield_source(&mut self) -> ProgramResult {
                self.vault.jet_reserve = self.jet_reserve.key();
                self.vault.vault_jet_lp_token = self.vault_jet_lp_token.key();
                self.vault
                    .set_yield_source_flag(YieldSourceFlags::JET, true)?;
                Ok(())
            }
        }
        pub struct RefreshJet<'info> {
            /// Vault state account
            /// Checks that the accounts passed in are correct
            # [account (mut , has_one = vault_jet_lp_token , has_one = jet_reserve ,)]
            pub vault: Box<Account<'info, Vault>>,
            /// Token account for the vault's jet lp tokens
            pub vault_jet_lp_token: Box<Account<'info, TokenAccount>>,
            # [account (executable , address = jet :: ID ,)]
            pub jet_program: AccountInfo<'info>,
            #[account(mut)]
            pub jet_market: AccountInfo<'info>,
            pub jet_market_authority: AccountInfo<'info>,
            #[account(mut)]
            pub jet_reserve: AccountLoader<'info, jet::state::Reserve>,
            #[account(mut)]
            pub jet_fee_note_vault: AccountInfo<'info>,
            #[account(mut)]
            pub jet_deposit_note_mint: AccountInfo<'info>,
            pub jet_pyth: AccountInfo<'info>,
            pub token_program: Program<'info, Token>,
            pub clock: Sysvar<'info, Clock>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for RefreshJet<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_jet_lp_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_program: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_market: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_market_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_reserve: anchor_lang::AccountLoader<jet::state::Reserve> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_fee_note_vault: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_deposit_note_mint: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_pyth: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: Sysvar<Clock> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.vault_jet_lp_token != vault_jet_lp_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.jet_reserve != jet_reserve.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !jet_program.to_account_info().executable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintExecutable.into());
                }
                if jet_program.to_account_info().key != &jet::ID {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintAddress.into());
                }
                if !jet_market.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !jet_reserve.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !jet_fee_note_vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !jet_deposit_note_mint.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(RefreshJet {
                    vault,
                    vault_jet_lp_token,
                    jet_program,
                    jet_market,
                    jet_market_authority,
                    jet_reserve,
                    jet_fee_note_vault,
                    jet_deposit_note_mint,
                    jet_pyth,
                    token_program,
                    clock,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for RefreshJet<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_jet_lp_token.to_account_infos());
                account_infos.extend(self.jet_program.to_account_infos());
                account_infos.extend(self.jet_market.to_account_infos());
                account_infos.extend(self.jet_market_authority.to_account_infos());
                account_infos.extend(self.jet_reserve.to_account_infos());
                account_infos.extend(self.jet_fee_note_vault.to_account_infos());
                account_infos.extend(self.jet_deposit_note_mint.to_account_infos());
                account_infos.extend(self.jet_pyth.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for RefreshJet<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_jet_lp_token.to_account_metas(None));
                account_metas.extend(self.jet_program.to_account_metas(None));
                account_metas.extend(self.jet_market.to_account_metas(None));
                account_metas.extend(self.jet_market_authority.to_account_metas(None));
                account_metas.extend(self.jet_reserve.to_account_metas(None));
                account_metas.extend(self.jet_fee_note_vault.to_account_metas(None));
                account_metas.extend(self.jet_deposit_note_mint.to_account_metas(None));
                account_metas.extend(self.jet_pyth.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for RefreshJet<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.jet_market, program_id)?;
                anchor_lang::AccountsExit::exit(&self.jet_reserve, program_id)?;
                anchor_lang::AccountsExit::exit(&self.jet_fee_note_vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.jet_deposit_note_mint, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_refresh_jet {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct RefreshJet {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_jet_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_market: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_market_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_fee_note_vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_deposit_note_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_pyth: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for RefreshJet
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_jet_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_market, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_market_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_fee_note_vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_deposit_note_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_pyth, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for RefreshJet {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_jet_lp_token,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.jet_program,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.jet_market,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.jet_market_authority,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.jet_reserve,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.jet_fee_note_vault,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.jet_deposit_note_mint,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.jet_pyth,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_refresh_jet {
            use super::*;
            pub struct RefreshJet<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_jet_lp_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_market: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_market_authority:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_fee_note_vault:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_deposit_note_mint:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_pyth: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for RefreshJet<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_jet_lp_token),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.jet_program),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.jet_market),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.jet_market_authority),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.jet_reserve),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.jet_fee_note_vault),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.jet_deposit_note_mint),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.jet_pyth),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for RefreshJet<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_jet_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_market,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_market_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_fee_note_vault,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_deposit_note_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.jet_pyth));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos
                }
            }
        }
        impl<'info> RefreshJet<'info> {
            fn jet_refresh_reserve_context(
                &self,
            ) -> CpiContext<'_, '_, '_, 'info, jet::cpi::accounts::RefreshReserve<'info>>
            {
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
                ::solana_program::log::sol_log("Refreshing jet");
                jet::cpi::refresh_reserve(self.jet_refresh_reserve_context())?;
                let jet_reserve = self.jet_reserve.load()?;
                let jet_exchange_rate = jet_reserve.deposit_note_exchange_rate(
                    self.clock.slot,
                    jet_reserve.total_deposits(),
                    jet_reserve.total_deposit_notes(),
                );
                let jet_value = (jet_exchange_rate * self.vault_jet_lp_token.amount).as_u64(0);
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Value: "],
                        &match (&jet_value,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
                self.vault.actual_allocations[Provider::Jet].update(jet_value, self.clock.slot);
                Ok(())
            }
        }
    }
    pub mod port {
        use std::ops::{Deref, DerefMut};
        use anchor_lang::prelude::*;
        use anchor_spl::token::{Token, TokenAccount};
        use port_anchor_adaptor::{port_lending_id, PortReserve};
        use port_variable_rate_lending_instructions::state::Reserve;
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
        pub struct PortAccounts<'info> {
            /// Vault state account
            /// Checks that the accounts passed in are correct
            # [account (mut , has_one = vault_authority , has_one = vault_reserve_token , has_one = vault_port_lp_token , has_one = port_reserve ,)]
            pub vault: Box<Account<'info, Vault>>,
            /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
            pub vault_authority: AccountInfo<'info>,
            /// Token account for the vault's reserve tokens
            #[account(mut)]
            pub vault_reserve_token: Box<Account<'info, TokenAccount>>,
            /// Token account for the vault's port lp tokens
            #[account(mut)]
            pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,
            # [account (executable , address = port_lending_id () ,)]
            pub port_program: AccountInfo<'info>,
            pub port_market_authority: AccountInfo<'info>,
            pub port_market: AccountInfo<'info>,
            #[account(mut)]
            pub port_reserve: Box<Account<'info, PortReserve>>,
            #[account(mut)]
            pub port_lp_mint: AccountInfo<'info>,
            #[account(mut)]
            pub port_reserve_token: AccountInfo<'info>,
            pub clock: Sysvar<'info, Clock>,
            pub token_program: Program<'info, Token>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for PortAccounts<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_reserve_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_port_lp_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let port_program: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let port_market_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let port_market: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let port_reserve: Box<anchor_lang::Account<PortReserve>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let port_lp_mint: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let port_reserve_token: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: Sysvar<Clock> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.vault_authority != vault_authority.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_reserve_token != vault_reserve_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_port_lp_token != vault_port_lp_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.port_reserve != port_reserve.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !vault_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !vault_port_lp_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !port_program.to_account_info().executable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintExecutable.into());
                }
                if port_program.to_account_info().key != &port_lending_id() {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintAddress.into());
                }
                if !port_reserve.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !port_lp_mint.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !port_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(PortAccounts {
                    vault,
                    vault_authority,
                    vault_reserve_token,
                    vault_port_lp_token,
                    port_program,
                    port_market_authority,
                    port_market,
                    port_reserve,
                    port_lp_mint,
                    port_reserve_token,
                    clock,
                    token_program,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for PortAccounts<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_authority.to_account_infos());
                account_infos.extend(self.vault_reserve_token.to_account_infos());
                account_infos.extend(self.vault_port_lp_token.to_account_infos());
                account_infos.extend(self.port_program.to_account_infos());
                account_infos.extend(self.port_market_authority.to_account_infos());
                account_infos.extend(self.port_market.to_account_infos());
                account_infos.extend(self.port_reserve.to_account_infos());
                account_infos.extend(self.port_lp_mint.to_account_infos());
                account_infos.extend(self.port_reserve_token.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for PortAccounts<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_authority.to_account_metas(None));
                account_metas.extend(self.vault_reserve_token.to_account_metas(None));
                account_metas.extend(self.vault_port_lp_token.to_account_metas(None));
                account_metas.extend(self.port_program.to_account_metas(None));
                account_metas.extend(self.port_market_authority.to_account_metas(None));
                account_metas.extend(self.port_market.to_account_metas(None));
                account_metas.extend(self.port_reserve.to_account_metas(None));
                account_metas.extend(self.port_lp_mint.to_account_metas(None));
                account_metas.extend(self.port_reserve_token.to_account_metas(None));
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for PortAccounts<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_reserve_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_port_lp_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.port_reserve, program_id)?;
                anchor_lang::AccountsExit::exit(&self.port_lp_mint, program_id)?;
                anchor_lang::AccountsExit::exit(&self.port_reserve_token, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_port_accounts {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct PortAccounts {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_port_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_market_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_market: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_lp_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for PortAccounts
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_port_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_market_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_market, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_lp_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for PortAccounts {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_authority,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_reserve_token,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_port_lp_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.port_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.port_market_authority,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.port_market,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.port_reserve,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.port_lp_mint,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.port_reserve_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_port_accounts {
            use super::*;
            pub struct PortAccounts<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_port_lp_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_market_authority:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_market: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_lp_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for PortAccounts<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_authority),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_reserve_token),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_port_lp_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.port_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.port_market_authority),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.port_market),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.port_reserve),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.port_lp_mint),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.port_reserve_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for PortAccounts<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_port_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_market_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_market,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_lp_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos
                }
            }
        }
        impl crate::instructions::reconcile::HasVault for PortAccounts<'_> {
            fn vault(&self) -> &Vault {
                self.vault.deref()
            }
            fn vault_mut(&mut self) -> &mut Vault {
                self.vault.deref_mut()
            }
        }
        impl<'info> LendingMarket for PortAccounts<'info> {
            fn deposit(&self, amount: u64) -> ProgramResult {
                let context = CpiContext::new(
                    self.port_program.clone(),
                    port_anchor_adaptor::Deposit {
                        source_liquidity: self.vault_reserve_token.to_account_info(),
                        destination_collateral: self.vault_port_lp_token.to_account_info(),
                        reserve: self.port_reserve.to_account_info(),
                        reserve_collateral_mint: self.port_lp_mint.clone(),
                        reserve_liquidity_supply: self.port_reserve_token.clone(),
                        lending_market: self.port_market.clone(),
                        lending_market_authority: self.port_market_authority.clone(),
                        transfer_authority: self.vault_authority.clone(),
                        clock: self.clock.to_account_info(),
                        token_program: self.token_program.to_account_info(),
                    },
                );
                match amount {
                    0 => Ok(()),
                    _ => port_anchor_adaptor::deposit_reserve(
                        context.with_signer(&[&self.vault.authority_seeds()]),
                        amount,
                    ),
                }
            }
            fn redeem(&self, amount: u64) -> ProgramResult {
                let context = CpiContext::new(
                    self.port_program.clone(),
                    port_anchor_adaptor::Redeem {
                        source_collateral: self.vault_port_lp_token.to_account_info(),
                        destination_liquidity: self.vault_reserve_token.to_account_info(),
                        reserve: self.port_reserve.to_account_info(),
                        reserve_collateral_mint: self.port_lp_mint.clone(),
                        reserve_liquidity_supply: self.port_reserve_token.clone(),
                        lending_market: self.port_market.clone(),
                        lending_market_authority: self.port_market_authority.clone(),
                        transfer_authority: self.vault_authority.clone(),
                        clock: self.clock.to_account_info(),
                        token_program: self.token_program.to_account_info(),
                    },
                );
                match amount {
                    0 => Ok(()),
                    _ => port_anchor_adaptor::redeem(
                        context.with_signer(&[&self.vault.authority_seeds()]),
                        amount,
                    ),
                }
            }
            fn convert_amount_reserve_to_lp(&self, amount: u64) -> Result<u64, ProgramError> {
                let exchange_rate = self.port_reserve.collateral_exchange_rate()?;
                exchange_rate.liquidity_to_collateral(amount)
            }
            fn convert_amount_lp_to_reserve(&self, amount: u64) -> Result<u64, ProgramError> {
                let exchange_rate = self.port_reserve.collateral_exchange_rate()?;
                exchange_rate.collateral_to_liquidity(amount)
            }
            fn reserve_tokens_in_vault(&self) -> u64 {
                self.vault_reserve_token.amount
            }
            fn lp_tokens_in_vault(&self) -> u64 {
                self.vault_port_lp_token.amount
            }
            fn provider(&self) -> Provider {
                Provider::Port
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
                reserve.liquidity.available_amount = reserve
                    .liquidity
                    .available_amount
                    .checked_add(allocation)
                    .ok_or(ErrorCode::OverflowError)?;
                Ok(reserve)
            }
        }
        # [instruction (bump : u8)]
        pub struct InitializePort<'info> {
            # [account (mut , has_one = owner , has_one = vault_authority ,)]
            pub vault: Box<Account<'info, Vault>>,
            pub vault_authority: AccountInfo<'info>,
            /// Token account for the vault's port lp tokens
            # [account (init , payer = payer , seeds = [vault . key () . as_ref () , port_lp_token_mint . key () . as_ref ()] , bump = bump , token :: authority = vault_authority , token :: mint = port_lp_token_mint ,)]
            pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,
            /// Mint of the port lp token
            pub port_lp_token_mint: AccountInfo<'info>,
            pub port_reserve: Box<Account<'info, PortReserve>>,
            pub owner: Signer<'info>,
            #[account(mut)]
            pub payer: Signer<'info>,
            pub token_program: Program<'info, Token>,
            pub system_program: Program<'info, System>,
            pub rent: Sysvar<'info, Rent>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for InitializePort<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let mut ix_data = ix_data;
                struct __Args {
                    bump: u8,
                }
                impl borsh::ser::BorshSerialize for __Args
                where
                    u8: borsh::ser::BorshSerialize,
                {
                    fn serialize<W: borsh::maybestd::io::Write>(
                        &self,
                        writer: &mut W,
                    ) -> ::core::result::Result<(), borsh::maybestd::io::Error>
                    {
                        borsh::BorshSerialize::serialize(&self.bump, writer)?;
                        Ok(())
                    }
                }
                impl borsh::de::BorshDeserialize for __Args
                where
                    u8: borsh::BorshDeserialize,
                {
                    fn deserialize(
                        buf: &mut &[u8],
                    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error>
                    {
                        Ok(Self {
                            bump: borsh::BorshDeserialize::deserialize(buf)?,
                        })
                    }
                }
                let __Args { bump } = __Args::deserialize(&mut ix_data)
                    .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_port_lp_token = &accounts[0];
                *accounts = &accounts[1..];
                let port_lp_token_mint: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let port_reserve: Box<anchor_lang::Account<PortReserve>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let owner: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let payer: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let system_program: anchor_lang::Program<System> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let rent: Sysvar<Rent> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let __anchor_rent = Rent::get()?;
                let vault_port_lp_token: Box<anchor_lang::Account<TokenAccount>> = {
                    if !false
                        || vault_port_lp_token.to_account_info().owner
                            == &anchor_lang::solana_program::system_program::ID
                    {
                        let payer = payer.to_account_info();
                        let __current_lamports = vault_port_lp_token.to_account_info().lamports();
                        if __current_lamports == 0 {
                            let lamports =
                                __anchor_rent.minimum_balance(anchor_spl::token::TokenAccount::LEN);
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::create_account(
                                    payer.to_account_info().key,
                                    vault_port_lp_token.to_account_info().key,
                                    lamports,
                                    anchor_spl::token::TokenAccount::LEN as u64,
                                    token_program.to_account_info().key,
                                ),
                                &[
                                    payer.to_account_info(),
                                    vault_port_lp_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    port_lp_token_mint.key().as_ref(),
                                    &[bump][..],
                                ][..]],
                            )?;
                        } else {
                            let required_lamports = __anchor_rent
                                .minimum_balance(anchor_spl::token::TokenAccount::LEN)
                                .max(1)
                                .saturating_sub(__current_lamports);
                            if required_lamports > 0 {
                                anchor_lang::solana_program::program::invoke(
                                    &anchor_lang::solana_program::system_instruction::transfer(
                                        payer.to_account_info().key,
                                        vault_port_lp_token.to_account_info().key,
                                        required_lamports,
                                    ),
                                    &[
                                        payer.to_account_info(),
                                        vault_port_lp_token.to_account_info(),
                                        system_program.to_account_info(),
                                    ],
                                )?;
                            }
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::allocate(
                                    vault_port_lp_token.to_account_info().key,
                                    anchor_spl::token::TokenAccount::LEN as u64,
                                ),
                                &[
                                    vault_port_lp_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    port_lp_token_mint.key().as_ref(),
                                    &[bump][..],
                                ][..]],
                            )?;
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::assign(
                                    vault_port_lp_token.to_account_info().key,
                                    token_program.to_account_info().key,
                                ),
                                &[
                                    vault_port_lp_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    port_lp_token_mint.key().as_ref(),
                                    &[bump][..],
                                ][..]],
                            )?;
                        }
                        let cpi_program = token_program.to_account_info();
                        let accounts = anchor_spl::token::InitializeAccount {
                            account: vault_port_lp_token.to_account_info(),
                            mint: port_lp_token_mint.to_account_info(),
                            authority: vault_authority.to_account_info(),
                            rent: rent.to_account_info(),
                        };
                        let cpi_ctx = CpiContext::new(cpi_program, accounts);
                        anchor_spl::token::initialize_account(cpi_ctx)?;
                    }
                    let pa: Box<anchor_lang::Account<TokenAccount>> = Box::new(
                        anchor_lang::Account::try_from_unchecked(&vault_port_lp_token)?,
                    );
                    pa
                };
                let (__program_signer, __bump) =
                    anchor_lang::solana_program::pubkey::Pubkey::find_program_address(
                        &[vault.key().as_ref(), port_lp_token_mint.key().as_ref()],
                        program_id,
                    );
                if vault_port_lp_token.to_account_info().key != &__program_signer {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if __bump != bump {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if !vault_port_lp_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !__anchor_rent.is_exempt(
                    vault_port_lp_token.to_account_info().lamports(),
                    vault_port_lp_token.to_account_info().try_data_len()?,
                ) {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
                }
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.owner != owner.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_authority != vault_authority.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !payer.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(InitializePort {
                    vault,
                    vault_authority,
                    vault_port_lp_token,
                    port_lp_token_mint,
                    port_reserve,
                    owner,
                    payer,
                    token_program,
                    system_program,
                    rent,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for InitializePort<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_authority.to_account_infos());
                account_infos.extend(self.vault_port_lp_token.to_account_infos());
                account_infos.extend(self.port_lp_token_mint.to_account_infos());
                account_infos.extend(self.port_reserve.to_account_infos());
                account_infos.extend(self.owner.to_account_infos());
                account_infos.extend(self.payer.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos.extend(self.system_program.to_account_infos());
                account_infos.extend(self.rent.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for InitializePort<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_authority.to_account_metas(None));
                account_metas.extend(self.vault_port_lp_token.to_account_metas(None));
                account_metas.extend(self.port_lp_token_mint.to_account_metas(None));
                account_metas.extend(self.port_reserve.to_account_metas(None));
                account_metas.extend(self.owner.to_account_metas(None));
                account_metas.extend(self.payer.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas.extend(self.system_program.to_account_metas(None));
                account_metas.extend(self.rent.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for InitializePort<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_port_lp_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.payer, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_initialize_port {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct InitializePort {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_port_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_lp_token_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub owner: anchor_lang::solana_program::pubkey::Pubkey,
                pub payer: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub rent: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for InitializePort
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_port_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_lp_token_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.owner, writer)?;
                    borsh::BorshSerialize::serialize(&self.payer, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.rent, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for InitializePort {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_authority,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_port_lp_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.port_lp_token_mint,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.port_reserve,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.owner, true,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.payer, true,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.system_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.rent, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_initialize_port {
            use super::*;
            pub struct InitializePort<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_port_lp_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_lp_token_mint:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub rent: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for InitializePort<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_authority),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_port_lp_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.port_lp_token_mint),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.port_reserve),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.owner),
                            true,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.payer),
                        true,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.system_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.rent),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for InitializePort<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_port_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_lp_token_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.owner));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.payer));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.system_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.rent));
                    account_infos
                }
            }
        }
        impl<'info> YieldSourceInitializer<'info> for InitializePort<'info> {
            fn initialize_yield_source(&mut self) -> ProgramResult {
                self.vault.port_reserve = self.port_reserve.key();
                self.vault.vault_port_lp_token = self.vault_port_lp_token.key();
                self.vault
                    .set_yield_source_flag(YieldSourceFlags::PORT, true)?;
                Ok(())
            }
        }
        pub struct RefreshPort<'info> {
            /// Vault state account
            /// Checks that the accounts passed in are correct
            # [account (mut , has_one = vault_port_lp_token , has_one = port_reserve ,)]
            pub vault: Box<Account<'info, Vault>>,
            /// Token account for the vault's port lp tokens
            pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,
            # [account (executable , address = port_lending_id () ,)]
            pub port_program: AccountInfo<'info>,
            #[account(mut)]
            pub port_reserve: Box<Account<'info, PortReserve>>,
            pub clock: Sysvar<'info, Clock>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for RefreshPort<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_port_lp_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let port_program: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let port_reserve: Box<anchor_lang::Account<PortReserve>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: Sysvar<Clock> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.vault_port_lp_token != vault_port_lp_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.port_reserve != port_reserve.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !port_program.to_account_info().executable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintExecutable.into());
                }
                if port_program.to_account_info().key != &port_lending_id() {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintAddress.into());
                }
                if !port_reserve.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(RefreshPort {
                    vault,
                    vault_port_lp_token,
                    port_program,
                    port_reserve,
                    clock,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for RefreshPort<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_port_lp_token.to_account_infos());
                account_infos.extend(self.port_program.to_account_infos());
                account_infos.extend(self.port_reserve.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for RefreshPort<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_port_lp_token.to_account_metas(None));
                account_metas.extend(self.port_program.to_account_metas(None));
                account_metas.extend(self.port_reserve.to_account_metas(None));
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for RefreshPort<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.port_reserve, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_refresh_port {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct RefreshPort {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_port_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for RefreshPort
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_port_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for RefreshPort {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_port_lp_token,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.port_program,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.port_reserve,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_refresh_port {
            use super::*;
            pub struct RefreshPort<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_port_lp_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for RefreshPort<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_port_lp_token),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.port_program),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.port_reserve),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for RefreshPort<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_port_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos
                }
            }
        }
        impl<'info> RefreshPort<'info> {
            fn port_refresh_reserve_context(
                &self,
                remaining_accounts: &[AccountInfo<'info>],
            ) -> CpiContext<'_, '_, '_, 'info, port_anchor_adaptor::RefreshReserve<'info>>
            {
                CpiContext::new(
                    self.port_program.clone(),
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
            ) -> ProgramResult {
                if self
                    .vault
                    .get_yield_source_flags()
                    .contains(YieldSourceFlags::PORT)
                {
                    port_anchor_adaptor::refresh_port_reserve(
                        self.port_refresh_reserve_context(remaining_accounts),
                    )?;
                    ::solana_program::log::sol_log("Refreshing port");
                    let port_exchange_rate = self.port_reserve.collateral_exchange_rate()?;
                    let port_value = port_exchange_rate
                        .collateral_to_liquidity(self.vault_port_lp_token.amount)?;
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Refresh port reserve token value: "],
                            &match (&port_value,) {
                                _args => [::core::fmt::ArgumentV1::new(
                                    _args.0,
                                    ::core::fmt::Display::fmt,
                                )],
                            },
                        ));
                        res
                    });
                    self.vault.actual_allocations[Provider::Port]
                        .update(port_value, self.clock.slot);
                }
                Ok(())
            }
        }
    }
    pub mod solend {
        use std::{
            io::Write,
            ops::{Deref, DerefMut},
        };
        use anchor_lang::{prelude::*, solana_program};
        use anchor_spl::token::{Token, TokenAccount};
        use solana_maths::Rate;
        use spl_token_lending::state::Reserve;
        use crate::{
            impl_has_vault,
            init_yield_source::YieldSourceInitializer,
            reconcile::LendingMarket,
            refresh::Refresher,
            reserves::{Provider, ReserveAccessor},
            state::{Vault, YieldSourceFlags},
        };
        pub struct SolendAccounts<'info> {
            /// Vault state account
            /// Checks that the accounts passed in are correct
            # [account (mut , has_one = vault_authority , has_one = vault_reserve_token , has_one = vault_solend_lp_token , has_one = solend_reserve ,)]
            pub vault: Box<Account<'info, Vault>>,
            /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
            pub vault_authority: AccountInfo<'info>,
            /// Token account for the vault's reserve tokens
            #[account(mut)]
            pub vault_reserve_token: Box<Account<'info, TokenAccount>>,
            /// Token account for the vault's solend lp tokens
            #[account(mut)]
            pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,
            # [account (executable , address = spl_token_lending :: ID ,)]
            pub solend_program: AccountInfo<'info>,
            pub solend_market_authority: AccountInfo<'info>,
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
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for SolendAccounts<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_reserve_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_solend_lp_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_program: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_market_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_market: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_reserve: Box<anchor_lang::Account<SolendReserve>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_lp_mint: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_reserve_token: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: Sysvar<Clock> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.vault_authority != vault_authority.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_reserve_token != vault_reserve_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_solend_lp_token != vault_solend_lp_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.solend_reserve != solend_reserve.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !vault_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !vault_solend_lp_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !solend_program.to_account_info().executable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintExecutable.into());
                }
                if solend_program.to_account_info().key != &spl_token_lending::ID {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintAddress.into());
                }
                if !solend_reserve.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !solend_lp_mint.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !solend_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(SolendAccounts {
                    vault,
                    vault_authority,
                    vault_reserve_token,
                    vault_solend_lp_token,
                    solend_program,
                    solend_market_authority,
                    solend_market,
                    solend_reserve,
                    solend_lp_mint,
                    solend_reserve_token,
                    clock,
                    token_program,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for SolendAccounts<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_authority.to_account_infos());
                account_infos.extend(self.vault_reserve_token.to_account_infos());
                account_infos.extend(self.vault_solend_lp_token.to_account_infos());
                account_infos.extend(self.solend_program.to_account_infos());
                account_infos.extend(self.solend_market_authority.to_account_infos());
                account_infos.extend(self.solend_market.to_account_infos());
                account_infos.extend(self.solend_reserve.to_account_infos());
                account_infos.extend(self.solend_lp_mint.to_account_infos());
                account_infos.extend(self.solend_reserve_token.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for SolendAccounts<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_authority.to_account_metas(None));
                account_metas.extend(self.vault_reserve_token.to_account_metas(None));
                account_metas.extend(self.vault_solend_lp_token.to_account_metas(None));
                account_metas.extend(self.solend_program.to_account_metas(None));
                account_metas.extend(self.solend_market_authority.to_account_metas(None));
                account_metas.extend(self.solend_market.to_account_metas(None));
                account_metas.extend(self.solend_reserve.to_account_metas(None));
                account_metas.extend(self.solend_lp_mint.to_account_metas(None));
                account_metas.extend(self.solend_reserve_token.to_account_metas(None));
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for SolendAccounts<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_reserve_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_solend_lp_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.solend_reserve, program_id)?;
                anchor_lang::AccountsExit::exit(&self.solend_lp_mint, program_id)?;
                anchor_lang::AccountsExit::exit(&self.solend_reserve_token, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_solend_accounts {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct SolendAccounts {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_solend_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_market_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_market: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_lp_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for SolendAccounts
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_solend_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_market_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_market, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_lp_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for SolendAccounts {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_authority,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_reserve_token,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_solend_lp_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.solend_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.solend_market_authority,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.solend_market,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.solend_reserve,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.solend_lp_mint,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.solend_reserve_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_solend_accounts {
            use super::*;
            pub struct SolendAccounts<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_solend_lp_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_market_authority:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_market: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_lp_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for SolendAccounts<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_authority),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_reserve_token),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_solend_lp_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.solend_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.solend_market_authority),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.solend_market),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.solend_reserve),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.solend_lp_mint),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.solend_reserve_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for SolendAccounts<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_solend_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_market_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_market,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_lp_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos
                }
            }
        }
        impl crate::instructions::reconcile::HasVault for SolendAccounts<'_> {
            fn vault(&self) -> &Vault {
                self.vault.deref()
            }
            fn vault_mut(&mut self) -> &mut Vault {
                self.vault.deref_mut()
            }
        }
        impl<'info> LendingMarket for SolendAccounts<'info> {
            fn deposit(&self, amount: u64) -> ProgramResult {
                let context = CpiContext::new(
                    self.solend_program.clone(),
                    DepositReserveLiquidity {
                        lending_program: self.solend_program.clone(),
                        source_liquidity: self.vault_reserve_token.to_account_info(),
                        destination_collateral_account: self
                            .vault_solend_lp_token
                            .to_account_info(),
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
                exchange_rate.liquidity_to_collateral(amount)
            }
            fn convert_amount_lp_to_reserve(&self, amount: u64) -> Result<u64, ProgramError> {
                let exchange_rate = self.solend_reserve.collateral_exchange_rate()?;
                exchange_rate.collateral_to_liquidity(amount)
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
        pub struct DepositReserveLiquidity<'info> {
            pub lending_program: AccountInfo<'info>,
            pub source_liquidity: AccountInfo<'info>,
            pub destination_collateral_account: AccountInfo<'info>,
            pub reserve: AccountInfo<'info>,
            pub reserve_collateral_mint: AccountInfo<'info>,
            pub reserve_liquidity_supply: AccountInfo<'info>,
            pub lending_market: AccountInfo<'info>,
            pub lending_market_authority: AccountInfo<'info>,
            pub transfer_authority: AccountInfo<'info>,
            pub clock: AccountInfo<'info>,
            pub token_program_id: AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for DepositReserveLiquidity<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let lending_program: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let source_liquidity: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let destination_collateral_account: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let reserve: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let reserve_collateral_mint: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let reserve_liquidity_supply: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let lending_market: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let lending_market_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let transfer_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program_id: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                Ok(DepositReserveLiquidity {
                    lending_program,
                    source_liquidity,
                    destination_collateral_account,
                    reserve,
                    reserve_collateral_mint,
                    reserve_liquidity_supply,
                    lending_market,
                    lending_market_authority,
                    transfer_authority,
                    clock,
                    token_program_id,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for DepositReserveLiquidity<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.lending_program.to_account_infos());
                account_infos.extend(self.source_liquidity.to_account_infos());
                account_infos.extend(self.destination_collateral_account.to_account_infos());
                account_infos.extend(self.reserve.to_account_infos());
                account_infos.extend(self.reserve_collateral_mint.to_account_infos());
                account_infos.extend(self.reserve_liquidity_supply.to_account_infos());
                account_infos.extend(self.lending_market.to_account_infos());
                account_infos.extend(self.lending_market_authority.to_account_infos());
                account_infos.extend(self.transfer_authority.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos.extend(self.token_program_id.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for DepositReserveLiquidity<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.lending_program.to_account_metas(None));
                account_metas.extend(self.source_liquidity.to_account_metas(None));
                account_metas.extend(self.destination_collateral_account.to_account_metas(None));
                account_metas.extend(self.reserve.to_account_metas(None));
                account_metas.extend(self.reserve_collateral_mint.to_account_metas(None));
                account_metas.extend(self.reserve_liquidity_supply.to_account_metas(None));
                account_metas.extend(self.lending_market.to_account_metas(None));
                account_metas.extend(self.lending_market_authority.to_account_metas(None));
                account_metas.extend(self.transfer_authority.to_account_metas(None));
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas.extend(self.token_program_id.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for DepositReserveLiquidity<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_deposit_reserve_liquidity {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct DepositReserveLiquidity {
                pub lending_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub source_liquidity: anchor_lang::solana_program::pubkey::Pubkey,
                pub destination_collateral_account: anchor_lang::solana_program::pubkey::Pubkey,
                pub reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub reserve_collateral_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub reserve_liquidity_supply: anchor_lang::solana_program::pubkey::Pubkey,
                pub lending_market: anchor_lang::solana_program::pubkey::Pubkey,
                pub lending_market_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub transfer_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program_id: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for DepositReserveLiquidity
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.lending_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.source_liquidity, writer)?;
                    borsh::BorshSerialize::serialize(&self.destination_collateral_account, writer)?;
                    borsh::BorshSerialize::serialize(&self.reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.reserve_collateral_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.reserve_liquidity_supply, writer)?;
                    borsh::BorshSerialize::serialize(&self.lending_market, writer)?;
                    borsh::BorshSerialize::serialize(&self.lending_market_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.transfer_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program_id, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for DepositReserveLiquidity {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.lending_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.source_liquidity,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.destination_collateral_account,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.reserve,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.reserve_collateral_mint,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.reserve_liquidity_supply,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.lending_market,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.lending_market_authority,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.transfer_authority,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program_id,
                            false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_deposit_reserve_liquidity {
            use super::*;
            pub struct DepositReserveLiquidity<'info> {
                pub lending_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub source_liquidity: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub destination_collateral_account:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub reserve_collateral_mint:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub reserve_liquidity_supply:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub lending_market: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub lending_market_authority:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub transfer_authority:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program_id: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for DepositReserveLiquidity<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.lending_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.source_liquidity),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.destination_collateral_account),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.reserve),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.reserve_collateral_mint),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.reserve_liquidity_supply),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.lending_market),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.lending_market_authority),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.transfer_authority),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program_id),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for DepositReserveLiquidity<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lending_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.source_liquidity,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.destination_collateral_account,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.reserve));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.reserve_collateral_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.reserve_liquidity_supply,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lending_market,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lending_market_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.transfer_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program_id,
                    ));
                    account_infos
                }
            }
        }
        pub struct RedeemReserveCollateral<'info> {
            pub lending_program: AccountInfo<'info>,
            pub source_collateral: AccountInfo<'info>,
            pub destination_liquidity: AccountInfo<'info>,
            pub reserve: AccountInfo<'info>,
            pub reserve_collateral_mint: AccountInfo<'info>,
            pub reserve_liquidity_supply: AccountInfo<'info>,
            pub lending_market: AccountInfo<'info>,
            pub lending_market_authority: AccountInfo<'info>,
            pub transfer_authority: AccountInfo<'info>,
            pub clock: AccountInfo<'info>,
            pub token_program_id: AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for RedeemReserveCollateral<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let lending_program: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let source_collateral: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let destination_liquidity: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let reserve: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let reserve_collateral_mint: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let reserve_liquidity_supply: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let lending_market: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let lending_market_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let transfer_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program_id: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                Ok(RedeemReserveCollateral {
                    lending_program,
                    source_collateral,
                    destination_liquidity,
                    reserve,
                    reserve_collateral_mint,
                    reserve_liquidity_supply,
                    lending_market,
                    lending_market_authority,
                    transfer_authority,
                    clock,
                    token_program_id,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for RedeemReserveCollateral<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.lending_program.to_account_infos());
                account_infos.extend(self.source_collateral.to_account_infos());
                account_infos.extend(self.destination_liquidity.to_account_infos());
                account_infos.extend(self.reserve.to_account_infos());
                account_infos.extend(self.reserve_collateral_mint.to_account_infos());
                account_infos.extend(self.reserve_liquidity_supply.to_account_infos());
                account_infos.extend(self.lending_market.to_account_infos());
                account_infos.extend(self.lending_market_authority.to_account_infos());
                account_infos.extend(self.transfer_authority.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos.extend(self.token_program_id.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for RedeemReserveCollateral<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.lending_program.to_account_metas(None));
                account_metas.extend(self.source_collateral.to_account_metas(None));
                account_metas.extend(self.destination_liquidity.to_account_metas(None));
                account_metas.extend(self.reserve.to_account_metas(None));
                account_metas.extend(self.reserve_collateral_mint.to_account_metas(None));
                account_metas.extend(self.reserve_liquidity_supply.to_account_metas(None));
                account_metas.extend(self.lending_market.to_account_metas(None));
                account_metas.extend(self.lending_market_authority.to_account_metas(None));
                account_metas.extend(self.transfer_authority.to_account_metas(None));
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas.extend(self.token_program_id.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for RedeemReserveCollateral<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_redeem_reserve_collateral {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct RedeemReserveCollateral {
                pub lending_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub source_collateral: anchor_lang::solana_program::pubkey::Pubkey,
                pub destination_liquidity: anchor_lang::solana_program::pubkey::Pubkey,
                pub reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub reserve_collateral_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub reserve_liquidity_supply: anchor_lang::solana_program::pubkey::Pubkey,
                pub lending_market: anchor_lang::solana_program::pubkey::Pubkey,
                pub lending_market_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub transfer_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program_id: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for RedeemReserveCollateral
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.lending_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.source_collateral, writer)?;
                    borsh::BorshSerialize::serialize(&self.destination_liquidity, writer)?;
                    borsh::BorshSerialize::serialize(&self.reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.reserve_collateral_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.reserve_liquidity_supply, writer)?;
                    borsh::BorshSerialize::serialize(&self.lending_market, writer)?;
                    borsh::BorshSerialize::serialize(&self.lending_market_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.transfer_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program_id, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for RedeemReserveCollateral {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.lending_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.source_collateral,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.destination_liquidity,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.reserve,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.reserve_collateral_mint,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.reserve_liquidity_supply,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.lending_market,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.lending_market_authority,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.transfer_authority,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program_id,
                            false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_redeem_reserve_collateral {
            use super::*;
            pub struct RedeemReserveCollateral<'info> {
                pub lending_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub source_collateral:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub destination_liquidity:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub reserve_collateral_mint:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub reserve_liquidity_supply:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub lending_market: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub lending_market_authority:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub transfer_authority:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program_id: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for RedeemReserveCollateral<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.lending_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.source_collateral),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.destination_liquidity),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.reserve),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.reserve_collateral_mint),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.reserve_liquidity_supply),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.lending_market),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.lending_market_authority),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.transfer_authority),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program_id),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for RedeemReserveCollateral<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lending_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.source_collateral,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.destination_liquidity,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.reserve));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.reserve_collateral_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.reserve_liquidity_supply,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lending_market,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lending_market_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.transfer_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program_id,
                    ));
                    account_infos
                }
            }
        }
        pub struct RefreshReserve<'info> {
            pub lending_program: AccountInfo<'info>,
            pub reserve: AccountInfo<'info>,
            pub pyth_reserve_liquidity_oracle: AccountInfo<'info>,
            pub switchboard_reserve_liquidity_oracle: AccountInfo<'info>,
            pub clock: AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for RefreshReserve<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let lending_program: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let reserve: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let pyth_reserve_liquidity_oracle: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let switchboard_reserve_liquidity_oracle: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                Ok(RefreshReserve {
                    lending_program,
                    reserve,
                    pyth_reserve_liquidity_oracle,
                    switchboard_reserve_liquidity_oracle,
                    clock,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for RefreshReserve<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.lending_program.to_account_infos());
                account_infos.extend(self.reserve.to_account_infos());
                account_infos.extend(self.pyth_reserve_liquidity_oracle.to_account_infos());
                account_infos.extend(self.switchboard_reserve_liquidity_oracle.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for RefreshReserve<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.lending_program.to_account_metas(None));
                account_metas.extend(self.reserve.to_account_metas(None));
                account_metas.extend(self.pyth_reserve_liquidity_oracle.to_account_metas(None));
                account_metas.extend(
                    self.switchboard_reserve_liquidity_oracle
                        .to_account_metas(None),
                );
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for RefreshReserve<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_refresh_reserve {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct RefreshReserve {
                pub lending_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub pyth_reserve_liquidity_oracle: anchor_lang::solana_program::pubkey::Pubkey,
                pub switchboard_reserve_liquidity_oracle:
                    anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for RefreshReserve
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.lending_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.pyth_reserve_liquidity_oracle, writer)?;
                    borsh::BorshSerialize::serialize(
                        &self.switchboard_reserve_liquidity_oracle,
                        writer,
                    )?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for RefreshReserve {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.lending_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.reserve,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.pyth_reserve_liquidity_oracle,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.switchboard_reserve_liquidity_oracle,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_refresh_reserve {
            use super::*;
            pub struct RefreshReserve<'info> {
                pub lending_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub pyth_reserve_liquidity_oracle:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub switchboard_reserve_liquidity_oracle:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for RefreshReserve<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.lending_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.reserve),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.pyth_reserve_liquidity_oracle),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.switchboard_reserve_liquidity_oracle),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for RefreshReserve<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lending_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.reserve));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.pyth_reserve_liquidity_oracle,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.switchboard_reserve_liquidity_oracle,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos
                }
            }
        }
        pub struct SolendReserve(Reserve);
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for SolendReserve {
            #[inline]
            fn clone(&self) -> SolendReserve {
                match *self {
                    SolendReserve(ref __self_0_0) => {
                        SolendReserve(::core::clone::Clone::clone(&(*__self_0_0)))
                    }
                }
            }
        }
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
        # [instruction (bump : u8)]
        pub struct InitializeSolend<'info> {
            # [account (mut , has_one = owner , has_one = vault_authority ,)]
            pub vault: Box<Account<'info, Vault>>,
            pub vault_authority: AccountInfo<'info>,
            /// Token account for the vault's solend lp tokens
            # [account (init , payer = payer , seeds = [vault . key () . as_ref () , solend_lp_token_mint . key () . as_ref ()] , bump = bump , token :: authority = vault_authority , token :: mint = solend_lp_token_mint ,)]
            pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,
            pub solend_lp_token_mint: AccountInfo<'info>,
            pub solend_reserve: Box<Account<'info, SolendReserve>>,
            pub owner: Signer<'info>,
            #[account(mut)]
            pub payer: Signer<'info>,
            pub token_program: Program<'info, Token>,
            pub system_program: Program<'info, System>,
            pub rent: Sysvar<'info, Rent>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for InitializeSolend<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let mut ix_data = ix_data;
                struct __Args {
                    bump: u8,
                }
                impl borsh::ser::BorshSerialize for __Args
                where
                    u8: borsh::ser::BorshSerialize,
                {
                    fn serialize<W: borsh::maybestd::io::Write>(
                        &self,
                        writer: &mut W,
                    ) -> ::core::result::Result<(), borsh::maybestd::io::Error>
                    {
                        borsh::BorshSerialize::serialize(&self.bump, writer)?;
                        Ok(())
                    }
                }
                impl borsh::de::BorshDeserialize for __Args
                where
                    u8: borsh::BorshDeserialize,
                {
                    fn deserialize(
                        buf: &mut &[u8],
                    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error>
                    {
                        Ok(Self {
                            bump: borsh::BorshDeserialize::deserialize(buf)?,
                        })
                    }
                }
                let __Args { bump } = __Args::deserialize(&mut ix_data)
                    .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_solend_lp_token = &accounts[0];
                *accounts = &accounts[1..];
                let solend_lp_token_mint: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_reserve: Box<anchor_lang::Account<SolendReserve>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let owner: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let payer: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let system_program: anchor_lang::Program<System> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let rent: Sysvar<Rent> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let __anchor_rent = Rent::get()?;
                let vault_solend_lp_token: Box<anchor_lang::Account<TokenAccount>> = {
                    if !false
                        || vault_solend_lp_token.to_account_info().owner
                            == &anchor_lang::solana_program::system_program::ID
                    {
                        let payer = payer.to_account_info();
                        let __current_lamports = vault_solend_lp_token.to_account_info().lamports();
                        if __current_lamports == 0 {
                            let lamports =
                                __anchor_rent.minimum_balance(anchor_spl::token::TokenAccount::LEN);
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::create_account(
                                    payer.to_account_info().key,
                                    vault_solend_lp_token.to_account_info().key,
                                    lamports,
                                    anchor_spl::token::TokenAccount::LEN as u64,
                                    token_program.to_account_info().key,
                                ),
                                &[
                                    payer.to_account_info(),
                                    vault_solend_lp_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    solend_lp_token_mint.key().as_ref(),
                                    &[bump][..],
                                ][..]],
                            )?;
                        } else {
                            let required_lamports = __anchor_rent
                                .minimum_balance(anchor_spl::token::TokenAccount::LEN)
                                .max(1)
                                .saturating_sub(__current_lamports);
                            if required_lamports > 0 {
                                anchor_lang::solana_program::program::invoke(
                                    &anchor_lang::solana_program::system_instruction::transfer(
                                        payer.to_account_info().key,
                                        vault_solend_lp_token.to_account_info().key,
                                        required_lamports,
                                    ),
                                    &[
                                        payer.to_account_info(),
                                        vault_solend_lp_token.to_account_info(),
                                        system_program.to_account_info(),
                                    ],
                                )?;
                            }
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::allocate(
                                    vault_solend_lp_token.to_account_info().key,
                                    anchor_spl::token::TokenAccount::LEN as u64,
                                ),
                                &[
                                    vault_solend_lp_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    solend_lp_token_mint.key().as_ref(),
                                    &[bump][..],
                                ][..]],
                            )?;
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::assign(
                                    vault_solend_lp_token.to_account_info().key,
                                    token_program.to_account_info().key,
                                ),
                                &[
                                    vault_solend_lp_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    solend_lp_token_mint.key().as_ref(),
                                    &[bump][..],
                                ][..]],
                            )?;
                        }
                        let cpi_program = token_program.to_account_info();
                        let accounts = anchor_spl::token::InitializeAccount {
                            account: vault_solend_lp_token.to_account_info(),
                            mint: solend_lp_token_mint.to_account_info(),
                            authority: vault_authority.to_account_info(),
                            rent: rent.to_account_info(),
                        };
                        let cpi_ctx = CpiContext::new(cpi_program, accounts);
                        anchor_spl::token::initialize_account(cpi_ctx)?;
                    }
                    let pa: Box<anchor_lang::Account<TokenAccount>> = Box::new(
                        anchor_lang::Account::try_from_unchecked(&vault_solend_lp_token)?,
                    );
                    pa
                };
                let (__program_signer, __bump) =
                    anchor_lang::solana_program::pubkey::Pubkey::find_program_address(
                        &[vault.key().as_ref(), solend_lp_token_mint.key().as_ref()],
                        program_id,
                    );
                if vault_solend_lp_token.to_account_info().key != &__program_signer {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if __bump != bump {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if !vault_solend_lp_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !__anchor_rent.is_exempt(
                    vault_solend_lp_token.to_account_info().lamports(),
                    vault_solend_lp_token.to_account_info().try_data_len()?,
                ) {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
                }
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.owner != owner.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_authority != vault_authority.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !payer.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(InitializeSolend {
                    vault,
                    vault_authority,
                    vault_solend_lp_token,
                    solend_lp_token_mint,
                    solend_reserve,
                    owner,
                    payer,
                    token_program,
                    system_program,
                    rent,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for InitializeSolend<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_authority.to_account_infos());
                account_infos.extend(self.vault_solend_lp_token.to_account_infos());
                account_infos.extend(self.solend_lp_token_mint.to_account_infos());
                account_infos.extend(self.solend_reserve.to_account_infos());
                account_infos.extend(self.owner.to_account_infos());
                account_infos.extend(self.payer.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos.extend(self.system_program.to_account_infos());
                account_infos.extend(self.rent.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for InitializeSolend<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_authority.to_account_metas(None));
                account_metas.extend(self.vault_solend_lp_token.to_account_metas(None));
                account_metas.extend(self.solend_lp_token_mint.to_account_metas(None));
                account_metas.extend(self.solend_reserve.to_account_metas(None));
                account_metas.extend(self.owner.to_account_metas(None));
                account_metas.extend(self.payer.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas.extend(self.system_program.to_account_metas(None));
                account_metas.extend(self.rent.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for InitializeSolend<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_solend_lp_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.payer, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_initialize_solend {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct InitializeSolend {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_solend_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_lp_token_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub owner: anchor_lang::solana_program::pubkey::Pubkey,
                pub payer: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub rent: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for InitializeSolend
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_solend_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_lp_token_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.owner, writer)?;
                    borsh::BorshSerialize::serialize(&self.payer, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.rent, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for InitializeSolend {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_authority,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_solend_lp_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.solend_lp_token_mint,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.solend_reserve,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.owner, true,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.payer, true,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.system_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.rent, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_initialize_solend {
            use super::*;
            pub struct InitializeSolend<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_solend_lp_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_lp_token_mint:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub rent: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for InitializeSolend<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_authority),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_solend_lp_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.solend_lp_token_mint),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.solend_reserve),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.owner),
                            true,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.payer),
                        true,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.system_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.rent),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for InitializeSolend<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_solend_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_lp_token_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.owner));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.payer));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.system_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.rent));
                    account_infos
                }
            }
        }
        impl<'info> YieldSourceInitializer<'info> for InitializeSolend<'info> {
            fn initialize_yield_source(&mut self) -> ProgramResult {
                self.vault.solend_reserve = self.solend_reserve.key();
                self.vault.vault_solend_lp_token = self.vault_solend_lp_token.key();
                self.vault
                    .set_yield_source_flag(YieldSourceFlags::SOLEND, true)?;
                Ok(())
            }
        }
        pub struct RefreshSolend<'info> {
            /// Vault state account
            /// Checks that the accounts passed in are correct
            # [account (mut , has_one = vault_solend_lp_token , has_one = solend_reserve ,)]
            pub vault: Box<Account<'info, Vault>>,
            /// Token account for the vault's solend lp tokens
            pub vault_solend_lp_token: Box<Account<'info, TokenAccount>>,
            # [account (executable , address = spl_token_lending :: ID ,)]
            pub solend_program: AccountInfo<'info>,
            #[account(mut)]
            pub solend_reserve: Box<Account<'info, SolendReserve>>,
            pub solend_pyth: AccountInfo<'info>,
            pub solend_switchboard: AccountInfo<'info>,
            pub clock: Sysvar<'info, Clock>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for RefreshSolend<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_solend_lp_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_program: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_reserve: Box<anchor_lang::Account<SolendReserve>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_pyth: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_switchboard: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: Sysvar<Clock> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.vault_solend_lp_token != vault_solend_lp_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.solend_reserve != solend_reserve.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !solend_program.to_account_info().executable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintExecutable.into());
                }
                if solend_program.to_account_info().key != &spl_token_lending::ID {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintAddress.into());
                }
                if !solend_reserve.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(RefreshSolend {
                    vault,
                    vault_solend_lp_token,
                    solend_program,
                    solend_reserve,
                    solend_pyth,
                    solend_switchboard,
                    clock,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for RefreshSolend<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_solend_lp_token.to_account_infos());
                account_infos.extend(self.solend_program.to_account_infos());
                account_infos.extend(self.solend_reserve.to_account_infos());
                account_infos.extend(self.solend_pyth.to_account_infos());
                account_infos.extend(self.solend_switchboard.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for RefreshSolend<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_solend_lp_token.to_account_metas(None));
                account_metas.extend(self.solend_program.to_account_metas(None));
                account_metas.extend(self.solend_reserve.to_account_metas(None));
                account_metas.extend(self.solend_pyth.to_account_metas(None));
                account_metas.extend(self.solend_switchboard.to_account_metas(None));
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for RefreshSolend<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.solend_reserve, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_refresh_solend {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct RefreshSolend {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_solend_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_pyth: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_switchboard: anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for RefreshSolend
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_solend_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_pyth, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_switchboard, writer)?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for RefreshSolend {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_solend_lp_token,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.solend_program,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.solend_reserve,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.solend_pyth,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.solend_switchboard,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_refresh_solend {
            use super::*;
            pub struct RefreshSolend<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_solend_lp_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_pyth: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_switchboard:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for RefreshSolend<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_solend_lp_token),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.solend_program),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.solend_reserve),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.solend_pyth),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.solend_switchboard),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for RefreshSolend<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_solend_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_pyth,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_switchboard,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos
                }
            }
        }
        impl<'info> RefreshSolend<'info> {
            fn solend_refresh_reserve_context(
                &self,
            ) -> CpiContext<'_, '_, '_, 'info, RefreshReserve<'info>> {
                CpiContext::new(
                    self.solend_program.clone(),
                    RefreshReserve {
                        lending_program: self.solend_program.clone(),
                        reserve: self.solend_reserve.to_account_info(),
                        pyth_reserve_liquidity_oracle: self.solend_pyth.clone(),
                        switchboard_reserve_liquidity_oracle: self.solend_switchboard.clone(),
                        clock: self.clock.to_account_info(),
                    },
                )
            }
        }
        impl<'info> Refresher<'info> for RefreshSolend<'info> {
            fn update_actual_allocation(
                &mut self,
                _remaining_accounts: &[AccountInfo<'info>],
            ) -> ProgramResult {
                ::solana_program::log::sol_log("Refreshing solend");
                refresh_reserve(self.solend_refresh_reserve_context())?;
                let solend_exchange_rate = self.solend_reserve.collateral_exchange_rate()?;
                let solend_value = solend_exchange_rate
                    .collateral_to_liquidity(self.vault_solend_lp_token.amount)?;
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Value: "],
                        &match (&solend_value,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
                self.vault.actual_allocations[Provider::Solend]
                    .update(solend_value, self.clock.slot);
                Ok(())
            }
        }
    }
    pub use self::jet::*;
    pub use port::*;
    pub use solend::*;
}
pub mod asset_container {
    mod iter {
        use core::iter::FromIterator;
        use strum::IntoEnumIterator;
        use crate::reserves::{Provider, ProviderIter};
        use super::AssetContainerGeneric;
        impl<'a, T, const N: usize> IntoIterator for &'a AssetContainerGeneric<T, N> {
            type Item = (Provider, Option<&'a T>);
            type IntoIter = AssetContainerIterator<'a, T, N>;
            fn into_iter(self) -> Self::IntoIter {
                AssetContainerIterator {
                    inner: self,
                    inner_iter: Provider::iter(),
                }
            }
        }
        impl<T, const N: usize> IntoIterator for AssetContainerGeneric<T, N> {
            type Item = (Provider, Option<T>);
            type IntoIter = OwnedAssetContainerIterator<T, N>;
            fn into_iter(self) -> Self::IntoIter {
                OwnedAssetContainerIterator {
                    inner: self,
                    inner_iter: Provider::iter(),
                }
            }
        }
        pub struct AssetContainerIterator<'inner, T, const N: usize> {
            inner: &'inner AssetContainerGeneric<T, N>,
            inner_iter: ProviderIter,
        }
        impl<'inner, T, const N: usize> Iterator for AssetContainerIterator<'inner, T, N> {
            type Item = (Provider, Option<&'inner T>);
            fn next(&mut self) -> Option<Self::Item> {
                self.inner_iter
                    .next()
                    .map(|provider| (provider, self.inner[provider].as_ref()))
            }
        }
        pub struct OwnedAssetContainerIterator<T, const N: usize> {
            inner: AssetContainerGeneric<T, N>,
            inner_iter: ProviderIter,
        }
        impl<T, const N: usize> Iterator for OwnedAssetContainerIterator<T, N> {
            type Item = (Provider, Option<T>);
            fn next(&mut self) -> Option<Self::Item> {
                self.inner_iter
                    .next()
                    .map(|provider| (provider, self.inner.inner[provider as usize].take()))
            }
        }
        impl<T: Default, const N: usize> FromIterator<(Provider, Option<T>)>
            for AssetContainerGeneric<T, N>
        {
            fn from_iter<U: IntoIterator<Item = (Provider, Option<T>)>>(iter: U) -> Self {
                iter.into_iter().fold(
                    AssetContainerGeneric::default(),
                    |mut acc, (provider, v)| {
                        acc[provider] = v;
                        acc
                    },
                )
            }
        }
    }
    mod rate {
        use anchor_lang::prelude::ProgramError;
        use boolinator::Boolinator;
        use solana_maths::{Rate, TryAdd};
        use crate::errors::ErrorCode;
        use super::AssetContainerGeneric;
        impl<const N: usize> AssetContainerGeneric<Rate, N> {
            /// Return error if weights do not add up to 100%
            /// OR if any are greater than the allocation cap
            pub fn verify_weights(&self, allocation_cap_pct: u8) -> Result<(), ProgramError> {
                let cap = &Rate::from_percent(allocation_cap_pct);
                let max = self
                    .into_iter()
                    .flat_map(|(_, r)| r)
                    .max()
                    .ok_or(ErrorCode::InvalidProposedWeights)?;
                let sum = self
                    .into_iter()
                    .flat_map(|(_, r)| r)
                    .try_fold(Rate::zero(), |acc, x| acc.try_add(*x))?;
                (sum == Rate::one() && max <= cap)
                    .ok_or_else(|| ErrorCode::InvalidProposedWeights.into())
            }
        }
        impl<const N: usize> From<AssetContainerGeneric<u16, N>> for AssetContainerGeneric<Rate, N> {
            fn from(c: AssetContainerGeneric<u16, N>) -> Self {
                c.apply(|_, v| v.map(|r| Rate::from_bips(u64::from(*r))))
            }
        }
    }
    mod reserves {
        use core::{convert::TryFrom, ops::Index};
        use std::cmp::Ordering;
        use itertools::Itertools;
        use solana_maths::{Rate, TryAdd, TryDiv, TryMul, TrySub};
        use anchor_lang::prelude::ProgramError;
        use crate::{
            errors::ErrorCode,
            reserves::{Provider, Reserves, ReturnCalculator},
            state::StrategyType,
        };
        use super::AssetContainer;
        pub fn compare(
            lhs: &impl ReturnCalculator,
            rhs: &impl ReturnCalculator,
        ) -> Result<Ordering, ProgramError> {
            Ok(lhs.calculate_return(0)?.cmp(&rhs.calculate_return(0)?))
        }
        impl AssetContainer<Reserves> {
            fn calculate_weights_max_yield(
                &self,
                allocation_cap_pct: u8,
            ) -> Result<AssetContainer<Rate>, ProgramError> {
                self.into_iter()
                    .flat_map(|(p, r)| r.map(|v| (p, v)))
                    .sorted_unstable_by(|(_, alloc_y), (_, alloc_x)| {
                        compare(*alloc_x, *alloc_y)
                            .expect("Could not successfully compare allocations")
                    })
                    .try_fold(
                        (AssetContainer::<Rate>::default(), Rate::one()),
                        |(mut strategy_weights, remaining_weight), (provider, _)| {
                            let target_weight =
                                remaining_weight.min(Rate::from_percent(allocation_cap_pct));
                            strategy_weights[provider] = Some(target_weight);
                            match remaining_weight.try_sub(target_weight) {
                                Ok(r) => Ok((strategy_weights, r)),
                                Err(e) => Err(e),
                            }
                        },
                    )
                    .map(|(r, _)| r)
            }
            fn calculate_weights_equal(&self) -> Result<AssetContainer<Rate>, ProgramError> {
                u8::try_from(self.len())
                    .map_err(|_| ErrorCode::StrategyError.into())
                    .and_then(|num_assets| Rate::from_percent(num_assets).try_mul(100))
                    .and_then(|r| Rate::one().try_div(r))
                    .map(|equal_allocation| self.apply(|_, v| v.map(|_| equal_allocation)))
            }
            pub fn calculate_weights(
                &self,
                strategy_type: StrategyType,
                allocation_cap_pct: u8,
            ) -> Result<AssetContainer<Rate>, ProgramError> {
                match strategy_type {
                    StrategyType::MaxYield => self.calculate_weights_max_yield(allocation_cap_pct),
                    StrategyType::EqualAllocation => self.calculate_weights_equal(),
                }
            }
            pub fn get_apr(
                &self,
                weights: &dyn Index<Provider, Output = Option<Rate>>,
                allocations: &dyn Index<Provider, Output = Option<u64>>,
            ) -> Result<Rate, ProgramError> {
                self.into_iter()
                    .map(|(p, r)| (r, allocations[p], weights[p]))
                    .flat_map(|v| match v {
                        (Some(r), Some(a), Some(w)) => Some((r, a, w)),
                        _ => None,
                    })
                    .map(|(r, a, w)| r.calculate_return(a).and_then(|ret| w.try_mul(ret)))
                    .try_fold(Rate::zero(), |acc, r| acc.try_add(r?))
            }
        }
    }
    mod u64 {
        use anchor_lang::prelude::ProgramError;
        use solana_maths::{Decimal, Rate, TryMul};
        use super::AssetContainerGeneric;
        impl<const N: usize> AssetContainerGeneric<u64, N> {
            /// Calculates $ allocations for a corresponding set of % allocations
            /// and a given total amount
            pub fn try_from_weights(
                rates: &AssetContainerGeneric<Rate, N>,
                total_amount: u64,
            ) -> Result<Self, ProgramError> {
                rates.try_apply(|_, rate| match rate {
                    Some(r) => {
                        Ok(Some(r.try_mul(total_amount).and_then(|product| {
                            Decimal::from(product).try_floor_u64()
                        })?))
                    }
                    None => Ok(None),
                })
            }
        }
    }
    pub use self::u64::*;
    pub use iter::*;
    pub use rate::*;
    pub use reserves::*;
    use core::ops::{Index, IndexMut};
    use strum::{EnumCount, IntoEnumIterator};
    use crate::reserves::Provider;
    pub type AssetContainer<T> = AssetContainerGeneric<T, { Provider::COUNT }>;
    /// Provides an abstraction over supported assets
    pub struct AssetContainerGeneric<T, const N: usize> {
        pub(crate) inner: [Option<T>; N],
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<T: ::core::fmt::Debug, const N: usize> ::core::fmt::Debug for AssetContainerGeneric<T, N> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                AssetContainerGeneric {
                    inner: ref __self_0_0,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "AssetContainerGeneric");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "inner",
                        &&(*__self_0_0),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<T: ::core::clone::Clone, const N: usize> ::core::clone::Clone for AssetContainerGeneric<T, N> {
        #[inline]
        fn clone(&self) -> AssetContainerGeneric<T, N> {
            match *self {
                AssetContainerGeneric {
                    inner: ref __self_0_0,
                } => AssetContainerGeneric {
                    inner: ::core::clone::Clone::clone(&(*__self_0_0)),
                },
            }
        }
    }
    impl<T, const N: usize> AssetContainerGeneric<T, N> {
        pub fn len(&self) -> usize {
            self.into_iter().filter(|(_, o)| o.is_some()).count()
        }
        /// Returns if the container is uninitialized
        pub fn is_empty(&self) -> bool {
            self.inner.iter().all(Option::is_none)
        }
    }
    impl<T, const N: usize> Index<Provider> for AssetContainerGeneric<T, N> {
        type Output = Option<T>;
        fn index(&self, index: Provider) -> &Self::Output {
            &self.inner[index as usize]
        }
    }
    impl<T, const N: usize> IndexMut<Provider> for AssetContainerGeneric<T, N> {
        fn index_mut(&mut self, index: Provider) -> &mut Self::Output {
            &mut self.inner[index as usize]
        }
    }
    impl<T: Default, const N: usize> Default for AssetContainerGeneric<T, N> {
        fn default() -> Self {
            Self {
                inner: [(); N].map(|_| None),
            }
        }
    }
    impl<'a, T, const N: usize> From<&'a dyn Index<Provider, Output = &'a T>>
        for AssetContainerGeneric<&'a T, N>
    where
        &'a T: Default,
    {
        fn from(p: &'a dyn Index<Provider, Output = &'a T>) -> Self {
            Provider::iter().fold(AssetContainerGeneric::default(), |mut acc, provider| {
                acc[provider] = Some(p[provider]);
                acc
            })
        }
    }
    impl<T, const N: usize> AssetContainerGeneric<T, N> {
        pub fn apply_owned<U: Clone + Default, F: Fn(Provider, Option<&T>) -> Option<U>>(
            mut self,
            f: F,
        ) -> AssetContainerGeneric<U, N> {
            Provider::iter()
                .map(|provider| {
                    (
                        provider,
                        f(provider, self.inner[provider as usize].take().as_ref()),
                    )
                })
                .collect()
        }
        /// Applies `f` to each element of the container individually, yielding a new container
        pub fn apply<U: Default, F: Fn(Provider, Option<&T>) -> Option<U>>(
            &self,
            f: F,
        ) -> AssetContainerGeneric<U, N> {
            Provider::iter()
                .map(|provider| (provider, f(provider, self[provider].as_ref())))
                .collect()
        }
        /// Identical to `apply` but returns a `Result<AssetContainerGeneric<..>>`
        pub fn try_apply<U: Default, E, F: Fn(Provider, Option<&T>) -> Result<Option<U>, E>>(
            &self,
            f: F,
        ) -> Result<AssetContainerGeneric<U, N>, E> {
            Provider::iter()
                .map(|provider| f(provider, self[provider].as_ref()).map(|res| (provider, res)))
                .collect()
        }
    }
}
pub mod errors {
    use anchor_lang::prelude::*;
    /// Anchor generated Result to be used as the return type for the
    /// program.
    pub type Result<T> = std::result::Result<T, Error>;
    /// Anchor generated error allowing one to easily return a
    /// `ProgramError` or a custom, user defined error code by utilizing
    /// its `From` implementation.
    #[doc(hidden)]
    pub enum Error {
        #[error(transparent)]
        ProgramError(#[from] anchor_lang::solana_program::program_error::ProgramError),
        #[error(transparent)]
        ErrorCode(#[from] ErrorCode),
    }
    #[allow(unused_qualifications)]
    impl std::error::Error for Error {
        fn source(&self) -> std::option::Option<&(dyn std::error::Error + 'static)> {
            use thiserror::private::AsDynError;
            #[allow(deprecated)]
            match self {
                Error::ProgramError { 0: transparent } => {
                    std::error::Error::source(transparent.as_dyn_error())
                }
                Error::ErrorCode { 0: transparent } => {
                    std::error::Error::source(transparent.as_dyn_error())
                }
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::fmt::Display for Error {
        fn fmt(&self, __formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            #[allow(unused_variables, deprecated, clippy::used_underscore_binding)]
            match self {
                Error::ProgramError(_0) => std::fmt::Display::fmt(_0, __formatter),
                Error::ErrorCode(_0) => std::fmt::Display::fmt(_0, __formatter),
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::convert::From<anchor_lang::solana_program::program_error::ProgramError> for Error {
        #[allow(deprecated)]
        fn from(source: anchor_lang::solana_program::program_error::ProgramError) -> Self {
            Error::ProgramError { 0: source }
        }
    }
    #[allow(unused_qualifications)]
    impl std::convert::From<ErrorCode> for Error {
        #[allow(deprecated)]
        fn from(source: ErrorCode) -> Self {
            Error::ErrorCode { 0: source }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Error {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&Error::ProgramError(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "ProgramError");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&Error::ErrorCode(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "ErrorCode");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    #[repr(u32)]
    pub enum ErrorCode {
        MathError,
        StrategyError,
        VaultIsNotRefreshed,
        AllocationIsNotUpdated,
        TryFromReserveError,
        OverflowError,
        InvalidReferralFeeConfig,
        InvalidFeeConfig,
        InvalidProposedWeights,
        RebalanceProofCheckFailed,
        DepositCapError,
        InvalidAccount,
        InsufficientAccounts,
        InvalidAllocationCap,
        InvalidVaultFlags,
        HaltedVault,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for ErrorCode {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&ErrorCode::MathError,) => ::core::fmt::Formatter::write_str(f, "MathError"),
                (&ErrorCode::StrategyError,) => {
                    ::core::fmt::Formatter::write_str(f, "StrategyError")
                }
                (&ErrorCode::VaultIsNotRefreshed,) => {
                    ::core::fmt::Formatter::write_str(f, "VaultIsNotRefreshed")
                }
                (&ErrorCode::AllocationIsNotUpdated,) => {
                    ::core::fmt::Formatter::write_str(f, "AllocationIsNotUpdated")
                }
                (&ErrorCode::TryFromReserveError,) => {
                    ::core::fmt::Formatter::write_str(f, "TryFromReserveError")
                }
                (&ErrorCode::OverflowError,) => {
                    ::core::fmt::Formatter::write_str(f, "OverflowError")
                }
                (&ErrorCode::InvalidReferralFeeConfig,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidReferralFeeConfig")
                }
                (&ErrorCode::InvalidFeeConfig,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidFeeConfig")
                }
                (&ErrorCode::InvalidProposedWeights,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidProposedWeights")
                }
                (&ErrorCode::RebalanceProofCheckFailed,) => {
                    ::core::fmt::Formatter::write_str(f, "RebalanceProofCheckFailed")
                }
                (&ErrorCode::DepositCapError,) => {
                    ::core::fmt::Formatter::write_str(f, "DepositCapError")
                }
                (&ErrorCode::InvalidAccount,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidAccount")
                }
                (&ErrorCode::InsufficientAccounts,) => {
                    ::core::fmt::Formatter::write_str(f, "InsufficientAccounts")
                }
                (&ErrorCode::InvalidAllocationCap,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidAllocationCap")
                }
                (&ErrorCode::InvalidVaultFlags,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidVaultFlags")
                }
                (&ErrorCode::HaltedVault,) => ::core::fmt::Formatter::write_str(f, "HaltedVault"),
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for ErrorCode {
        #[inline]
        fn clone(&self) -> ErrorCode {
            {
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for ErrorCode {}
    impl std::fmt::Display for ErrorCode {
        fn fmt(
            &self,
            fmt: &mut std::fmt::Formatter<'_>,
        ) -> std::result::Result<(), std::fmt::Error> {
            match self {
                ErrorCode::MathError => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["failed to perform some math operation safely"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::StrategyError => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Failed to run the strategy"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::VaultIsNotRefreshed => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Vault is not refreshed"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::AllocationIsNotUpdated => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Allocation is not updated"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::TryFromReserveError => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Failed to convert from Reserve"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::OverflowError => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Failed to perform a math operation without an overflow"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::InvalidReferralFeeConfig => {
                    fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Referral fee split cannot set to be over 50%"],
                        &match () {
                            _args => [],
                        },
                    ))
                }
                ErrorCode::InvalidFeeConfig => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Fees cannot be set to over 100%"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::InvalidProposedWeights => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Proposed weights do not meet the required constraints"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::RebalanceProofCheckFailed => {
                    fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Proposed weights failed proof check"],
                        &match () {
                            _args => [],
                        },
                    ))
                }
                ErrorCode::DepositCapError => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Vault size limit is reached"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::InvalidAccount => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Account passed in is not valid"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::InsufficientAccounts => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Insufficient number of accounts for a given operation"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::InvalidAllocationCap => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Allocation cap cannot set to under 1/(number of assets) or over 100%"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::InvalidVaultFlags => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Bits passed in do not result in valid vault flags"],
                    &match () {
                        _args => [],
                    },
                )),
                ErrorCode::HaltedVault => fmt.write_fmt(::core::fmt::Arguments::new_v1(
                    &["Vault is halted"],
                    &match () {
                        _args => [],
                    },
                )),
            }
        }
    }
    impl std::error::Error for ErrorCode {}
    impl std::convert::From<Error> for anchor_lang::solana_program::program_error::ProgramError {
        fn from(e: Error) -> anchor_lang::solana_program::program_error::ProgramError {
            match e {
                Error::ProgramError(e) => e,
                Error::ErrorCode(c) => {
                    anchor_lang::solana_program::program_error::ProgramError::Custom(
                        c as u32 + anchor_lang::__private::ERROR_CODE_OFFSET,
                    )
                }
            }
        }
    }
    impl std::convert::From<ErrorCode> for anchor_lang::solana_program::program_error::ProgramError {
        fn from(e: ErrorCode) -> anchor_lang::solana_program::program_error::ProgramError {
            let err: Error = e.into();
            err.into()
        }
    }
}
pub mod instructions {
    pub mod consolidate_refresh {
        #![allow(dead_code)]
        #![allow(unused_imports)]
        use boolinator::Boolinator;
        use anchor_lang::prelude::*;
        use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
        use port_anchor_adaptor::{port_lending_id, PortReserve};
        use crate::adapters::{solend, SolendReserve};
        use crate::errors::ErrorCode;
        use crate::reserves::Provider;
        use crate::state::{Vault, VaultFlags};
        use strum::IntoEnumIterator;
        pub struct ConsolidateRefresh<'info> {
            /// Vault state account
            /// Checks that the accounts passed in are correct
            # [account (mut , has_one = vault_authority , has_one = vault_reserve_token , has_one = lp_token_mint ,)]
            pub vault: Box<Account<'info, Vault>>,
            /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
            pub vault_authority: AccountInfo<'info>,
            /// Token account for the vault's reserve tokens
            pub vault_reserve_token: Box<Account<'info, TokenAccount>>,
            /// Mint for the vault lp token
            #[account(mut)]
            pub lp_token_mint: Box<Account<'info, Mint>>,
            pub token_program: Program<'info, Token>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for ConsolidateRefresh<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_reserve_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let lp_token_mint: Box<anchor_lang::Account<Mint>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.vault_authority != vault_authority.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_reserve_token != vault_reserve_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.lp_token_mint != lp_token_mint.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !lp_token_mint.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(ConsolidateRefresh {
                    vault,
                    vault_authority,
                    vault_reserve_token,
                    lp_token_mint,
                    token_program,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for ConsolidateRefresh<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_authority.to_account_infos());
                account_infos.extend(self.vault_reserve_token.to_account_infos());
                account_infos.extend(self.lp_token_mint.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for ConsolidateRefresh<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_authority.to_account_metas(None));
                account_metas.extend(self.vault_reserve_token.to_account_metas(None));
                account_metas.extend(self.lp_token_mint.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for ConsolidateRefresh<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.lp_token_mint, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_consolidate_refresh {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct ConsolidateRefresh {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub lp_token_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for ConsolidateRefresh
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.lp_token_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for ConsolidateRefresh {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_authority,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_reserve_token,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.lp_token_mint,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_consolidate_refresh {
            use super::*;
            pub struct ConsolidateRefresh<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub lp_token_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for ConsolidateRefresh<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_authority),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_reserve_token),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.lp_token_mint),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for ConsolidateRefresh<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lp_token_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos
                }
            }
        }
        impl<'info> ConsolidateRefresh<'info> {}
        /// updates the vault total value, and collects fees
        pub fn handler<'info>(
            ctx: Context<'_, '_, '_, 'info, ConsolidateRefresh<'info>>,
        ) -> ProgramResult {
            ::solana_program::log::sol_log("Consolidate vault refreshing");
            (!ctx
                .accounts
                .vault
                .get_halt_flags()
                .contains(VaultFlags::HALT_REFRESHES))
            .ok_or::<ProgramError>(ErrorCode::HaltedVault.into())?;
            let clock_slot = Clock::get()?.slot;
            let vault_reserve_token_amount = ctx.accounts.vault_reserve_token.amount;
            let vault_value =
                Provider::iter().try_fold(ctx.accounts.vault_reserve_token.amount, |acc, p| {
                    let allocation = ctx.accounts.vault.actual_allocations[p];
                    if ctx.accounts.vault.get_yield_source_availability(p) {
                        (allocation.last_update.slots_elapsed(clock_slot)? == 0)
                            .as_result::<u64, ProgramError>(
                                acc.checked_add(allocation.value)
                                    .ok_or(ErrorCode::OverflowError)?,
                                ErrorCode::AllocationIsNotUpdated.into(),
                            )
                    } else {
                        Ok(acc)
                    }
                })?;
            #[cfg(feature = "debug")]
            {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Tokens value: "],
                        &match (&vault_reserve_token_amount,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Vault value: "],
                        &match (&vault_value,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
            }
            #[cfg(not(feature = "fees"))]
            if ctx.accounts.vault.config.fee_carry_bps > 0
                || ctx.accounts.vault.config.fee_mgmt_bps > 0
            {
                ::solana_program::log::sol_log(
                    "WARNING: Fees are non-zero but the fee feature is deactivated",
                );
            }
            ctx.accounts.vault.value.update(vault_value, clock_slot);
            Ok(())
        }
    }
    pub mod deposit {
        use std::convert::Into;
        use boolinator::Boolinator;
        use anchor_lang::prelude::*;
        use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};
        use crate::{
            errors::ErrorCode,
            state::{Vault, VaultFlags},
        };
        pub struct DepositEvent {
            vault: Pubkey,
            user: Pubkey,
            amount: u64,
        }
        impl borsh::ser::BorshSerialize for DepositEvent
        where
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            u64: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.vault, writer)?;
                borsh::BorshSerialize::serialize(&self.user, writer)?;
                borsh::BorshSerialize::serialize(&self.amount, writer)?;
                Ok(())
            }
        }
        impl borsh::de::BorshDeserialize for DepositEvent
        where
            Pubkey: borsh::BorshDeserialize,
            Pubkey: borsh::BorshDeserialize,
            u64: borsh::BorshDeserialize,
        {
            fn deserialize(
                buf: &mut &[u8],
            ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
                Ok(Self {
                    vault: borsh::BorshDeserialize::deserialize(buf)?,
                    user: borsh::BorshDeserialize::deserialize(buf)?,
                    amount: borsh::BorshDeserialize::deserialize(buf)?,
                })
            }
        }
        impl anchor_lang::Event for DepositEvent {
            fn data(&self) -> Vec<u8> {
                let mut d = [120, 248, 61, 83, 31, 142, 107, 144].to_vec();
                d.append(&mut self.try_to_vec().unwrap());
                d
            }
        }
        impl anchor_lang::Discriminator for DepositEvent {
            fn discriminator() -> [u8; 8] {
                [120, 248, 61, 83, 31, 142, 107, 144]
            }
        }
        pub struct Deposit<'info> {
            /// Vault state account
            /// Checks that the refresh has been called in the same slot
            /// Checks that the accounts passed in are correct
            # [account (mut , constraint =! vault . value . last_update . is_stale (clock . slot) ? @ ErrorCode :: VaultIsNotRefreshed , has_one = lp_token_mint , has_one = vault_authority , has_one = vault_reserve_token ,)]
            pub vault: Box<Account<'info, Vault>>,
            /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
            pub vault_authority: AccountInfo<'info>,
            /// Token account for the vault's reserve tokens
            #[account(mut)]
            pub vault_reserve_token: Box<Account<'info, TokenAccount>>,
            /// Mint for the vault's lp token
            #[account(mut)]
            pub lp_token_mint: Box<Account<'info, Mint>>,
            /// Token account from which reserve tokens are transferred
            #[account(mut)]
            pub user_reserve_token: Box<Account<'info, TokenAccount>>,
            /// Account where vault LP tokens are minted to
            #[account(mut)]
            pub user_lp_token: Box<Account<'info, TokenAccount>>,
            /// Authority of the user_reserve_token account
            /// Must be a signer
            pub user_authority: Signer<'info>,
            pub token_program: Program<'info, Token>,
            pub clock: Sysvar<'info, Clock>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for Deposit<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_reserve_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let lp_token_mint: Box<anchor_lang::Account<Mint>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let user_reserve_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let user_lp_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let user_authority: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: Sysvar<Clock> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.lp_token_mint != lp_token_mint.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_authority != vault_authority.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_reserve_token != vault_reserve_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !(!vault.value.last_update.is_stale(clock.slot)?) {
                    return Err(ErrorCode::VaultIsNotRefreshed.into());
                }
                if !vault_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !lp_token_mint.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !user_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !user_lp_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(Deposit {
                    vault,
                    vault_authority,
                    vault_reserve_token,
                    lp_token_mint,
                    user_reserve_token,
                    user_lp_token,
                    user_authority,
                    token_program,
                    clock,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Deposit<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_authority.to_account_infos());
                account_infos.extend(self.vault_reserve_token.to_account_infos());
                account_infos.extend(self.lp_token_mint.to_account_infos());
                account_infos.extend(self.user_reserve_token.to_account_infos());
                account_infos.extend(self.user_lp_token.to_account_infos());
                account_infos.extend(self.user_authority.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Deposit<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_authority.to_account_metas(None));
                account_metas.extend(self.vault_reserve_token.to_account_metas(None));
                account_metas.extend(self.lp_token_mint.to_account_metas(None));
                account_metas.extend(self.user_reserve_token.to_account_metas(None));
                account_metas.extend(self.user_lp_token.to_account_metas(None));
                account_metas.extend(self.user_authority.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for Deposit<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_reserve_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.lp_token_mint, program_id)?;
                anchor_lang::AccountsExit::exit(&self.user_reserve_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.user_lp_token, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_deposit {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct Deposit {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub lp_token_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub user_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub user_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub user_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for Deposit
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.lp_token_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.user_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.user_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.user_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for Deposit {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_authority,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_reserve_token,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.lp_token_mint,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.user_reserve_token,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.user_lp_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.user_authority,
                            true,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_deposit {
            use super::*;
            pub struct Deposit<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub lp_token_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub user_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub user_lp_token: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub user_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for Deposit<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_authority),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_reserve_token),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.lp_token_mint),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.user_reserve_token),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.user_lp_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.user_authority),
                            true,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for Deposit<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lp_token_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.user_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.user_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.user_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos
                }
            }
        }
        impl<'info> Deposit<'info> {
            /// CpiContext for minting vault Lp tokens to user account
            fn mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
                CpiContext::new(
                    self.token_program.to_account_info(),
                    MintTo {
                        mint: self.lp_token_mint.to_account_info(),
                        to: self.user_lp_token.to_account_info(),
                        authority: self.vault_authority.clone(),
                    },
                )
            }
            /// CpiContext for transferring reserve tokens from user to vault
            fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.user_reserve_token.to_account_info(),
                        to: self.vault_reserve_token.to_account_info(),
                        authority: self.user_authority.to_account_info(),
                    },
                )
            }
        }
        /// Deposit to the vault
        ///
        /// Transfers reserve tokens from user to vault and mints their share of lp tokens
        pub fn handler(ctx: Context<Deposit>, reserve_token_amount: u64) -> ProgramResult {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Depositing ", " reserve tokens"],
                    &match (&reserve_token_amount,) {
                        _args => [::core::fmt::ArgumentV1::new(
                            _args.0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            });
            (!ctx
                .accounts
                .vault
                .get_halt_flags()
                .contains(VaultFlags::HALT_DEPOSITS_WITHDRAWS))
            .ok_or::<ProgramError>(ErrorCode::HaltedVault.into())?;
            let vault = &ctx.accounts.vault;
            let lp_tokens_to_mint = crate::math::calc_reserve_to_lp(
                reserve_token_amount,
                ctx.accounts.lp_token_mint.supply,
                vault.value.value,
            )
            .ok_or(ErrorCode::MathError)?;
            let total_value = ctx
                .accounts
                .vault
                .value
                .value
                .checked_add(reserve_token_amount)
                .ok_or(ErrorCode::OverflowError)?;
            if total_value > ctx.accounts.vault.config.deposit_cap {
                ::solana_program::log::sol_log("Deposit cap reached");
                return Err(ErrorCode::DepositCapError.into());
            }
            token::transfer(ctx.accounts.transfer_context(), reserve_token_amount)?;
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Minting ", " LP tokens"],
                    &match (&lp_tokens_to_mint,) {
                        _args => [::core::fmt::ArgumentV1::new(
                            _args.0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            });
            token::mint_to(
                ctx.accounts
                    .mint_to_context()
                    .with_signer(&[&vault.authority_seeds()]),
                lp_tokens_to_mint,
            )?;
            ctx.accounts.vault.value.value = ctx
                .accounts
                .vault
                .value
                .value
                .checked_add(reserve_token_amount)
                .ok_or(ErrorCode::MathError)?;
            {
                let data = anchor_lang::Event::data(&DepositEvent {
                    vault: ctx.accounts.vault.key(),
                    user: ctx.accounts.user_authority.key(),
                    amount: reserve_token_amount,
                });
                let msg_str = &anchor_lang::__private::base64::encode(data);
                ::solana_program::log::sol_log(msg_str);
            };
            Ok(())
        }
    }
    pub mod init_vault {
        use anchor_lang::prelude::*;
        use anchor_spl::{
            associated_token::{self, AssociatedToken, Create},
            token::{Mint, Token, TokenAccount},
        };
        use std::convert::Into;
        use crate::state::*;
        pub struct InitBumpSeeds {
            authority: u8,
            reserve: u8,
            lp_mint: u8,
        }
        impl borsh::de::BorshDeserialize for InitBumpSeeds
        where
            u8: borsh::BorshDeserialize,
            u8: borsh::BorshDeserialize,
            u8: borsh::BorshDeserialize,
        {
            fn deserialize(
                buf: &mut &[u8],
            ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
                Ok(Self {
                    authority: borsh::BorshDeserialize::deserialize(buf)?,
                    reserve: borsh::BorshDeserialize::deserialize(buf)?,
                    lp_mint: borsh::BorshDeserialize::deserialize(buf)?,
                })
            }
        }
        impl borsh::ser::BorshSerialize for InitBumpSeeds
        where
            u8: borsh::ser::BorshSerialize,
            u8: borsh::ser::BorshSerialize,
            u8: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.authority, writer)?;
                borsh::BorshSerialize::serialize(&self.reserve, writer)?;
                borsh::BorshSerialize::serialize(&self.lp_mint, writer)?;
                Ok(())
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for InitBumpSeeds {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match *self {
                    InitBumpSeeds {
                        authority: ref __self_0_0,
                        reserve: ref __self_0_1,
                        lp_mint: ref __self_0_2,
                    } => {
                        let debug_trait_builder =
                            &mut ::core::fmt::Formatter::debug_struct(f, "InitBumpSeeds");
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "authority",
                            &&(*__self_0_0),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "reserve",
                            &&(*__self_0_1),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "lp_mint",
                            &&(*__self_0_2),
                        );
                        ::core::fmt::DebugStruct::finish(debug_trait_builder)
                    }
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for InitBumpSeeds {
            #[inline]
            fn clone(&self) -> InitBumpSeeds {
                match *self {
                    InitBumpSeeds {
                        authority: ref __self_0_0,
                        reserve: ref __self_0_1,
                        lp_mint: ref __self_0_2,
                    } => InitBumpSeeds {
                        authority: ::core::clone::Clone::clone(&(*__self_0_0)),
                        reserve: ::core::clone::Clone::clone(&(*__self_0_1)),
                        lp_mint: ::core::clone::Clone::clone(&(*__self_0_2)),
                    },
                }
            }
        }
        pub struct VaultConfigArg {
            pub deposit_cap: u64,
            pub fee_carry_bps: u32,
            pub fee_mgmt_bps: u32,
            pub referral_fee_pct: u8,
            pub allocation_cap_pct: u8,
            pub rebalance_mode: RebalanceMode,
            pub strategy_type: StrategyType,
        }
        impl borsh::de::BorshDeserialize for VaultConfigArg
        where
            u64: borsh::BorshDeserialize,
            u32: borsh::BorshDeserialize,
            u32: borsh::BorshDeserialize,
            u8: borsh::BorshDeserialize,
            u8: borsh::BorshDeserialize,
            RebalanceMode: borsh::BorshDeserialize,
            StrategyType: borsh::BorshDeserialize,
        {
            fn deserialize(
                buf: &mut &[u8],
            ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
                Ok(Self {
                    deposit_cap: borsh::BorshDeserialize::deserialize(buf)?,
                    fee_carry_bps: borsh::BorshDeserialize::deserialize(buf)?,
                    fee_mgmt_bps: borsh::BorshDeserialize::deserialize(buf)?,
                    referral_fee_pct: borsh::BorshDeserialize::deserialize(buf)?,
                    allocation_cap_pct: borsh::BorshDeserialize::deserialize(buf)?,
                    rebalance_mode: borsh::BorshDeserialize::deserialize(buf)?,
                    strategy_type: borsh::BorshDeserialize::deserialize(buf)?,
                })
            }
        }
        impl borsh::ser::BorshSerialize for VaultConfigArg
        where
            u64: borsh::ser::BorshSerialize,
            u32: borsh::ser::BorshSerialize,
            u32: borsh::ser::BorshSerialize,
            u8: borsh::ser::BorshSerialize,
            u8: borsh::ser::BorshSerialize,
            RebalanceMode: borsh::ser::BorshSerialize,
            StrategyType: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.deposit_cap, writer)?;
                borsh::BorshSerialize::serialize(&self.fee_carry_bps, writer)?;
                borsh::BorshSerialize::serialize(&self.fee_mgmt_bps, writer)?;
                borsh::BorshSerialize::serialize(&self.referral_fee_pct, writer)?;
                borsh::BorshSerialize::serialize(&self.allocation_cap_pct, writer)?;
                borsh::BorshSerialize::serialize(&self.rebalance_mode, writer)?;
                borsh::BorshSerialize::serialize(&self.strategy_type, writer)?;
                Ok(())
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for VaultConfigArg {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match *self {
                    VaultConfigArg {
                        deposit_cap: ref __self_0_0,
                        fee_carry_bps: ref __self_0_1,
                        fee_mgmt_bps: ref __self_0_2,
                        referral_fee_pct: ref __self_0_3,
                        allocation_cap_pct: ref __self_0_4,
                        rebalance_mode: ref __self_0_5,
                        strategy_type: ref __self_0_6,
                    } => {
                        let debug_trait_builder =
                            &mut ::core::fmt::Formatter::debug_struct(f, "VaultConfigArg");
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "deposit_cap",
                            &&(*__self_0_0),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "fee_carry_bps",
                            &&(*__self_0_1),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "fee_mgmt_bps",
                            &&(*__self_0_2),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "referral_fee_pct",
                            &&(*__self_0_3),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "allocation_cap_pct",
                            &&(*__self_0_4),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "rebalance_mode",
                            &&(*__self_0_5),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "strategy_type",
                            &&(*__self_0_6),
                        );
                        ::core::fmt::DebugStruct::finish(debug_trait_builder)
                    }
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for VaultConfigArg {
            #[inline]
            fn clone(&self) -> VaultConfigArg {
                match *self {
                    VaultConfigArg {
                        deposit_cap: ref __self_0_0,
                        fee_carry_bps: ref __self_0_1,
                        fee_mgmt_bps: ref __self_0_2,
                        referral_fee_pct: ref __self_0_3,
                        allocation_cap_pct: ref __self_0_4,
                        rebalance_mode: ref __self_0_5,
                        strategy_type: ref __self_0_6,
                    } => VaultConfigArg {
                        deposit_cap: ::core::clone::Clone::clone(&(*__self_0_0)),
                        fee_carry_bps: ::core::clone::Clone::clone(&(*__self_0_1)),
                        fee_mgmt_bps: ::core::clone::Clone::clone(&(*__self_0_2)),
                        referral_fee_pct: ::core::clone::Clone::clone(&(*__self_0_3)),
                        allocation_cap_pct: ::core::clone::Clone::clone(&(*__self_0_4)),
                        rebalance_mode: ::core::clone::Clone::clone(&(*__self_0_5)),
                        strategy_type: ::core::clone::Clone::clone(&(*__self_0_6)),
                    },
                }
            }
        }
        # [instruction (bumps : InitBumpSeeds)]
        pub struct Initialize<'info> {
            /// Vault state account
            #[account(zero)]
            pub vault: Box<Account<'info, Vault>>,
            /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
            # [account (mut , seeds = [vault . key () . as_ref () , b"authority" . as_ref ()] , bump = bumps . authority ,)]
            pub vault_authority: AccountInfo<'info>,
            /// Mint for vault lp token
            # [account (init , payer = payer , seeds = [vault . key () . as_ref () , b"lp_mint" . as_ref ()] , bump = bumps . lp_mint , mint :: authority = vault_authority , mint :: decimals = reserve_token_mint . decimals ,)]
            pub lp_token_mint: Box<Account<'info, Mint>>,
            /// Token account for vault reserve tokens
            # [account (init , payer = payer , seeds = [vault . key () . as_ref () , reserve_token_mint . key () . as_ref ()] , bump = bumps . reserve , token :: authority = vault_authority , token :: mint = reserve_token_mint ,)]
            pub vault_reserve_token: Box<Account<'info, TokenAccount>>,
            /// Mint of the token that the vault accepts and stores
            pub reserve_token_mint: Box<Account<'info, Mint>>,
            /// Token account that receives the primary ratio of fees from the vault
            /// denominated in vault lp tokens
            #[account(mut)]
            pub fee_receiver: AccountInfo<'info>,
            /// Token account that receives the secondary ratio of fees from the vault
            /// denominated in vault lp tokens
            #[account(mut)]
            pub referral_fee_receiver: AccountInfo<'info>,
            /// Owner of the referral fee reciever token account
            pub referral_fee_owner: AccountInfo<'info>,
            /// Account that pays for above account inits
            #[account(mut)]
            pub payer: Signer<'info>,
            /// Owner of the vault
            /// Only this account can call restricted instructions
            /// Acts as authority of the fee receiver account
            pub owner: AccountInfo<'info>,
            pub system_program: Program<'info, System>,
            pub token_program: Program<'info, Token>,
            pub associated_token_program: Program<'info, AssociatedToken>,
            pub rent: Sysvar<'info, Rent>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for Initialize<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let mut ix_data = ix_data;
                struct __Args {
                    bumps: InitBumpSeeds,
                }
                impl borsh::ser::BorshSerialize for __Args
                where
                    InitBumpSeeds: borsh::ser::BorshSerialize,
                {
                    fn serialize<W: borsh::maybestd::io::Write>(
                        &self,
                        writer: &mut W,
                    ) -> ::core::result::Result<(), borsh::maybestd::io::Error>
                    {
                        borsh::BorshSerialize::serialize(&self.bumps, writer)?;
                        Ok(())
                    }
                }
                impl borsh::de::BorshDeserialize for __Args
                where
                    InitBumpSeeds: borsh::BorshDeserialize,
                {
                    fn deserialize(
                        buf: &mut &[u8],
                    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error>
                    {
                        Ok(Self {
                            bumps: borsh::BorshDeserialize::deserialize(buf)?,
                        })
                    }
                }
                let __Args { bumps } = __Args::deserialize(&mut ix_data)
                    .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
                let vault = &accounts[0];
                *accounts = &accounts[1..];
                let vault_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let lp_token_mint = &accounts[0];
                *accounts = &accounts[1..];
                let vault_reserve_token = &accounts[0];
                *accounts = &accounts[1..];
                let reserve_token_mint: Box<anchor_lang::Account<Mint>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let fee_receiver: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let referral_fee_receiver: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let referral_fee_owner: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let payer: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let owner: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let system_program: anchor_lang::Program<System> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let associated_token_program: anchor_lang::Program<AssociatedToken> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let rent: Sysvar<Rent> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let __anchor_rent = Rent::get()?;
                let lp_token_mint: Box<anchor_lang::Account<Mint>> = {
                    if !false
                        || lp_token_mint.to_account_info().owner
                            == &anchor_lang::solana_program::system_program::ID
                    {
                        let payer = payer.to_account_info();
                        let __current_lamports = lp_token_mint.to_account_info().lamports();
                        if __current_lamports == 0 {
                            let lamports =
                                __anchor_rent.minimum_balance(anchor_spl::token::Mint::LEN);
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::create_account(
                                    payer.to_account_info().key,
                                    lp_token_mint.to_account_info().key,
                                    lamports,
                                    anchor_spl::token::Mint::LEN as u64,
                                    token_program.to_account_info().key,
                                ),
                                &[
                                    payer.to_account_info(),
                                    lp_token_mint.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    b"lp_mint".as_ref(),
                                    &[bumps.lp_mint][..],
                                ][..]],
                            )?;
                        } else {
                            let required_lamports = __anchor_rent
                                .minimum_balance(anchor_spl::token::Mint::LEN)
                                .max(1)
                                .saturating_sub(__current_lamports);
                            if required_lamports > 0 {
                                anchor_lang::solana_program::program::invoke(
                                    &anchor_lang::solana_program::system_instruction::transfer(
                                        payer.to_account_info().key,
                                        lp_token_mint.to_account_info().key,
                                        required_lamports,
                                    ),
                                    &[
                                        payer.to_account_info(),
                                        lp_token_mint.to_account_info(),
                                        system_program.to_account_info(),
                                    ],
                                )?;
                            }
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::allocate(
                                    lp_token_mint.to_account_info().key,
                                    anchor_spl::token::Mint::LEN as u64,
                                ),
                                &[
                                    lp_token_mint.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    b"lp_mint".as_ref(),
                                    &[bumps.lp_mint][..],
                                ][..]],
                            )?;
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::assign(
                                    lp_token_mint.to_account_info().key,
                                    token_program.to_account_info().key,
                                ),
                                &[
                                    lp_token_mint.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    b"lp_mint".as_ref(),
                                    &[bumps.lp_mint][..],
                                ][..]],
                            )?;
                        }
                        let cpi_program = token_program.to_account_info();
                        let accounts = anchor_spl::token::InitializeMint {
                            mint: lp_token_mint.to_account_info(),
                            rent: rent.to_account_info(),
                        };
                        let cpi_ctx = CpiContext::new(cpi_program, accounts);
                        anchor_spl::token::initialize_mint(
                            cpi_ctx,
                            reserve_token_mint.decimals,
                            &vault_authority.to_account_info().key,
                            None,
                        )?;
                    }
                    let pa: Box<anchor_lang::Account<Mint>> =
                        Box::new(anchor_lang::Account::try_from_unchecked(&lp_token_mint)?);
                    pa
                };
                let (__program_signer, __bump) =
                    anchor_lang::solana_program::pubkey::Pubkey::find_program_address(
                        &[vault.key().as_ref(), b"lp_mint".as_ref()],
                        program_id,
                    );
                if lp_token_mint.to_account_info().key != &__program_signer {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if __bump != bumps.lp_mint {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if !lp_token_mint.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !__anchor_rent.is_exempt(
                    lp_token_mint.to_account_info().lamports(),
                    lp_token_mint.to_account_info().try_data_len()?,
                ) {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
                }
                let __anchor_rent = Rent::get()?;
                let vault_reserve_token: Box<anchor_lang::Account<TokenAccount>> = {
                    if !false
                        || vault_reserve_token.to_account_info().owner
                            == &anchor_lang::solana_program::system_program::ID
                    {
                        let payer = payer.to_account_info();
                        let __current_lamports = vault_reserve_token.to_account_info().lamports();
                        if __current_lamports == 0 {
                            let lamports =
                                __anchor_rent.minimum_balance(anchor_spl::token::TokenAccount::LEN);
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::create_account(
                                    payer.to_account_info().key,
                                    vault_reserve_token.to_account_info().key,
                                    lamports,
                                    anchor_spl::token::TokenAccount::LEN as u64,
                                    token_program.to_account_info().key,
                                ),
                                &[
                                    payer.to_account_info(),
                                    vault_reserve_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    reserve_token_mint.key().as_ref(),
                                    &[bumps.reserve][..],
                                ][..]],
                            )?;
                        } else {
                            let required_lamports = __anchor_rent
                                .minimum_balance(anchor_spl::token::TokenAccount::LEN)
                                .max(1)
                                .saturating_sub(__current_lamports);
                            if required_lamports > 0 {
                                anchor_lang::solana_program::program::invoke(
                                    &anchor_lang::solana_program::system_instruction::transfer(
                                        payer.to_account_info().key,
                                        vault_reserve_token.to_account_info().key,
                                        required_lamports,
                                    ),
                                    &[
                                        payer.to_account_info(),
                                        vault_reserve_token.to_account_info(),
                                        system_program.to_account_info(),
                                    ],
                                )?;
                            }
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::allocate(
                                    vault_reserve_token.to_account_info().key,
                                    anchor_spl::token::TokenAccount::LEN as u64,
                                ),
                                &[
                                    vault_reserve_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    reserve_token_mint.key().as_ref(),
                                    &[bumps.reserve][..],
                                ][..]],
                            )?;
                            anchor_lang::solana_program::program::invoke_signed(
                                &anchor_lang::solana_program::system_instruction::assign(
                                    vault_reserve_token.to_account_info().key,
                                    token_program.to_account_info().key,
                                ),
                                &[
                                    vault_reserve_token.to_account_info(),
                                    system_program.to_account_info(),
                                ],
                                &[&[
                                    vault.key().as_ref(),
                                    reserve_token_mint.key().as_ref(),
                                    &[bumps.reserve][..],
                                ][..]],
                            )?;
                        }
                        let cpi_program = token_program.to_account_info();
                        let accounts = anchor_spl::token::InitializeAccount {
                            account: vault_reserve_token.to_account_info(),
                            mint: reserve_token_mint.to_account_info(),
                            authority: vault_authority.to_account_info(),
                            rent: rent.to_account_info(),
                        };
                        let cpi_ctx = CpiContext::new(cpi_program, accounts);
                        anchor_spl::token::initialize_account(cpi_ctx)?;
                    }
                    let pa: Box<anchor_lang::Account<TokenAccount>> = Box::new(
                        anchor_lang::Account::try_from_unchecked(&vault_reserve_token)?,
                    );
                    pa
                };
                let (__program_signer, __bump) =
                    anchor_lang::solana_program::pubkey::Pubkey::find_program_address(
                        &[vault.key().as_ref(), reserve_token_mint.key().as_ref()],
                        program_id,
                    );
                if vault_reserve_token.to_account_info().key != &__program_signer {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if __bump != bumps.reserve {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if !vault_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !__anchor_rent.is_exempt(
                    vault_reserve_token.to_account_info().lamports(),
                    vault_reserve_token.to_account_info().try_data_len()?,
                ) {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
                }
                let __anchor_rent = Rent::get()?;
                let vault: Box<anchor_lang::Account<Vault>> = {
                    let mut __data: &[u8] = &vault.try_borrow_data()?;
                    let mut __disc_bytes = [0u8; 8];
                    __disc_bytes.copy_from_slice(&__data[..8]);
                    let __discriminator = u64::from_le_bytes(__disc_bytes);
                    if __discriminator != 0 {
                        return Err(anchor_lang::__private::ErrorCode::ConstraintZero.into());
                    }
                    Box::new(anchor_lang::Account::try_from_unchecked(&vault)?)
                };
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !__anchor_rent.is_exempt(
                    vault.to_account_info().lamports(),
                    vault.to_account_info().try_data_len()?,
                ) {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
                }
                let __program_signer = Pubkey::create_program_address(
                    &[
                        vault.key().as_ref(),
                        b"authority".as_ref(),
                        &[bumps.authority][..],
                    ][..],
                    program_id,
                )
                .map_err(|_| anchor_lang::__private::ErrorCode::ConstraintSeeds)?;
                if vault_authority.to_account_info().key != &__program_signer {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
                }
                if !vault_authority.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !fee_receiver.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !referral_fee_receiver.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !payer.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(Initialize {
                    vault,
                    vault_authority,
                    lp_token_mint,
                    vault_reserve_token,
                    reserve_token_mint,
                    fee_receiver,
                    referral_fee_receiver,
                    referral_fee_owner,
                    payer,
                    owner,
                    system_program,
                    token_program,
                    associated_token_program,
                    rent,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Initialize<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_authority.to_account_infos());
                account_infos.extend(self.lp_token_mint.to_account_infos());
                account_infos.extend(self.vault_reserve_token.to_account_infos());
                account_infos.extend(self.reserve_token_mint.to_account_infos());
                account_infos.extend(self.fee_receiver.to_account_infos());
                account_infos.extend(self.referral_fee_receiver.to_account_infos());
                account_infos.extend(self.referral_fee_owner.to_account_infos());
                account_infos.extend(self.payer.to_account_infos());
                account_infos.extend(self.owner.to_account_infos());
                account_infos.extend(self.system_program.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos.extend(self.associated_token_program.to_account_infos());
                account_infos.extend(self.rent.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Initialize<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_authority.to_account_metas(None));
                account_metas.extend(self.lp_token_mint.to_account_metas(None));
                account_metas.extend(self.vault_reserve_token.to_account_metas(None));
                account_metas.extend(self.reserve_token_mint.to_account_metas(None));
                account_metas.extend(self.fee_receiver.to_account_metas(None));
                account_metas.extend(self.referral_fee_receiver.to_account_metas(None));
                account_metas.extend(self.referral_fee_owner.to_account_metas(None));
                account_metas.extend(self.payer.to_account_metas(None));
                account_metas.extend(self.owner.to_account_metas(None));
                account_metas.extend(self.system_program.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas.extend(self.associated_token_program.to_account_metas(None));
                account_metas.extend(self.rent.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for Initialize<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_authority, program_id)?;
                anchor_lang::AccountsExit::exit(&self.lp_token_mint, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_reserve_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.fee_receiver, program_id)?;
                anchor_lang::AccountsExit::exit(&self.referral_fee_receiver, program_id)?;
                anchor_lang::AccountsExit::exit(&self.payer, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_initialize {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct Initialize {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub lp_token_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub reserve_token_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub fee_receiver: anchor_lang::solana_program::pubkey::Pubkey,
                pub referral_fee_receiver: anchor_lang::solana_program::pubkey::Pubkey,
                pub referral_fee_owner: anchor_lang::solana_program::pubkey::Pubkey,
                pub payer: anchor_lang::solana_program::pubkey::Pubkey,
                pub owner: anchor_lang::solana_program::pubkey::Pubkey,
                pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub associated_token_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub rent: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for Initialize
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.lp_token_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.reserve_token_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.fee_receiver, writer)?;
                    borsh::BorshSerialize::serialize(&self.referral_fee_receiver, writer)?;
                    borsh::BorshSerialize::serialize(&self.referral_fee_owner, writer)?;
                    borsh::BorshSerialize::serialize(&self.payer, writer)?;
                    borsh::BorshSerialize::serialize(&self.owner, writer)?;
                    borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.associated_token_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.rent, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for Initialize {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_authority,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.lp_token_mint,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_reserve_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.reserve_token_mint,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.fee_receiver,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.referral_fee_receiver,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.referral_fee_owner,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.payer, true,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.owner, false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.system_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.associated_token_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.rent, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_initialize {
            use super::*;
            pub struct Initialize<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub lp_token_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub reserve_token_mint:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub fee_receiver: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub referral_fee_receiver:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub referral_fee_owner:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub associated_token_program:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub rent: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for Initialize<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_authority),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.lp_token_mint),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_reserve_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.reserve_token_mint),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.fee_receiver),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.referral_fee_receiver),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.referral_fee_owner),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.payer),
                        true,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.owner),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.system_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.associated_token_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.rent),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for Initialize<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lp_token_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.reserve_token_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.fee_receiver,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.referral_fee_receiver,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.referral_fee_owner,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.payer));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.owner));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.system_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.associated_token_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.rent));
                    account_infos
                }
            }
        }
        impl<'info> Initialize<'info> {
            fn init_fee_receiver_create_context(
                &self,
                fee_token_account: AccountInfo<'info>,
                token_authority: AccountInfo<'info>,
            ) -> CpiContext<'_, '_, '_, 'info, Create<'info>> {
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Create {
                        payer: self.payer.to_account_info(),
                        associated_token: fee_token_account,
                        authority: token_authority,
                        mint: self.lp_token_mint.to_account_info(),
                        system_program: self.system_program.to_account_info(),
                        token_program: self.token_program.to_account_info(),
                        rent: self.rent.to_account_info(),
                    },
                )
            }
            fn validate_referral_token(&self) -> ProgramResult {
                let referral_fee_receiver = associated_token::get_associated_token_address(
                    &self.referral_fee_owner.key(),
                    &self.lp_token_mint.key(),
                );
                if referral_fee_receiver.ne(&self.referral_fee_receiver.key()) {
                    return Err(ProgramError::InvalidAccountData);
                }
                Ok(())
            }
        }
        pub fn handler(
            ctx: Context<Initialize>,
            bumps: InitBumpSeeds,
            config: VaultConfigArg,
        ) -> ProgramResult {
            let clock = Clock::get()?;
            ctx.accounts.validate_referral_token()?;
            let vault = &mut ctx.accounts.vault;
            vault.version = get_version_arr();
            vault.owner = ctx.accounts.owner.key();
            vault.vault_authority = ctx.accounts.vault_authority.key();
            vault.authority_seed = vault.key();
            vault.authority_bump = [bumps.authority];
            vault.vault_reserve_token = ctx.accounts.vault_reserve_token.key();
            vault.lp_token_mint = ctx.accounts.lp_token_mint.key();
            vault.reserve_token_mint = ctx.accounts.reserve_token_mint.key();
            vault.fee_receiver = ctx.accounts.fee_receiver.key();
            vault.referral_fee_receiver = ctx.accounts.referral_fee_receiver.key();
            vault.value = SlotTrackedValue {
                value: 0,
                last_update: LastUpdate::new(clock.slot),
            };
            vault.config = VaultConfig::new(config)?;
            associated_token::create(ctx.accounts.init_fee_receiver_create_context(
                ctx.accounts.fee_receiver.to_account_info(),
                ctx.accounts.owner.to_account_info(),
            ))?;
            associated_token::create(ctx.accounts.init_fee_receiver_create_context(
                ctx.accounts.referral_fee_receiver.to_account_info(),
                ctx.accounts.referral_fee_owner.to_account_info(),
            ))?;
            Ok(())
        }
        fn get_version_arr() -> [u8; 3] {
            [
                "3".parse::<u8>().expect("failed to parse major version"),
                "0".parse::<u8>().expect("failed to parse minor version"),
                "0".parse::<u8>().expect("failed to parse patch version"),
            ]
        }
    }
    pub mod init_yield_source {
        use anchor_lang::prelude::*;
        pub trait YieldSourceInitializer<'info> {
            fn initialize_yield_source(&mut self) -> ProgramResult;
        }
        pub fn handler<'info, T: YieldSourceInitializer<'info>>(
            ctx: Context<'_, '_, '_, 'info, T>,
            _bump: u8,
        ) -> ProgramResult {
            ctx.accounts.initialize_yield_source()
        }
    }
    pub mod rebalance {
        use std::{convert::TryFrom, ops::Deref};
        use boolinator::Boolinator;
        use strum::IntoEnumIterator;
        use anchor_lang::prelude::*;
        use port_anchor_adaptor::PortReserve;
        use solana_maths::Rate;
        use crate::{
            adapters::SolendReserve,
            asset_container::AssetContainer,
            errors::ErrorCode,
            impl_provider_index,
            reserves::{Provider, Reserves},
            state::*,
        };
        pub struct RebalanceEvent {
            vault: Pubkey,
        }
        impl borsh::ser::BorshSerialize for RebalanceEvent
        where
            Pubkey: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.vault, writer)?;
                Ok(())
            }
        }
        impl borsh::de::BorshDeserialize for RebalanceEvent
        where
            Pubkey: borsh::BorshDeserialize,
        {
            fn deserialize(
                buf: &mut &[u8],
            ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
                Ok(Self {
                    vault: borsh::BorshDeserialize::deserialize(buf)?,
                })
            }
        }
        impl anchor_lang::Event for RebalanceEvent {
            fn data(&self) -> Vec<u8> {
                let mut d = [120, 27, 117, 235, 104, 42, 132, 75].to_vec();
                d.append(&mut self.try_to_vec().unwrap());
                d
            }
        }
        impl anchor_lang::Discriminator for RebalanceEvent {
            fn discriminator() -> [u8; 8] {
                [120, 27, 117, 235, 104, 42, 132, 75]
            }
        }
        /// Used by the SDK to figure out the order in which reconcile TXs should be sent
        pub struct RebalanceDataEvent {
            solend: u64,
            port: u64,
            jet: u64,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::default::Default for RebalanceDataEvent {
            #[inline]
            fn default() -> RebalanceDataEvent {
                RebalanceDataEvent {
                    solend: ::core::default::Default::default(),
                    port: ::core::default::Default::default(),
                    jet: ::core::default::Default::default(),
                }
            }
        }
        impl borsh::ser::BorshSerialize for RebalanceDataEvent
        where
            u64: borsh::ser::BorshSerialize,
            u64: borsh::ser::BorshSerialize,
            u64: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.solend, writer)?;
                borsh::BorshSerialize::serialize(&self.port, writer)?;
                borsh::BorshSerialize::serialize(&self.jet, writer)?;
                Ok(())
            }
        }
        impl borsh::de::BorshDeserialize for RebalanceDataEvent
        where
            u64: borsh::BorshDeserialize,
            u64: borsh::BorshDeserialize,
            u64: borsh::BorshDeserialize,
        {
            fn deserialize(
                buf: &mut &[u8],
            ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
                Ok(Self {
                    solend: borsh::BorshDeserialize::deserialize(buf)?,
                    port: borsh::BorshDeserialize::deserialize(buf)?,
                    jet: borsh::BorshDeserialize::deserialize(buf)?,
                })
            }
        }
        impl anchor_lang::Event for RebalanceDataEvent {
            fn data(&self) -> Vec<u8> {
                let mut d = [68, 24, 236, 140, 184, 59, 30, 100].to_vec();
                d.append(&mut self.try_to_vec().unwrap());
                d
            }
        }
        impl anchor_lang::Discriminator for RebalanceDataEvent {
            fn discriminator() -> [u8; 8] {
                [68, 24, 236, 140, 184, 59, 30, 100]
            }
        }
        impl core::ops::Index<Provider> for RebalanceDataEvent {
            type Output = u64;
            fn index(&self, provider: Provider) -> &Self::Output {
                match provider {
                    Provider::Solend => &self.solend,
                    Provider::Port => &self.port,
                    Provider::Jet => &self.jet,
                }
            }
        }
        impl core::ops::IndexMut<Provider> for RebalanceDataEvent {
            fn index_mut(&mut self, provider: Provider) -> &mut Self::Output {
                match provider {
                    Provider::Solend => &mut self.solend,
                    Provider::Port => &mut self.port,
                    Provider::Jet => &mut self.jet,
                }
            }
        }
        impl From<&Allocations> for RebalanceDataEvent {
            fn from(allocations: &Allocations) -> Self {
                Provider::iter().fold(Self::default(), |mut acc, provider| {
                    acc[provider] = allocations[provider].value;
                    acc
                })
            }
        }
        pub struct Rebalance<'info> {
            /// Vault state account
            /// Checks that the refresh has been called in the same slot
            /// Checks that the accounts passed in are correct
            # [account (mut , constraint =! vault . value . last_update . is_stale (clock . slot) ? @ ErrorCode :: VaultIsNotRefreshed ,)]
            pub vault: Box<Account<'info, Vault>>,
            pub solend_reserve: AccountInfo<'info>,
            pub port_reserve: AccountInfo<'info>,
            pub jet_reserve: AccountInfo<'info>,
            pub clock: Sysvar<'info, Clock>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for Rebalance<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let solend_reserve: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let port_reserve: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let jet_reserve: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: Sysvar<Clock> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !(!vault.value.last_update.is_stale(clock.slot)?) {
                    return Err(ErrorCode::VaultIsNotRefreshed.into());
                }
                Ok(Rebalance {
                    vault,
                    solend_reserve,
                    port_reserve,
                    jet_reserve,
                    clock,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Rebalance<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.solend_reserve.to_account_infos());
                account_infos.extend(self.port_reserve.to_account_infos());
                account_infos.extend(self.jet_reserve.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Rebalance<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.solend_reserve.to_account_metas(None));
                account_metas.extend(self.port_reserve.to_account_metas(None));
                account_metas.extend(self.jet_reserve.to_account_metas(None));
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for Rebalance<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_rebalance {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct Rebalance {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub solend_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub port_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub jet_reserve: anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for Rebalance
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.solend_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.port_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.jet_reserve, writer)?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for Rebalance {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.solend_reserve,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.port_reserve,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.jet_reserve,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_rebalance {
            use super::*;
            pub struct Rebalance<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub solend_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub port_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub jet_reserve: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for Rebalance<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.solend_reserve),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.port_reserve),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.jet_reserve),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for Rebalance<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.solend_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.port_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.jet_reserve,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos
                }
            }
        }
        impl TryFrom<&Rebalance<'_>> for AssetContainer<Reserves> {
            type Error = ProgramError;
            fn try_from(r: &Rebalance<'_>) -> Result<AssetContainer<Reserves>, Self::Error> {
                let flags: YieldSourceFlags = r.vault.get_yield_source_flags();
                let solend = flags
                    .contains(YieldSourceFlags::SOLEND)
                    .as_option()
                    .map(|()| {
                        r.solend_reserve.key.eq(&r.vault.solend_reserve).as_result(
                            Account::<SolendReserve>::try_from(&r.solend_reserve),
                            ErrorCode::InvalidAccount,
                        )?
                    })
                    .transpose()?
                    .map(|a| Reserves::Solend(a.deref().clone()));
                let port = flags
                    .contains(YieldSourceFlags::PORT)
                    .as_option()
                    .map(|()| {
                        r.port_reserve.key.eq(&r.vault.port_reserve).as_result(
                            Account::<PortReserve>::try_from(&r.port_reserve),
                            ErrorCode::InvalidAccount,
                        )?
                    })
                    .transpose()?
                    .map(|a| Reserves::Port(a.deref().clone()));
                let jet = flags
                    .contains(YieldSourceFlags::JET)
                    .as_option()
                    .map(|()| {
                        r.jet_reserve.key.eq(&r.vault.jet_reserve).as_result(
                            Ok::<_, ProgramError>(Box::new(
                                *(AccountLoader::<jet::state::Reserve>::try_from(&r.jet_reserve)?)
                                    .load()?,
                            )),
                            ErrorCode::InvalidAccount,
                        )?
                    })
                    .transpose()?
                    .map(|a| Reserves::Jet(a));
                Ok(AssetContainer {
                    inner: [solend, port, jet],
                })
            }
        }
        pub struct StrategyWeightsArg {
            solend: u16,
            port: u16,
            jet: u16,
        }
        impl borsh::de::BorshDeserialize for StrategyWeightsArg
        where
            u16: borsh::BorshDeserialize,
            u16: borsh::BorshDeserialize,
            u16: borsh::BorshDeserialize,
        {
            fn deserialize(
                buf: &mut &[u8],
            ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
                Ok(Self {
                    solend: borsh::BorshDeserialize::deserialize(buf)?,
                    port: borsh::BorshDeserialize::deserialize(buf)?,
                    jet: borsh::BorshDeserialize::deserialize(buf)?,
                })
            }
        }
        impl borsh::ser::BorshSerialize for StrategyWeightsArg
        where
            u16: borsh::ser::BorshSerialize,
            u16: borsh::ser::BorshSerialize,
            u16: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.solend, writer)?;
                borsh::BorshSerialize::serialize(&self.port, writer)?;
                borsh::BorshSerialize::serialize(&self.jet, writer)?;
                Ok(())
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for StrategyWeightsArg {
            #[inline]
            fn clone(&self) -> StrategyWeightsArg {
                {
                    let _: ::core::clone::AssertParamIsClone<u16>;
                    let _: ::core::clone::AssertParamIsClone<u16>;
                    let _: ::core::clone::AssertParamIsClone<u16>;
                    *self
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for StrategyWeightsArg {}
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for StrategyWeightsArg {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match *self {
                    StrategyWeightsArg {
                        solend: ref __self_0_0,
                        port: ref __self_0_1,
                        jet: ref __self_0_2,
                    } => {
                        let debug_trait_builder =
                            &mut ::core::fmt::Formatter::debug_struct(f, "StrategyWeightsArg");
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "solend",
                            &&(*__self_0_0),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "port",
                            &&(*__self_0_1),
                        );
                        let _ = ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            "jet",
                            &&(*__self_0_2),
                        );
                        ::core::fmt::DebugStruct::finish(debug_trait_builder)
                    }
                }
            }
        }
        impl core::ops::Index<Provider> for StrategyWeightsArg {
            type Output = u16;
            fn index(&self, provider: Provider) -> &Self::Output {
                match provider {
                    Provider::Solend => &self.solend,
                    Provider::Port => &self.port,
                    Provider::Jet => &self.jet,
                }
            }
        }
        impl core::ops::IndexMut<Provider> for StrategyWeightsArg {
            fn index_mut(&mut self, provider: Provider) -> &mut Self::Output {
                match provider {
                    Provider::Solend => &mut self.solend,
                    Provider::Port => &mut self.port,
                    Provider::Jet => &mut self.jet,
                }
            }
        }
        impl From<StrategyWeightsArg> for AssetContainer<Rate> {
            fn from(s: StrategyWeightsArg) -> Self {
                Provider::iter().fold(Self::default(), |mut acc, provider| {
                    acc[provider] = Some(Rate::from_bips(s[provider] as u64));
                    acc
                })
            }
        }
        /// Calculate and store optimal allocations to downstream lending markets
        pub fn handler(
            ctx: Context<Rebalance>,
            proposed_weights_arg: StrategyWeightsArg,
        ) -> ProgramResult {
            ::solana_program::log::sol_log("Rebalancing");
            let vault_value = ctx.accounts.vault.value.value;
            let slot = Clock::get()?.slot;
            let assets = Box::new(AssetContainer::try_from(&*ctx.accounts)?);
            let strategy_weights = assets.calculate_weights(
                ctx.accounts.vault.config.strategy_type,
                ctx.accounts.vault.config.allocation_cap_pct,
            )?;
            AssetContainer::<u64>::try_from_weights(&strategy_weights, vault_value)
                .and_then(
                    |strategy_allocations| match ctx.accounts.vault.config.rebalance_mode {
                        RebalanceMode::ProofChecker => {
                            let proposed_weights =
                                AssetContainer::<Rate>::from(proposed_weights_arg);
                            let proposed_allocations = AssetContainer::<u64>::try_from_weights(
                                &strategy_weights,
                                vault_value,
                            )?;
                            ::solana_program::log::sol_log(&{
                                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                    &["Running as proof checker with proposed weights: "],
                                    &match (&proposed_weights.inner,) {
                                        _args => [::core::fmt::ArgumentV1::new(
                                            _args.0,
                                            ::core::fmt::Debug::fmt,
                                        )],
                                    },
                                ));
                                res
                            });
                            proposed_weights
                                .verify_weights(ctx.accounts.vault.config.allocation_cap_pct)?;
                            let proposed_apr =
                                assets.get_apr(&proposed_weights, &proposed_allocations)?;
                            let proof_apr =
                                assets.get_apr(&strategy_weights, &strategy_allocations)?;
                            ::solana_program::log::sol_log(&{
                                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                    &["Proposed APR: ", "\nProof APR: "],
                                    &match (&proposed_apr, &proof_apr) {
                                        _args => [
                                            ::core::fmt::ArgumentV1::new(
                                                _args.0,
                                                ::core::fmt::Debug::fmt,
                                            ),
                                            ::core::fmt::ArgumentV1::new(
                                                _args.1,
                                                ::core::fmt::Debug::fmt,
                                            ),
                                        ],
                                    },
                                ));
                                res
                            });
                            (proposed_apr >= proof_apr).as_result(
                                proposed_allocations,
                                ErrorCode::RebalanceProofCheckFailed.into(),
                            )
                        }
                        RebalanceMode::Calculator => {
                            ::solana_program::log::sol_log("Running as calculator");
                            Ok(strategy_allocations)
                        }
                    },
                )
                .map(|final_allocations_container| {
                    let final_allocations =
                        Allocations::from_container(final_allocations_container, slot);
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Final allocations: "],
                            &match (&final_allocations,) {
                                _args => [::core::fmt::ArgumentV1::new(
                                    _args.0,
                                    ::core::fmt::Debug::fmt,
                                )],
                            },
                        ));
                        res
                    });
                    {
                        let data = anchor_lang::Event::data(&RebalanceEvent {
                            vault: ctx.accounts.vault.key(),
                        });
                        let msg_str = &anchor_lang::__private::base64::encode(data);
                        ::solana_program::log::sol_log(msg_str);
                    };
                    {
                        let data =
                            anchor_lang::Event::data(&RebalanceDataEvent::from(&final_allocations));
                        let msg_str = &anchor_lang::__private::base64::encode(data);
                        ::solana_program::log::sol_log(msg_str);
                    };
                    ctx.accounts.vault.target_allocations = final_allocations;
                })
        }
    }
    pub mod reconcile {
        use std::cmp;
        use anchor_lang::prelude::*;
        use boolinator::Boolinator;
        use crate::{
            errors::ErrorCode,
            reserves::Provider,
            state::{Vault, VaultFlags},
        };
        const MAX_SLOTS_SINCE_ALLOC_UPDATE: u64 = 100;
        pub trait LendingMarket {
            fn deposit(&self, amount: u64) -> ProgramResult;
            fn redeem(&self, amount: u64) -> ProgramResult;
            fn convert_amount_reserve_to_lp(&self, amount: u64) -> Result<u64, ProgramError>;
            fn convert_amount_lp_to_reserve(&self, amount: u64) -> Result<u64, ProgramError>;
            fn reserve_tokens_in_vault(&self) -> u64;
            fn lp_tokens_in_vault(&self) -> u64;
            fn provider(&self) -> Provider;
        }
        pub trait HasVault {
            fn vault(&self) -> &Vault;
            fn vault_mut(&mut self) -> &mut Vault;
        }
        pub fn handler<T: LendingMarket + HasVault>(
            ctx: Context<T>,
            withdraw_option: u64,
        ) -> ProgramResult {
            (!ctx
                .accounts
                .vault()
                .get_halt_flags()
                .contains(VaultFlags::HALT_RECONCILES))
            .ok_or::<ProgramError>(ErrorCode::HaltedVault.into())?;
            let provider = ctx.accounts.provider();
            match withdraw_option {
                0 => {
                    let lp_tokens_in_vault = ctx.accounts.lp_tokens_in_vault();
                    let current_value = ctx
                        .accounts
                        .convert_amount_lp_to_reserve(lp_tokens_in_vault)?;
                    let allocation = ctx.accounts.vault().target_allocations[provider];
                    #[cfg(feature = "debug")]
                    {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Desired allocation: "],
                                &match (&allocation.value,) {
                                    _args => [::core::fmt::ArgumentV1::new(
                                        _args.0,
                                        ::core::fmt::Display::fmt,
                                    )],
                                },
                            ));
                            res
                        });
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Current allocation: "],
                                &match (&current_value,) {
                                    _args => [::core::fmt::ArgumentV1::new(
                                        _args.0,
                                        ::core::fmt::Display::fmt,
                                    )],
                                },
                            ));
                            res
                        });
                    }
                    let clock = Clock::get()?;
                    if allocation.last_update.slots_elapsed(clock.slot)?
                        > MAX_SLOTS_SINCE_ALLOC_UPDATE
                    {
                        return Err(ErrorCode::AllocationIsNotUpdated.into());
                    }
                    match allocation.value.checked_sub(current_value) {
                        Some(tokens_to_deposit) => {
                            let tokens_to_deposit_checked =
                                cmp::min(tokens_to_deposit, ctx.accounts.reserve_tokens_in_vault());
                            ::solana_program::log::sol_log(&{
                                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                    &["Depositing "],
                                    &match (&tokens_to_deposit_checked,) {
                                        _args => [::core::fmt::ArgumentV1::new(
                                            _args.0,
                                            ::core::fmt::Display::fmt,
                                        )],
                                    },
                                ));
                                res
                            });
                            ctx.accounts.deposit(tokens_to_deposit_checked)?;
                        }
                        None => {
                            let tokens_to_redeem = ctx
                                .accounts
                                .lp_tokens_in_vault()
                                .checked_sub(
                                    ctx.accounts
                                        .convert_amount_reserve_to_lp(allocation.value)?,
                                )
                                .ok_or(ErrorCode::MathError)?;
                            ::solana_program::log::sol_log(&{
                                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                    &["Redeeming "],
                                    &match (&tokens_to_redeem,) {
                                        _args => [::core::fmt::ArgumentV1::new(
                                            _args.0,
                                            ::core::fmt::Display::fmt,
                                        )],
                                    },
                                ));
                                res
                            });
                            ctx.accounts.redeem(tokens_to_redeem)?;
                        }
                    }
                    ctx.accounts.vault_mut().target_allocations[provider].reset();
                }
                _ => {
                    let tokens_to_redeem =
                        ctx.accounts.convert_amount_reserve_to_lp(withdraw_option)?;
                    let tokens_to_redeem_checked =
                        cmp::min(tokens_to_redeem, ctx.accounts.lp_tokens_in_vault());
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Redeeming "],
                            &match (&tokens_to_redeem_checked,) {
                                _args => [::core::fmt::ArgumentV1::new(
                                    _args.0,
                                    ::core::fmt::Display::fmt,
                                )],
                            },
                        ));
                        res
                    });
                    ctx.accounts.redeem(tokens_to_redeem_checked)?;
                }
            }
            Ok(())
        }
    }
    pub mod refresh {
        #![allow(dead_code)]
        #![allow(unused_imports)]
        use anchor_lang::prelude::*;
        use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
        use port_anchor_adaptor::{port_lending_id, PortReserve};
        use crate::{
            adapters::{solend, SolendReserve},
            errors::ErrorCode,
        };
        pub trait Refresher<'info> {
            fn update_actual_allocation(
                &mut self,
                remaining_accounts: &[AccountInfo<'info>],
            ) -> ProgramResult;
        }
        /// Refreshes the reserves of downstream lending markets
        pub fn handler<'info, T: Refresher<'info>>(
            ctx: Context<'_, '_, '_, 'info, T>,
        ) -> ProgramResult {
            ::solana_program::log::sol_log("Refreshing yield source");
            ctx.accounts
                .update_actual_allocation(ctx.remaining_accounts)
        }
    }
    pub mod update_config {
        use anchor_lang::prelude::*;
        use std::convert::Into;
        use crate::state::{Vault, VaultConfig};
        use super::VaultConfigArg;
        pub struct UpdateConfig<'info> {
            # [account (mut , has_one = owner ,)]
            pub vault: Box<Account<'info, Vault>>,
            pub owner: Signer<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for UpdateConfig<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let owner: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.owner != owner.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                Ok(UpdateConfig { vault, owner })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateConfig<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.owner.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for UpdateConfig<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.owner.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for UpdateConfig<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_update_config {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct UpdateConfig {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub owner: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for UpdateConfig
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.owner, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for UpdateConfig {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.owner, true,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_update_config {
            use super::*;
            pub struct UpdateConfig<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for UpdateConfig<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.owner),
                            true,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateConfig<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.owner));
                    account_infos
                }
            }
        }
        pub fn handler(ctx: Context<UpdateConfig>, config: VaultConfigArg) -> ProgramResult {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["New config: "],
                    &match (&config,) {
                        _args => [::core::fmt::ArgumentV1::new(
                            _args.0,
                            ::core::fmt::Debug::fmt,
                        )],
                    },
                ));
                res
            });
            ctx.accounts.vault.config = VaultConfig::new(config)?;
            ctx.accounts.vault.adjust_allocation_cap()
        }
    }
    pub mod update_halt_flags {
        use anchor_lang::prelude::*;
        use std::convert::Into;
        use crate::state::Vault;
        pub struct UpdateHaltFlags<'info> {
            # [account (mut , has_one = owner ,)]
            pub vault: Box<Account<'info, Vault>>,
            pub owner: Signer<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for UpdateHaltFlags<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let owner: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.owner != owner.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                Ok(UpdateHaltFlags { vault, owner })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateHaltFlags<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.owner.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for UpdateHaltFlags<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.owner.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for UpdateHaltFlags<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_update_halt_flags {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct UpdateHaltFlags {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub owner: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for UpdateHaltFlags
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.owner, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for UpdateHaltFlags {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.owner, true,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_update_halt_flags {
            use super::*;
            pub struct UpdateHaltFlags<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for UpdateHaltFlags<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.owner),
                            true,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateHaltFlags<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.owner));
                    account_infos
                }
            }
        }
        pub fn handler(ctx: Context<UpdateHaltFlags>, flags: u16) -> ProgramResult {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["New flags: "],
                    &match (&flags,) {
                        _args => [::core::fmt::ArgumentV1::new(
                            _args.0,
                            ::core::fmt::Debug::fmt,
                        )],
                    },
                ));
                res
            });
            ctx.accounts.vault.set_halt_flags(flags)
        }
    }
    pub mod withdraw {
        use std::convert::Into;
        use boolinator::Boolinator;
        use anchor_lang::prelude::*;
        use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};
        use crate::{
            errors::ErrorCode,
            state::{Vault, VaultFlags},
        };
        pub struct WithdrawEvent {
            vault: Pubkey,
            user: Pubkey,
            amount: u64,
        }
        impl borsh::ser::BorshSerialize for WithdrawEvent
        where
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            u64: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.vault, writer)?;
                borsh::BorshSerialize::serialize(&self.user, writer)?;
                borsh::BorshSerialize::serialize(&self.amount, writer)?;
                Ok(())
            }
        }
        impl borsh::de::BorshDeserialize for WithdrawEvent
        where
            Pubkey: borsh::BorshDeserialize,
            Pubkey: borsh::BorshDeserialize,
            u64: borsh::BorshDeserialize,
        {
            fn deserialize(
                buf: &mut &[u8],
            ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
                Ok(Self {
                    vault: borsh::BorshDeserialize::deserialize(buf)?,
                    user: borsh::BorshDeserialize::deserialize(buf)?,
                    amount: borsh::BorshDeserialize::deserialize(buf)?,
                })
            }
        }
        impl anchor_lang::Event for WithdrawEvent {
            fn data(&self) -> Vec<u8> {
                let mut d = [22, 9, 133, 26, 160, 44, 71, 192].to_vec();
                d.append(&mut self.try_to_vec().unwrap());
                d
            }
        }
        impl anchor_lang::Discriminator for WithdrawEvent {
            fn discriminator() -> [u8; 8] {
                [22, 9, 133, 26, 160, 44, 71, 192]
            }
        }
        pub struct Withdraw<'info> {
            /// Vault state account
            /// Checks that the refresh has been called in the same slot
            /// Checks that the accounts passed in are correct
            # [account (mut , constraint =! vault . value . last_update . is_stale (clock . slot) ? @ ErrorCode :: VaultIsNotRefreshed , has_one = vault_authority , has_one = vault_reserve_token , has_one = lp_token_mint ,)]
            pub vault: Box<Account<'info, Vault>>,
            /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
            pub vault_authority: AccountInfo<'info>,
            /// Token account for the vault's reserve tokens
            #[account(mut)]
            pub vault_reserve_token: Box<Account<'info, TokenAccount>>,
            /// Mint for the vault's lp token
            #[account(mut)]
            pub lp_token_mint: Box<Account<'info, Mint>>,
            /// Token account from which lp tokens are burned
            #[account(mut)]
            pub user_lp_token: Box<Account<'info, TokenAccount>>,
            /// Account where vault LP tokens are transferred to
            #[account(mut)]
            pub user_reserve_token: Box<Account<'info, TokenAccount>>,
            /// Authority of the user_lp_token account
            /// Must be a signer
            pub user_authority: Signer<'info>,
            pub token_program: Program<'info, Token>,
            pub clock: Sysvar<'info, Clock>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info> for Withdraw<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
                ix_data: &[u8],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
            {
                let vault: Box<anchor_lang::Account<Vault>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_authority: AccountInfo =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let vault_reserve_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let lp_token_mint: Box<anchor_lang::Account<Mint>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let user_lp_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let user_reserve_token: Box<anchor_lang::Account<TokenAccount>> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let user_authority: Signer =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let token_program: anchor_lang::Program<Token> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                let clock: Sysvar<Clock> =
                    anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
                if !vault.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if &vault.vault_authority != vault_authority.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.vault_reserve_token != vault_reserve_token.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if &vault.lp_token_mint != lp_token_mint.to_account_info().key {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
                }
                if !(!vault.value.last_update.is_stale(clock.slot)?) {
                    return Err(ErrorCode::VaultIsNotRefreshed.into());
                }
                if !vault_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !lp_token_mint.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !user_lp_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                if !user_reserve_token.to_account_info().is_writable {
                    return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
                }
                Ok(Withdraw {
                    vault,
                    vault_authority,
                    vault_reserve_token,
                    lp_token_mint,
                    user_lp_token,
                    user_reserve_token,
                    user_authority,
                    token_program,
                    clock,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Withdraw<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.vault.to_account_infos());
                account_infos.extend(self.vault_authority.to_account_infos());
                account_infos.extend(self.vault_reserve_token.to_account_infos());
                account_infos.extend(self.lp_token_mint.to_account_infos());
                account_infos.extend(self.user_lp_token.to_account_infos());
                account_infos.extend(self.user_reserve_token.to_account_infos());
                account_infos.extend(self.user_authority.to_account_infos());
                account_infos.extend(self.token_program.to_account_infos());
                account_infos.extend(self.clock.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Withdraw<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.vault.to_account_metas(None));
                account_metas.extend(self.vault_authority.to_account_metas(None));
                account_metas.extend(self.vault_reserve_token.to_account_metas(None));
                account_metas.extend(self.lp_token_mint.to_account_metas(None));
                account_metas.extend(self.user_lp_token.to_account_metas(None));
                account_metas.extend(self.user_reserve_token.to_account_metas(None));
                account_metas.extend(self.user_authority.to_account_metas(None));
                account_metas.extend(self.token_program.to_account_metas(None));
                account_metas.extend(self.clock.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for Withdraw<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                anchor_lang::AccountsExit::exit(&self.vault, program_id)?;
                anchor_lang::AccountsExit::exit(&self.vault_reserve_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.lp_token_mint, program_id)?;
                anchor_lang::AccountsExit::exit(&self.user_lp_token, program_id)?;
                anchor_lang::AccountsExit::exit(&self.user_reserve_token, program_id)?;
                Ok(())
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_withdraw {
            use super::*;
            use anchor_lang::prelude::borsh;
            pub struct Withdraw {
                pub vault: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub vault_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub lp_token_mint: anchor_lang::solana_program::pubkey::Pubkey,
                pub user_lp_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub user_reserve_token: anchor_lang::solana_program::pubkey::Pubkey,
                pub user_authority: anchor_lang::solana_program::pubkey::Pubkey,
                pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
                pub clock: anchor_lang::solana_program::pubkey::Pubkey,
            }
            impl borsh::ser::BorshSerialize for Withdraw
            where
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
                anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.vault, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.vault_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.lp_token_mint, writer)?;
                    borsh::BorshSerialize::serialize(&self.user_lp_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.user_reserve_token, writer)?;
                    borsh::BorshSerialize::serialize(&self.user_authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.token_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.clock, writer)?;
                    Ok(())
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for Withdraw {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.vault_authority,
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.vault_reserve_token,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.lp_token_mint,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.user_lp_token,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.user_reserve_token,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.user_authority,
                            true,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.token_program,
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.clock, false,
                        ),
                    );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `cpi::accounts` module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_withdraw {
            use super::*;
            pub struct Withdraw<'info> {
                pub vault: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub vault_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub lp_token_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub user_lp_token: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub user_reserve_token:
                    anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub user_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub clock: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for Withdraw<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.vault_authority),
                            false,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.vault_reserve_token),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.lp_token_mint),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.user_lp_token),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.user_reserve_token),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.user_authority),
                            true,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.token_program),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.clock),
                            false,
                        ),
                    );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for Withdraw<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.vault));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.vault_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.lp_token_mint,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.user_lp_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.user_reserve_token,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.user_authority,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(
                        &self.token_program,
                    ));
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.clock));
                    account_infos
                }
            }
        }
        impl<'info> Withdraw<'info> {
            /// CpiContext for burning vault lp tokens from user account
            fn burn_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Burn {
                        mint: self.lp_token_mint.to_account_info(),
                        to: self.user_lp_token.to_account_info(),
                        authority: self.user_authority.to_account_info(),
                    },
                )
            }
            /// CpiContext for transferring reserve tokens from vault to user
            fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.vault_reserve_token.to_account_info(),
                        to: self.user_reserve_token.to_account_info(),
                        authority: self.vault_authority.clone(),
                    },
                )
            }
        }
        /// Withdraw from the vault
        ///
        /// Burns the user's lp tokens and transfers their share of reserve tokens
        pub fn handler(ctx: Context<Withdraw>, lp_token_amount: u64) -> ProgramResult {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Withdrawing ", " lp tokens"],
                    &match (&lp_token_amount,) {
                        _args => [::core::fmt::ArgumentV1::new(
                            _args.0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            });
            (!ctx
                .accounts
                .vault
                .get_halt_flags()
                .contains(VaultFlags::HALT_DEPOSITS_WITHDRAWS))
            .ok_or::<ProgramError>(ErrorCode::HaltedVault.into())?;
            let vault = &ctx.accounts.vault;
            let reserve_tokens_to_transfer = crate::math::calc_lp_to_reserve(
                lp_token_amount,
                ctx.accounts.lp_token_mint.supply,
                vault.value.value,
            )
            .ok_or(ErrorCode::MathError)?;
            token::burn(ctx.accounts.burn_context(), lp_token_amount)?;
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Transferring ", " reserve tokens"],
                    &match (&reserve_tokens_to_transfer,) {
                        _args => [::core::fmt::ArgumentV1::new(
                            _args.0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            });
            token::transfer(
                ctx.accounts
                    .transfer_context()
                    .with_signer(&[&vault.authority_seeds()]),
                reserve_tokens_to_transfer,
            )?;
            ctx.accounts.vault.value.value = ctx
                .accounts
                .vault
                .value
                .value
                .checked_sub(reserve_tokens_to_transfer)
                .ok_or(ErrorCode::MathError)?;
            {
                let data = anchor_lang::Event::data(&WithdrawEvent {
                    vault: ctx.accounts.vault.key(),
                    user: ctx.accounts.user_authority.key(),
                    amount: lp_token_amount,
                });
                let msg_str = &anchor_lang::__private::base64::encode(data);
                ::solana_program::log::sol_log(msg_str);
            };
            Ok(())
        }
    }
    pub use consolidate_refresh::*;
    pub use deposit::*;
    pub use init_vault::*;
    pub use init_yield_source::*;
    pub use rebalance::*;
    pub use reconcile::*;
    pub use refresh::*;
    pub use update_config::*;
    pub use update_halt_flags::*;
    pub use withdraw::*;
}
pub mod math {
    use std::convert::TryFrom;
    use anchor_lang::{
        prelude::ProgramError,
        solana_program::clock::{
            DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY,
        },
    };
    use spl_math::precise_number::PreciseNumber;
    use crate::errors::ErrorCode;
    pub const INITIAL_COLLATERAL_RATIO: u64 = 1;
    pub fn calc_reserve_to_lp(
        reserve_token_amount: u64,
        lp_token_supply: u64,
        reserve_tokens_in_vault: u64,
    ) -> Option<u64> {
        match reserve_tokens_in_vault {
            0 => Some(INITIAL_COLLATERAL_RATIO.checked_mul(reserve_token_amount)?),
            _ => {
                let reserve_token_amount = PreciseNumber::new(reserve_token_amount as u128)?;
                let lp_token_supply = PreciseNumber::new(lp_token_supply as u128)?;
                let reserve_tokens_in_vault = PreciseNumber::new(reserve_tokens_in_vault as u128)?;
                let lp_tokens_to_mint = lp_token_supply
                    .checked_mul(&reserve_token_amount)?
                    .checked_div(&reserve_tokens_in_vault)?
                    .floor()?
                    .to_imprecise()?;
                u64::try_from(lp_tokens_to_mint).ok()
            }
        }
    }
    pub fn calc_lp_to_reserve(
        lp_token_amount: u64,
        lp_token_supply: u64,
        reserve_tokens_in_vault: u64,
    ) -> Option<u64> {
        let lp_token_amount = PreciseNumber::new(lp_token_amount as u128)?;
        let lp_token_supply = PreciseNumber::new(lp_token_supply as u128)?;
        let reserve_tokens_in_vault = PreciseNumber::new(reserve_tokens_in_vault as u128)?;
        let reserve_tokens_to_transfer = lp_token_amount
            .checked_mul(&reserve_tokens_in_vault)?
            .checked_div(&lp_token_supply)?
            .floor()?
            .to_imprecise()?;
        u64::try_from(reserve_tokens_to_transfer).ok()
    }
    /// Number of slots per year
    /// 63072000
    pub const SLOTS_PER_YEAR: u64 =
        DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;
    pub const ONE_AS_BPS: u64 = 10000;
    pub fn calc_carry_fees(profit: u64, fee_bps: u64) -> Result<u64, ProgramError> {
        profit
            .checked_mul(fee_bps)
            .map(|n| n / ONE_AS_BPS)
            .ok_or_else(|| ErrorCode::OverflowError.into())
    }
    pub fn calc_mgmt_fees(aum: u64, fee_bps: u64, slots_elapsed: u64) -> Result<u64, ProgramError> {
        [fee_bps, slots_elapsed]
            .iter()
            .try_fold(aum, |acc, r| acc.checked_mul(*r))
            .map(|n| n / ONE_AS_BPS / SLOTS_PER_YEAR)
            .ok_or_else(|| ErrorCode::OverflowError.into())
    }
}
pub mod reserves {
    use anchor_lang::prelude::*;
    use port_anchor_adaptor::PortReserve;
    use solana_maths::{Rate, TryMul};
    use strum_macros::{EnumCount, EnumIter};
    use crate::adapters::solend::SolendReserve;
    pub enum Provider {
        Solend = 0,
        Port,
        Jet,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Provider {
        #[inline]
        fn clone(&self) -> Provider {
            {
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Provider {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Provider {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&Provider::Solend,) => ::core::fmt::Formatter::write_str(f, "Solend"),
                (&Provider::Port,) => ::core::fmt::Formatter::write_str(f, "Port"),
                (&Provider::Jet,) => ::core::fmt::Formatter::write_str(f, "Jet"),
            }
        }
    }
    ///An iterator over the variants of [Self]
    pub struct ProviderIter {
        idx: usize,
        back_idx: usize,
        marker: ::core::marker::PhantomData<()>,
    }
    impl ProviderIter {
        fn get(&self, idx: usize) -> Option<Provider> {
            match idx {
                0usize => ::core::option::Option::Some(Provider::Solend),
                1usize => ::core::option::Option::Some(Provider::Port),
                2usize => ::core::option::Option::Some(Provider::Jet),
                _ => ::core::option::Option::None,
            }
        }
    }
    impl ::strum::IntoEnumIterator for Provider {
        type Iterator = ProviderIter;
        fn iter() -> ProviderIter {
            ProviderIter {
                idx: 0,
                back_idx: 0,
                marker: ::core::marker::PhantomData,
            }
        }
    }
    impl Iterator for ProviderIter {
        type Item = Provider;
        fn next(&mut self) -> Option<<Self as Iterator>::Item> {
            self.nth(0)
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let t = if self.idx + self.back_idx >= 3usize {
                0
            } else {
                3usize - self.idx - self.back_idx
            };
            (t, Some(t))
        }
        fn nth(&mut self, n: usize) -> Option<<Self as Iterator>::Item> {
            let idx = self.idx + n + 1;
            if idx + self.back_idx > 3usize {
                self.idx = 3usize;
                None
            } else {
                self.idx = idx;
                self.get(idx - 1)
            }
        }
    }
    impl ExactSizeIterator for ProviderIter {
        fn len(&self) -> usize {
            self.size_hint().0
        }
    }
    impl DoubleEndedIterator for ProviderIter {
        fn next_back(&mut self) -> Option<<Self as Iterator>::Item> {
            let back_idx = self.back_idx + 1;
            if self.idx + back_idx > 3usize {
                self.back_idx = 3usize;
                None
            } else {
                self.back_idx = back_idx;
                self.get(3usize - self.back_idx)
            }
        }
    }
    impl Clone for ProviderIter {
        fn clone(&self) -> ProviderIter {
            ProviderIter {
                idx: self.idx,
                back_idx: self.back_idx,
                marker: self.marker.clone(),
            }
        }
    }
    impl ::strum::EnumCount for Provider {
        const COUNT: usize = 3usize;
    }
    impl ::core::marker::StructuralPartialEq for Provider {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for Provider {
        #[inline]
        fn eq(&self, other: &Provider) -> bool {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        _ => true,
                    }
                } else {
                    false
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Ord for Provider {
        #[inline]
        fn cmp(&self, other: &Provider) -> ::core::cmp::Ordering {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        _ => ::core::cmp::Ordering::Equal,
                    }
                } else {
                    ::core::cmp::Ord::cmp(&__self_vi, &__arg_1_vi)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::hash::Hash for Provider {
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            match (&*self,) {
                _ => ::core::hash::Hash::hash(&::core::intrinsics::discriminant_value(self), state),
            }
        }
    }
    impl ::core::marker::StructuralEq for Provider {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for Provider {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {
            {}
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialOrd for Provider {
        #[inline]
        fn partial_cmp(&self, other: &Provider) -> ::core::option::Option<::core::cmp::Ordering> {
            {
                let __self_vi = ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi = ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        _ => ::core::option::Option::Some(::core::cmp::Ordering::Equal),
                    }
                } else {
                    ::core::cmp::PartialOrd::partial_cmp(&__self_vi, &__arg_1_vi)
                }
            }
        }
    }
    impl borsh::ser::BorshSerialize for Provider {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> core::result::Result<(), borsh::maybestd::io::Error> {
            match self {
                Provider::Solend => {
                    let variant_idx: u8 = 0u8;
                    writer.write_all(&variant_idx.to_le_bytes())?;
                }
                Provider::Port => {
                    let variant_idx: u8 = 1u8;
                    writer.write_all(&variant_idx.to_le_bytes())?;
                }
                Provider::Jet => {
                    let variant_idx: u8 = 2u8;
                    writer.write_all(&variant_idx.to_le_bytes())?;
                }
            }
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for Provider {
        fn deserialize(buf: &mut &[u8]) -> core::result::Result<Self, borsh::maybestd::io::Error> {
            let variant_idx: u8 = borsh::BorshDeserialize::deserialize(buf)?;
            let return_value = match variant_idx {
                0u8 => Provider::Solend,
                1u8 => Provider::Port,
                2u8 => Provider::Jet,
                _ => {
                    let msg = {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Unexpected variant index: "],
                            &match (&variant_idx,) {
                                _args => [::core::fmt::ArgumentV1::new(
                                    _args.0,
                                    ::core::fmt::Debug::fmt,
                                )],
                            },
                        ));
                        res
                    };
                    return Err(borsh::maybestd::io::Error::new(
                        borsh::maybestd::io::ErrorKind::InvalidInput,
                        msg,
                    ));
                }
            };
            Ok(return_value)
        }
    }
    pub trait ReserveAccessor {
        fn utilization_rate(&self) -> Result<Rate, ProgramError>;
        fn borrow_rate(&self) -> Result<Rate, ProgramError>;
        fn reserve_with_deposit(
            &self,
            allocation: u64,
        ) -> Result<Box<dyn ReserveAccessor>, ProgramError>;
    }
    pub trait ReturnCalculator {
        fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError>;
    }
    impl<T> ReturnCalculator for T
    where
        T: ReserveAccessor,
    {
        fn calculate_return(&self, allocation: u64) -> Result<Rate, ProgramError> {
            let reserve = self.reserve_with_deposit(allocation)?;
            reserve.utilization_rate()?.try_mul(reserve.borrow_rate()?)
        }
    }
    pub enum Reserves {
        Solend(SolendReserve),
        Port(PortReserve),
        Jet(Box<jet::state::Reserve>),
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Reserves {
        #[inline]
        fn clone(&self) -> Reserves {
            match (&*self,) {
                (&Reserves::Solend(ref __self_0),) => {
                    Reserves::Solend(::core::clone::Clone::clone(&(*__self_0)))
                }
                (&Reserves::Port(ref __self_0),) => {
                    Reserves::Port(::core::clone::Clone::clone(&(*__self_0)))
                }
                (&Reserves::Jet(ref __self_0),) => {
                    Reserves::Jet(::core::clone::Clone::clone(&(*__self_0)))
                }
            }
        }
    }
    impl<'a> ReserveAccessor for Reserves {
        fn utilization_rate(&self) -> Result<Rate, ProgramError> {
            match self {
                Reserves::Solend(reserve) => reserve.utilization_rate(),
                Reserves::Port(reserve) => reserve.utilization_rate(),
                Reserves::Jet(reserve) => reserve.utilization_rate(),
            }
        }
        fn borrow_rate(&self) -> Result<Rate, ProgramError> {
            match self {
                Reserves::Solend(reserve) => reserve.borrow_rate(),
                Reserves::Port(reserve) => reserve.borrow_rate(),
                Reserves::Jet(reserve) => reserve.borrow_rate(),
            }
        }
        fn reserve_with_deposit(
            &self,
            allocation: u64,
        ) -> Result<Box<dyn ReserveAccessor>, ProgramError> {
            match self {
                Reserves::Solend(reserve) => reserve.reserve_with_deposit(allocation),
                Reserves::Port(reserve) => reserve.reserve_with_deposit(allocation),
                Reserves::Jet(reserve) => reserve.reserve_with_deposit(allocation),
            }
        }
    }
}
pub mod state {
    use std::cmp::Ordering;
    use core::convert::TryFrom;
    use strum::IntoEnumIterator;
    use anchor_lang::prelude::*;
    use jet_proto_proc_macros::assert_size;
    use crate::{
        asset_container::AssetContainer,
        errors::ErrorCode,
        impl_provider_index,
        instructions::VaultConfigArg,
        math::{calc_carry_fees, calc_mgmt_fees},
        reserves::Provider,
    };
    #[allow(unknown_lints, eq_op)]
    const _: [(); 0 - !{
        const ASSERT: bool = 768usize == std::mem::size_of::<Vault>();
        ASSERT
    } as usize] = [];
    #[repr(C, align(8))]
    pub struct Vault {
        /// Program version when initialized: [major, minor, patch]
        pub version: [u8; 3],
        /// Account which is allowed to call restricted instructions
        /// Also the authority of the fee receiver account
        pub owner: Pubkey,
        /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
        pub vault_authority: Pubkey,
        pub authority_seed: Pubkey,
        pub authority_bump: [u8; 1],
        pub solend_reserve: Pubkey,
        pub port_reserve: Pubkey,
        pub jet_reserve: Pubkey,
        /// Account where reserve tokens are stored
        pub vault_reserve_token: Pubkey,
        /// Account where solend LP tokens are stored
        pub vault_solend_lp_token: Pubkey,
        /// Account where port LP tokens are stored
        pub vault_port_lp_token: Pubkey,
        /// Account where jet LP tokens are stored
        pub vault_jet_lp_token: Pubkey,
        /// Mint address of vault LP tokens
        pub lp_token_mint: Pubkey,
        /// Mint address of the tokens that are stored in vault
        pub reserve_token_mint: Pubkey,
        pub fee_receiver: Pubkey,
        pub referral_fee_receiver: Pubkey,
        halt_flags: u16,
        yield_source_flags: u16,
        /// Total value of vault denominated in the reserve token
        pub value: SlotTrackedValue,
        /// Prospective allocations set by rebalance, executed by reconciles
        pub target_allocations: Allocations,
        pub config: VaultConfig,
        pub actual_allocations: Allocations,
        /// Reserved space for future upgrades
        _reserved: [u32; 28],
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Vault {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Vault {
                    version: ref __self_0_0,
                    owner: ref __self_0_1,
                    vault_authority: ref __self_0_2,
                    authority_seed: ref __self_0_3,
                    authority_bump: ref __self_0_4,
                    solend_reserve: ref __self_0_5,
                    port_reserve: ref __self_0_6,
                    jet_reserve: ref __self_0_7,
                    vault_reserve_token: ref __self_0_8,
                    vault_solend_lp_token: ref __self_0_9,
                    vault_port_lp_token: ref __self_0_10,
                    vault_jet_lp_token: ref __self_0_11,
                    lp_token_mint: ref __self_0_12,
                    reserve_token_mint: ref __self_0_13,
                    fee_receiver: ref __self_0_14,
                    referral_fee_receiver: ref __self_0_15,
                    halt_flags: ref __self_0_16,
                    yield_source_flags: ref __self_0_17,
                    value: ref __self_0_18,
                    target_allocations: ref __self_0_19,
                    config: ref __self_0_20,
                    actual_allocations: ref __self_0_21,
                    _reserved: ref __self_0_22,
                } => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_struct(f, "Vault");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "version",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "owner",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "vault_authority",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "authority_seed",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "authority_bump",
                        &&(*__self_0_4),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "solend_reserve",
                        &&(*__self_0_5),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "port_reserve",
                        &&(*__self_0_6),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "jet_reserve",
                        &&(*__self_0_7),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "vault_reserve_token",
                        &&(*__self_0_8),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "vault_solend_lp_token",
                        &&(*__self_0_9),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "vault_port_lp_token",
                        &&(*__self_0_10),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "vault_jet_lp_token",
                        &&(*__self_0_11),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "lp_token_mint",
                        &&(*__self_0_12),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "reserve_token_mint",
                        &&(*__self_0_13),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "fee_receiver",
                        &&(*__self_0_14),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "referral_fee_receiver",
                        &&(*__self_0_15),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "halt_flags",
                        &&(*__self_0_16),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "yield_source_flags",
                        &&(*__self_0_17),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "value",
                        &&(*__self_0_18),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "target_allocations",
                        &&(*__self_0_19),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "config",
                        &&(*__self_0_20),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "actual_allocations",
                        &&(*__self_0_21),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "_reserved",
                        &&(*__self_0_22),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl borsh::ser::BorshSerialize for Vault
    where
        [u8; 3]: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        [u8; 1]: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        u16: borsh::ser::BorshSerialize,
        u16: borsh::ser::BorshSerialize,
        SlotTrackedValue: borsh::ser::BorshSerialize,
        Allocations: borsh::ser::BorshSerialize,
        VaultConfig: borsh::ser::BorshSerialize,
        Allocations: borsh::ser::BorshSerialize,
        [u32; 28]: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.version, writer)?;
            borsh::BorshSerialize::serialize(&self.owner, writer)?;
            borsh::BorshSerialize::serialize(&self.vault_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.authority_seed, writer)?;
            borsh::BorshSerialize::serialize(&self.authority_bump, writer)?;
            borsh::BorshSerialize::serialize(&self.solend_reserve, writer)?;
            borsh::BorshSerialize::serialize(&self.port_reserve, writer)?;
            borsh::BorshSerialize::serialize(&self.jet_reserve, writer)?;
            borsh::BorshSerialize::serialize(&self.vault_reserve_token, writer)?;
            borsh::BorshSerialize::serialize(&self.vault_solend_lp_token, writer)?;
            borsh::BorshSerialize::serialize(&self.vault_port_lp_token, writer)?;
            borsh::BorshSerialize::serialize(&self.vault_jet_lp_token, writer)?;
            borsh::BorshSerialize::serialize(&self.lp_token_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.reserve_token_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.fee_receiver, writer)?;
            borsh::BorshSerialize::serialize(&self.referral_fee_receiver, writer)?;
            borsh::BorshSerialize::serialize(&self.halt_flags, writer)?;
            borsh::BorshSerialize::serialize(&self.yield_source_flags, writer)?;
            borsh::BorshSerialize::serialize(&self.value, writer)?;
            borsh::BorshSerialize::serialize(&self.target_allocations, writer)?;
            borsh::BorshSerialize::serialize(&self.config, writer)?;
            borsh::BorshSerialize::serialize(&self.actual_allocations, writer)?;
            borsh::BorshSerialize::serialize(&self._reserved, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for Vault
    where
        [u8; 3]: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        [u8; 1]: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        u16: borsh::BorshDeserialize,
        u16: borsh::BorshDeserialize,
        SlotTrackedValue: borsh::BorshDeserialize,
        Allocations: borsh::BorshDeserialize,
        VaultConfig: borsh::BorshDeserialize,
        Allocations: borsh::BorshDeserialize,
        [u32; 28]: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                version: borsh::BorshDeserialize::deserialize(buf)?,
                owner: borsh::BorshDeserialize::deserialize(buf)?,
                vault_authority: borsh::BorshDeserialize::deserialize(buf)?,
                authority_seed: borsh::BorshDeserialize::deserialize(buf)?,
                authority_bump: borsh::BorshDeserialize::deserialize(buf)?,
                solend_reserve: borsh::BorshDeserialize::deserialize(buf)?,
                port_reserve: borsh::BorshDeserialize::deserialize(buf)?,
                jet_reserve: borsh::BorshDeserialize::deserialize(buf)?,
                vault_reserve_token: borsh::BorshDeserialize::deserialize(buf)?,
                vault_solend_lp_token: borsh::BorshDeserialize::deserialize(buf)?,
                vault_port_lp_token: borsh::BorshDeserialize::deserialize(buf)?,
                vault_jet_lp_token: borsh::BorshDeserialize::deserialize(buf)?,
                lp_token_mint: borsh::BorshDeserialize::deserialize(buf)?,
                reserve_token_mint: borsh::BorshDeserialize::deserialize(buf)?,
                fee_receiver: borsh::BorshDeserialize::deserialize(buf)?,
                referral_fee_receiver: borsh::BorshDeserialize::deserialize(buf)?,
                halt_flags: borsh::BorshDeserialize::deserialize(buf)?,
                yield_source_flags: borsh::BorshDeserialize::deserialize(buf)?,
                value: borsh::BorshDeserialize::deserialize(buf)?,
                target_allocations: borsh::BorshDeserialize::deserialize(buf)?,
                config: borsh::BorshDeserialize::deserialize(buf)?,
                actual_allocations: borsh::BorshDeserialize::deserialize(buf)?,
                _reserved: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Vault {
        #[inline]
        fn clone(&self) -> Vault {
            match *self {
                Vault {
                    version: ref __self_0_0,
                    owner: ref __self_0_1,
                    vault_authority: ref __self_0_2,
                    authority_seed: ref __self_0_3,
                    authority_bump: ref __self_0_4,
                    solend_reserve: ref __self_0_5,
                    port_reserve: ref __self_0_6,
                    jet_reserve: ref __self_0_7,
                    vault_reserve_token: ref __self_0_8,
                    vault_solend_lp_token: ref __self_0_9,
                    vault_port_lp_token: ref __self_0_10,
                    vault_jet_lp_token: ref __self_0_11,
                    lp_token_mint: ref __self_0_12,
                    reserve_token_mint: ref __self_0_13,
                    fee_receiver: ref __self_0_14,
                    referral_fee_receiver: ref __self_0_15,
                    halt_flags: ref __self_0_16,
                    yield_source_flags: ref __self_0_17,
                    value: ref __self_0_18,
                    target_allocations: ref __self_0_19,
                    config: ref __self_0_20,
                    actual_allocations: ref __self_0_21,
                    _reserved: ref __self_0_22,
                } => Vault {
                    version: ::core::clone::Clone::clone(&(*__self_0_0)),
                    owner: ::core::clone::Clone::clone(&(*__self_0_1)),
                    vault_authority: ::core::clone::Clone::clone(&(*__self_0_2)),
                    authority_seed: ::core::clone::Clone::clone(&(*__self_0_3)),
                    authority_bump: ::core::clone::Clone::clone(&(*__self_0_4)),
                    solend_reserve: ::core::clone::Clone::clone(&(*__self_0_5)),
                    port_reserve: ::core::clone::Clone::clone(&(*__self_0_6)),
                    jet_reserve: ::core::clone::Clone::clone(&(*__self_0_7)),
                    vault_reserve_token: ::core::clone::Clone::clone(&(*__self_0_8)),
                    vault_solend_lp_token: ::core::clone::Clone::clone(&(*__self_0_9)),
                    vault_port_lp_token: ::core::clone::Clone::clone(&(*__self_0_10)),
                    vault_jet_lp_token: ::core::clone::Clone::clone(&(*__self_0_11)),
                    lp_token_mint: ::core::clone::Clone::clone(&(*__self_0_12)),
                    reserve_token_mint: ::core::clone::Clone::clone(&(*__self_0_13)),
                    fee_receiver: ::core::clone::Clone::clone(&(*__self_0_14)),
                    referral_fee_receiver: ::core::clone::Clone::clone(&(*__self_0_15)),
                    halt_flags: ::core::clone::Clone::clone(&(*__self_0_16)),
                    yield_source_flags: ::core::clone::Clone::clone(&(*__self_0_17)),
                    value: ::core::clone::Clone::clone(&(*__self_0_18)),
                    target_allocations: ::core::clone::Clone::clone(&(*__self_0_19)),
                    config: ::core::clone::Clone::clone(&(*__self_0_20)),
                    actual_allocations: ::core::clone::Clone::clone(&(*__self_0_21)),
                    _reserved: ::core::clone::Clone::clone(&(*__self_0_22)),
                },
            }
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountSerialize for Vault {
        fn try_serialize<W: std::io::Write>(
            &self,
            writer: &mut W,
        ) -> std::result::Result<(), ProgramError> {
            writer
                .write_all(&[211, 8, 232, 43, 2, 152, 117, 119])
                .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotSerialize)?;
            AnchorSerialize::serialize(self, writer)
                .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotSerialize)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountDeserialize for Vault {
        fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
            if buf.len() < [211, 8, 232, 43, 2, 152, 117, 119].len() {
                return Err(anchor_lang::__private::ErrorCode::AccountDiscriminatorNotFound.into());
            }
            let given_disc = &buf[..8];
            if &[211, 8, 232, 43, 2, 152, 117, 119] != given_disc {
                return Err(anchor_lang::__private::ErrorCode::AccountDiscriminatorMismatch.into());
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
            let mut data: &[u8] = &buf[8..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotDeserialize.into())
        }
    }
    #[automatically_derived]
    impl anchor_lang::Discriminator for Vault {
        fn discriminator() -> [u8; 8] {
            [211, 8, 232, 43, 2, 152, 117, 119]
        }
    }
    #[automatically_derived]
    impl anchor_lang::Owner for Vault {
        fn owner() -> Pubkey {
            crate::ID
        }
    }
    impl Vault {
        pub fn get_halt_flags(&self) -> VaultFlags {
            VaultFlags::from_bits(self.halt_flags).unwrap_or_else(|| {
                ::std::rt::panic_fmt(::core::fmt::Arguments::new_v1(
                    &["", " does not resolve to valid VaultFlags"],
                    &match (&self.halt_flags,) {
                        _args => [::core::fmt::ArgumentV1::new(
                            _args.0,
                            ::core::fmt::Debug::fmt,
                        )],
                    },
                ))
            })
        }
        pub fn set_halt_flags(&mut self, bits: u16) -> ProgramResult {
            VaultFlags::from_bits(bits)
                .ok_or_else::<ProgramError, _>(|| ErrorCode::InvalidVaultFlags.into())?;
            self.halt_flags = bits;
            Ok(())
        }
        pub fn get_yield_source_flags(&self) -> YieldSourceFlags {
            YieldSourceFlags::from_bits(self.yield_source_flags).unwrap_or_else(|| {
                {
                    ::std::rt::panic_fmt(::core::fmt::Arguments::new_v1(
                        &["", " does not resolve to valid YieldSourceFlags"],
                        &match (&self.yield_source_flags,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Debug::fmt,
                            )],
                        },
                    ))
                }
            })
        }
        pub fn set_yield_source_flag(
            &mut self,
            flag: YieldSourceFlags,
            initialized: bool,
        ) -> ProgramResult {
            let mut new_flags = self.get_yield_source_flags();
            new_flags.set(flag, initialized);
            self.yield_source_flags = new_flags.bits();
            Ok(())
        }
        pub fn adjust_allocation_cap(&mut self) -> ProgramResult {
            let cnt: u8 =
                u8::try_from((0..16).fold(0, |sum, i| sum + ((self.yield_source_flags >> i) & 1)))
                    .map_err::<ProgramError, _>(|_| ErrorCode::MathError.into())?;
            let new_allocation_cap = 100_u8
                .checked_div(cnt)
                .ok_or_else::<ProgramError, _>(|| ErrorCode::MathError.into())?
                .checked_add(1)
                .ok_or_else::<ProgramError, _>(|| ErrorCode::MathError.into())?
                .clamp(0, 100);
            self.config.allocation_cap_pct = self
                .config
                .allocation_cap_pct
                .clamp(new_allocation_cap, 100);
            #[cfg(feature = "debug")]
            {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["num of active pools: "],
                        &match (&cnt,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &[" new allocation cap: "],
                        &match (&self.config.allocation_cap_pct,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
            }
            Ok(())
        }
        pub fn get_yield_source_availability(&self, provider: Provider) -> bool {
            let flags = self.get_yield_source_flags();
            match provider {
                Provider::Solend => flags.contains(YieldSourceFlags::SOLEND),
                Provider::Port => flags.contains(YieldSourceFlags::PORT),
                Provider::Jet => flags.contains(YieldSourceFlags::JET),
            }
        }
        pub fn calculate_fees(&self, new_vault_value: u64, slot: u64) -> Result<u64, ProgramError> {
            let vault_value_diff = new_vault_value.saturating_sub(self.value.value);
            let slots_elapsed = self.value.last_update.slots_elapsed(slot)?;
            let carry = calc_carry_fees(vault_value_diff, self.config.fee_carry_bps as u64)?;
            let mgmt = calc_mgmt_fees(
                new_vault_value,
                self.config.fee_mgmt_bps as u64,
                slots_elapsed,
            )?;
            #[cfg(feature = "debug")]
            {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Slots elapsed: "],
                        &match (&slots_elapsed,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["New vault value: "],
                        &match (&new_vault_value,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Old vault value: "],
                        &match (&self.value.value,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Carry fee: "],
                        &match (&carry,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Mgmt fee: "],
                        &match (&mgmt,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
            }
            carry
                .checked_add(mgmt)
                .ok_or_else(|| ErrorCode::OverflowError.into())
        }
        pub fn authority_seeds(&self) -> [&[u8]; 3] {
            [
                self.authority_seed.as_ref(),
                b"authority".as_ref(),
                &self.authority_bump,
            ]
        }
    }
    #[allow(unknown_lints, eq_op)]
    const _: [(); 0 - !{
        const ASSERT: bool = 0 == std::mem::size_of::<VaultConfig>() % 8;
        ASSERT
    } as usize] = [];
    #[allow(unknown_lints, eq_op)]
    const _: [(); 0 - !{
        const ASSERT: bool = 32usize == std::mem::size_of::<VaultConfig>();
        ASSERT
    } as usize] = [];
    #[repr(C, align(8))]
    pub struct VaultConfig {
        /// Max num of reserve tokens. If total_value grows higher than this, will stop accepting deposits.
        pub deposit_cap: u64,
        /// Basis points of the accrued interest that gets sent to the fee_receiver
        pub fee_carry_bps: u32,
        /// Basis points of the AUM that gets sent to the fee_receiver
        pub fee_mgmt_bps: u32,
        /// Referral fee share for fee splitting
        pub referral_fee_pct: u8,
        /// Max percentage to allocate to each pool
        pub allocation_cap_pct: u8,
        /// Whether to run rebalance as a proof check or a calculation
        pub rebalance_mode: RebalanceMode,
        /// Strategy type that is executed during rebalance
        pub strategy_type: StrategyType,
        _padding: [u32; 3],
    }
    impl borsh::de::BorshDeserialize for VaultConfig
    where
        u64: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        RebalanceMode: borsh::BorshDeserialize,
        StrategyType: borsh::BorshDeserialize,
        [u32; 3]: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                deposit_cap: borsh::BorshDeserialize::deserialize(buf)?,
                fee_carry_bps: borsh::BorshDeserialize::deserialize(buf)?,
                fee_mgmt_bps: borsh::BorshDeserialize::deserialize(buf)?,
                referral_fee_pct: borsh::BorshDeserialize::deserialize(buf)?,
                allocation_cap_pct: borsh::BorshDeserialize::deserialize(buf)?,
                rebalance_mode: borsh::BorshDeserialize::deserialize(buf)?,
                strategy_type: borsh::BorshDeserialize::deserialize(buf)?,
                _padding: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl borsh::ser::BorshSerialize for VaultConfig
    where
        u64: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        RebalanceMode: borsh::ser::BorshSerialize,
        StrategyType: borsh::ser::BorshSerialize,
        [u32; 3]: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.deposit_cap, writer)?;
            borsh::BorshSerialize::serialize(&self.fee_carry_bps, writer)?;
            borsh::BorshSerialize::serialize(&self.fee_mgmt_bps, writer)?;
            borsh::BorshSerialize::serialize(&self.referral_fee_pct, writer)?;
            borsh::BorshSerialize::serialize(&self.allocation_cap_pct, writer)?;
            borsh::BorshSerialize::serialize(&self.rebalance_mode, writer)?;
            borsh::BorshSerialize::serialize(&self.strategy_type, writer)?;
            borsh::BorshSerialize::serialize(&self._padding, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for VaultConfig {
        #[inline]
        fn clone(&self) -> VaultConfig {
            {
                let _: ::core::clone::AssertParamIsClone<u64>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                let _: ::core::clone::AssertParamIsClone<u8>;
                let _: ::core::clone::AssertParamIsClone<u8>;
                let _: ::core::clone::AssertParamIsClone<RebalanceMode>;
                let _: ::core::clone::AssertParamIsClone<StrategyType>;
                let _: ::core::clone::AssertParamIsClone<[u32; 3]>;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for VaultConfig {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for VaultConfig {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                VaultConfig {
                    deposit_cap: ref __self_0_0,
                    fee_carry_bps: ref __self_0_1,
                    fee_mgmt_bps: ref __self_0_2,
                    referral_fee_pct: ref __self_0_3,
                    allocation_cap_pct: ref __self_0_4,
                    rebalance_mode: ref __self_0_5,
                    strategy_type: ref __self_0_6,
                    _padding: ref __self_0_7,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "VaultConfig");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "deposit_cap",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "fee_carry_bps",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "fee_mgmt_bps",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "referral_fee_pct",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "allocation_cap_pct",
                        &&(*__self_0_4),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "rebalance_mode",
                        &&(*__self_0_5),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "strategy_type",
                        &&(*__self_0_6),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "_padding",
                        &&(*__self_0_7),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl VaultConfig {
        pub fn new(config: VaultConfigArg) -> Result<Self, ProgramError> {
            if config.fee_carry_bps > 10000 {
                return Err(ErrorCode::InvalidFeeConfig.into());
            }
            if config.fee_mgmt_bps > 10000 {
                return Err(ErrorCode::InvalidFeeConfig.into());
            }
            if config.referral_fee_pct > 50 {
                return Err(ErrorCode::InvalidReferralFeeConfig.into());
            }
            if !(34..=100).contains(&config.allocation_cap_pct) {
                return Err(ErrorCode::InvalidAllocationCap.into());
            }
            Ok(Self {
                deposit_cap: config.deposit_cap,
                fee_carry_bps: config.fee_carry_bps,
                fee_mgmt_bps: config.fee_mgmt_bps,
                referral_fee_pct: config.referral_fee_pct,
                allocation_cap_pct: config.allocation_cap_pct,
                rebalance_mode: config.rebalance_mode,
                strategy_type: config.strategy_type,
                _padding: [0; 3],
            })
        }
    }
    #[repr(u8)]
    pub enum RebalanceMode {
        Calculator,
        ProofChecker,
    }
    impl borsh::de::BorshDeserialize for RebalanceMode {
        fn deserialize(buf: &mut &[u8]) -> core::result::Result<Self, borsh::maybestd::io::Error> {
            let variant_idx: u8 = borsh::BorshDeserialize::deserialize(buf)?;
            let return_value = match variant_idx {
                0u8 => RebalanceMode::Calculator,
                1u8 => RebalanceMode::ProofChecker,
                _ => {
                    let msg = {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Unexpected variant index: "],
                            &match (&variant_idx,) {
                                _args => [::core::fmt::ArgumentV1::new(
                                    _args.0,
                                    ::core::fmt::Debug::fmt,
                                )],
                            },
                        ));
                        res
                    };
                    return Err(borsh::maybestd::io::Error::new(
                        borsh::maybestd::io::ErrorKind::InvalidInput,
                        msg,
                    ));
                }
            };
            Ok(return_value)
        }
    }
    impl borsh::ser::BorshSerialize for RebalanceMode {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> core::result::Result<(), borsh::maybestd::io::Error> {
            match self {
                RebalanceMode::Calculator => {
                    let variant_idx: u8 = 0u8;
                    writer.write_all(&variant_idx.to_le_bytes())?;
                }
                RebalanceMode::ProofChecker => {
                    let variant_idx: u8 = 1u8;
                    writer.write_all(&variant_idx.to_le_bytes())?;
                }
            }
            Ok(())
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for RebalanceMode {
        #[inline]
        fn clone(&self) -> RebalanceMode {
            {
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for RebalanceMode {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for RebalanceMode {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&RebalanceMode::Calculator,) => ::core::fmt::Formatter::write_str(f, "Calculator"),
                (&RebalanceMode::ProofChecker,) => {
                    ::core::fmt::Formatter::write_str(f, "ProofChecker")
                }
            }
        }
    }
    #[repr(u8)]
    pub enum StrategyType {
        MaxYield,
        EqualAllocation,
    }
    impl borsh::de::BorshDeserialize for StrategyType {
        fn deserialize(buf: &mut &[u8]) -> core::result::Result<Self, borsh::maybestd::io::Error> {
            let variant_idx: u8 = borsh::BorshDeserialize::deserialize(buf)?;
            let return_value = match variant_idx {
                0u8 => StrategyType::MaxYield,
                1u8 => StrategyType::EqualAllocation,
                _ => {
                    let msg = {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Unexpected variant index: "],
                            &match (&variant_idx,) {
                                _args => [::core::fmt::ArgumentV1::new(
                                    _args.0,
                                    ::core::fmt::Debug::fmt,
                                )],
                            },
                        ));
                        res
                    };
                    return Err(borsh::maybestd::io::Error::new(
                        borsh::maybestd::io::ErrorKind::InvalidInput,
                        msg,
                    ));
                }
            };
            Ok(return_value)
        }
    }
    impl borsh::ser::BorshSerialize for StrategyType {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> core::result::Result<(), borsh::maybestd::io::Error> {
            match self {
                StrategyType::MaxYield => {
                    let variant_idx: u8 = 0u8;
                    writer.write_all(&variant_idx.to_le_bytes())?;
                }
                StrategyType::EqualAllocation => {
                    let variant_idx: u8 = 1u8;
                    writer.write_all(&variant_idx.to_le_bytes())?;
                }
            }
            Ok(())
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for StrategyType {
        #[inline]
        fn clone(&self) -> StrategyType {
            {
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for StrategyType {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for StrategyType {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&StrategyType::MaxYield,) => ::core::fmt::Formatter::write_str(f, "MaxYield"),
                (&StrategyType::EqualAllocation,) => {
                    ::core::fmt::Formatter::write_str(f, "EqualAllocation")
                }
            }
        }
    }
    pub struct VaultFlags {
        bits: u16,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for VaultFlags {}
    impl ::core::marker::StructuralPartialEq for VaultFlags {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for VaultFlags {
        #[inline]
        fn eq(&self, other: &VaultFlags) -> bool {
            match *other {
                VaultFlags {
                    bits: ref __self_1_0,
                } => match *self {
                    VaultFlags {
                        bits: ref __self_0_0,
                    } => (*__self_0_0) == (*__self_1_0),
                },
            }
        }
        #[inline]
        fn ne(&self, other: &VaultFlags) -> bool {
            match *other {
                VaultFlags {
                    bits: ref __self_1_0,
                } => match *self {
                    VaultFlags {
                        bits: ref __self_0_0,
                    } => (*__self_0_0) != (*__self_1_0),
                },
            }
        }
    }
    impl ::core::marker::StructuralEq for VaultFlags {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for VaultFlags {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {
            {
                let _: ::core::cmp::AssertParamIsEq<u16>;
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for VaultFlags {
        #[inline]
        fn clone(&self) -> VaultFlags {
            {
                let _: ::core::clone::AssertParamIsClone<u16>;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialOrd for VaultFlags {
        #[inline]
        fn partial_cmp(&self, other: &VaultFlags) -> ::core::option::Option<::core::cmp::Ordering> {
            match *other {
                VaultFlags {
                    bits: ref __self_1_0,
                } => match *self {
                    VaultFlags {
                        bits: ref __self_0_0,
                    } => match ::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0), &(*__self_1_0))
                    {
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                        }
                        cmp => cmp,
                    },
                },
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Ord for VaultFlags {
        #[inline]
        fn cmp(&self, other: &VaultFlags) -> ::core::cmp::Ordering {
            match *other {
                VaultFlags {
                    bits: ref __self_1_0,
                } => match *self {
                    VaultFlags {
                        bits: ref __self_0_0,
                    } => match ::core::cmp::Ord::cmp(&(*__self_0_0), &(*__self_1_0)) {
                        ::core::cmp::Ordering::Equal => ::core::cmp::Ordering::Equal,
                        cmp => cmp,
                    },
                },
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::hash::Hash for VaultFlags {
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            match *self {
                VaultFlags {
                    bits: ref __self_0_0,
                } => ::core::hash::Hash::hash(&(*__self_0_0), state),
            }
        }
    }
    impl ::bitflags::_core::fmt::Debug for VaultFlags {
        fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter) -> ::bitflags::_core::fmt::Result {
            #[allow(non_snake_case)]
            trait __BitFlags {
                #[inline]
                fn HALT_RECONCILES(&self) -> bool {
                    false
                }
                #[inline]
                fn HALT_REFRESHES(&self) -> bool {
                    false
                }
                #[inline]
                fn HALT_DEPOSITS_WITHDRAWS(&self) -> bool {
                    false
                }
                #[inline]
                fn HALT_ALL(&self) -> bool {
                    false
                }
            }
            #[allow(non_snake_case)]
            impl __BitFlags for VaultFlags {
                #[allow(deprecated)]
                #[inline]
                fn HALT_RECONCILES(&self) -> bool {
                    if Self::HALT_RECONCILES.bits == 0 && self.bits != 0 {
                        false
                    } else {
                        self.bits & Self::HALT_RECONCILES.bits == Self::HALT_RECONCILES.bits
                    }
                }
                #[allow(deprecated)]
                #[inline]
                fn HALT_REFRESHES(&self) -> bool {
                    if Self::HALT_REFRESHES.bits == 0 && self.bits != 0 {
                        false
                    } else {
                        self.bits & Self::HALT_REFRESHES.bits == Self::HALT_REFRESHES.bits
                    }
                }
                #[allow(deprecated)]
                #[inline]
                fn HALT_DEPOSITS_WITHDRAWS(&self) -> bool {
                    if Self::HALT_DEPOSITS_WITHDRAWS.bits == 0 && self.bits != 0 {
                        false
                    } else {
                        self.bits & Self::HALT_DEPOSITS_WITHDRAWS.bits
                            == Self::HALT_DEPOSITS_WITHDRAWS.bits
                    }
                }
                #[allow(deprecated)]
                #[inline]
                fn HALT_ALL(&self) -> bool {
                    if Self::HALT_ALL.bits == 0 && self.bits != 0 {
                        false
                    } else {
                        self.bits & Self::HALT_ALL.bits == Self::HALT_ALL.bits
                    }
                }
            }
            let mut first = true;
            if <Self as __BitFlags>::HALT_RECONCILES(self) {
                if !first {
                    f.write_str(" | ")?;
                }
                first = false;
                f.write_str("HALT_RECONCILES")?;
            }
            if <Self as __BitFlags>::HALT_REFRESHES(self) {
                if !first {
                    f.write_str(" | ")?;
                }
                first = false;
                f.write_str("HALT_REFRESHES")?;
            }
            if <Self as __BitFlags>::HALT_DEPOSITS_WITHDRAWS(self) {
                if !first {
                    f.write_str(" | ")?;
                }
                first = false;
                f.write_str("HALT_DEPOSITS_WITHDRAWS")?;
            }
            if <Self as __BitFlags>::HALT_ALL(self) {
                if !first {
                    f.write_str(" | ")?;
                }
                first = false;
                f.write_str("HALT_ALL")?;
            }
            let extra_bits = self.bits & !Self::all().bits();
            if extra_bits != 0 {
                if !first {
                    f.write_str(" | ")?;
                }
                first = false;
                f.write_str("0x")?;
                ::bitflags::_core::fmt::LowerHex::fmt(&extra_bits, f)?;
            }
            if first {
                f.write_str("(empty)")?;
            }
            Ok(())
        }
    }
    impl ::bitflags::_core::fmt::Binary for VaultFlags {
        fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter) -> ::bitflags::_core::fmt::Result {
            ::bitflags::_core::fmt::Binary::fmt(&self.bits, f)
        }
    }
    impl ::bitflags::_core::fmt::Octal for VaultFlags {
        fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter) -> ::bitflags::_core::fmt::Result {
            ::bitflags::_core::fmt::Octal::fmt(&self.bits, f)
        }
    }
    impl ::bitflags::_core::fmt::LowerHex for VaultFlags {
        fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter) -> ::bitflags::_core::fmt::Result {
            ::bitflags::_core::fmt::LowerHex::fmt(&self.bits, f)
        }
    }
    impl ::bitflags::_core::fmt::UpperHex for VaultFlags {
        fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter) -> ::bitflags::_core::fmt::Result {
            ::bitflags::_core::fmt::UpperHex::fmt(&self.bits, f)
        }
    }
    #[allow(dead_code)]
    impl VaultFlags {
        /// Disable reconciles
        pub const HALT_RECONCILES: Self = Self { bits: 1 << 0 };
        /// Disable refreshes
        pub const HALT_REFRESHES: Self = Self { bits: 1 << 1 };
        /// Disable deposits + withdrawals
        pub const HALT_DEPOSITS_WITHDRAWS: Self = Self { bits: 1 << 2 };
        /// Disable all operations
        pub const HALT_ALL: Self = Self {
            bits: Self::HALT_RECONCILES.bits
                | Self::HALT_REFRESHES.bits
                | Self::HALT_DEPOSITS_WITHDRAWS.bits,
        };
        /// Returns an empty set of flags.
        #[inline]
        pub const fn empty() -> Self {
            Self { bits: 0 }
        }
        /// Returns the set containing all flags.
        #[inline]
        pub const fn all() -> Self {
            #[allow(non_snake_case)]
            trait __BitFlags {
                const HALT_RECONCILES: u16 = 0;
                const HALT_REFRESHES: u16 = 0;
                const HALT_DEPOSITS_WITHDRAWS: u16 = 0;
                const HALT_ALL: u16 = 0;
            }
            #[allow(non_snake_case)]
            impl __BitFlags for VaultFlags {
                #[allow(deprecated)]
                const HALT_RECONCILES: u16 = Self::HALT_RECONCILES.bits;
                #[allow(deprecated)]
                const HALT_REFRESHES: u16 = Self::HALT_REFRESHES.bits;
                #[allow(deprecated)]
                const HALT_DEPOSITS_WITHDRAWS: u16 = Self::HALT_DEPOSITS_WITHDRAWS.bits;
                #[allow(deprecated)]
                const HALT_ALL: u16 = Self::HALT_ALL.bits;
            }
            Self {
                bits: <Self as __BitFlags>::HALT_RECONCILES
                    | <Self as __BitFlags>::HALT_REFRESHES
                    | <Self as __BitFlags>::HALT_DEPOSITS_WITHDRAWS
                    | <Self as __BitFlags>::HALT_ALL,
            }
        }
        /// Returns the raw value of the flags currently stored.
        #[inline]
        pub const fn bits(&self) -> u16 {
            self.bits
        }
        /// Convert from underlying bit representation, unless that
        /// representation contains bits that do not correspond to a flag.
        #[inline]
        pub const fn from_bits(bits: u16) -> ::bitflags::_core::option::Option<Self> {
            if (bits & !Self::all().bits()) == 0 {
                ::bitflags::_core::option::Option::Some(Self { bits })
            } else {
                ::bitflags::_core::option::Option::None
            }
        }
        /// Convert from underlying bit representation, dropping any bits
        /// that do not correspond to flags.
        #[inline]
        pub const fn from_bits_truncate(bits: u16) -> Self {
            Self {
                bits: bits & Self::all().bits,
            }
        }
        /// Convert from underlying bit representation, preserving all
        /// bits (even those not corresponding to a defined flag).
        ///
        /// # Safety
        ///
        /// The caller of the `bitflags!` macro can chose to allow or
        /// disallow extra bits for their bitflags type.
        ///
        /// The caller of `from_bits_unchecked()` has to ensure that
        /// all bits correspond to a defined flag or that extra bits
        /// are valid for this bitflags type.
        #[inline]
        pub const unsafe fn from_bits_unchecked(bits: u16) -> Self {
            Self { bits }
        }
        /// Returns `true` if no flags are currently stored.
        #[inline]
        pub const fn is_empty(&self) -> bool {
            self.bits() == Self::empty().bits()
        }
        /// Returns `true` if all flags are currently set.
        #[inline]
        pub const fn is_all(&self) -> bool {
            Self::all().bits | self.bits == self.bits
        }
        /// Returns `true` if there are flags common to both `self` and `other`.
        #[inline]
        pub const fn intersects(&self, other: Self) -> bool {
            !(Self {
                bits: self.bits & other.bits,
            })
            .is_empty()
        }
        /// Returns `true` if all of the flags in `other` are contained within `self`.
        #[inline]
        pub const fn contains(&self, other: Self) -> bool {
            (self.bits & other.bits) == other.bits
        }
        /// Inserts the specified flags in-place.
        #[inline]
        pub fn insert(&mut self, other: Self) {
            self.bits |= other.bits;
        }
        /// Removes the specified flags in-place.
        #[inline]
        pub fn remove(&mut self, other: Self) {
            self.bits &= !other.bits;
        }
        /// Toggles the specified flags in-place.
        #[inline]
        pub fn toggle(&mut self, other: Self) {
            self.bits ^= other.bits;
        }
        /// Inserts or removes the specified flags depending on the passed value.
        #[inline]
        pub fn set(&mut self, other: Self, value: bool) {
            if value {
                self.insert(other);
            } else {
                self.remove(other);
            }
        }
        /// Returns the intersection between the flags in `self` and
        /// `other`.
        ///
        /// Specifically, the returned set contains only the flags which are
        /// present in *both* `self` *and* `other`.
        ///
        /// This is equivalent to using the `&` operator (e.g.
        /// [`ops::BitAnd`]), as in `flags & other`.
        ///
        /// [`ops::BitAnd`]: https://doc.rust-lang.org/std/ops/trait.BitAnd.html
        #[inline]
        #[must_use]
        pub const fn intersection(self, other: Self) -> Self {
            Self {
                bits: self.bits & other.bits,
            }
        }
        /// Returns the union of between the flags in `self` and `other`.
        ///
        /// Specifically, the returned set contains all flags which are
        /// present in *either* `self` *or* `other`, including any which are
        /// present in both (see [`Self::symmetric_difference`] if that
        /// is undesirable).
        ///
        /// This is equivalent to using the `|` operator (e.g.
        /// [`ops::BitOr`]), as in `flags | other`.
        ///
        /// [`ops::BitOr`]: https://doc.rust-lang.org/std/ops/trait.BitOr.html
        #[inline]
        #[must_use]
        pub const fn union(self, other: Self) -> Self {
            Self {
                bits: self.bits | other.bits,
            }
        }
        /// Returns the difference between the flags in `self` and `other`.
        ///
        /// Specifically, the returned set contains all flags present in
        /// `self`, except for the ones present in `other`.
        ///
        /// It is also conceptually equivalent to the "bit-clear" operation:
        /// `flags & !other` (and this syntax is also supported).
        ///
        /// This is equivalent to using the `-` operator (e.g.
        /// [`ops::Sub`]), as in `flags - other`.
        ///
        /// [`ops::Sub`]: https://doc.rust-lang.org/std/ops/trait.Sub.html
        #[inline]
        #[must_use]
        pub const fn difference(self, other: Self) -> Self {
            Self {
                bits: self.bits & !other.bits,
            }
        }
        /// Returns the [symmetric difference][sym-diff] between the flags
        /// in `self` and `other`.
        ///
        /// Specifically, the returned set contains the flags present which
        /// are present in `self` or `other`, but that are not present in
        /// both. Equivalently, it contains the flags present in *exactly
        /// one* of the sets `self` and `other`.
        ///
        /// This is equivalent to using the `^` operator (e.g.
        /// [`ops::BitXor`]), as in `flags ^ other`.
        ///
        /// [sym-diff]: https://en.wikipedia.org/wiki/Symmetric_difference
        /// [`ops::BitXor`]: https://doc.rust-lang.org/std/ops/trait.BitXor.html
        #[inline]
        #[must_use]
        pub const fn symmetric_difference(self, other: Self) -> Self {
            Self {
                bits: self.bits ^ other.bits,
            }
        }
        /// Returns the complement of this set of flags.
        ///
        /// Specifically, the returned set contains all the flags which are
        /// not set in `self`, but which are allowed for this type.
        ///
        /// Alternatively, it can be thought of as the set difference
        /// between [`Self::all()`] and `self` (e.g. `Self::all() - self`)
        ///
        /// This is equivalent to using the `!` operator (e.g.
        /// [`ops::Not`]), as in `!flags`.
        ///
        /// [`Self::all()`]: Self::all
        /// [`ops::Not`]: https://doc.rust-lang.org/std/ops/trait.Not.html
        #[inline]
        #[must_use]
        pub const fn complement(self) -> Self {
            Self::from_bits_truncate(!self.bits)
        }
    }
    impl ::bitflags::_core::ops::BitOr for VaultFlags {
        type Output = Self;
        /// Returns the union of the two sets of flags.
        #[inline]
        fn bitor(self, other: VaultFlags) -> Self {
            Self {
                bits: self.bits | other.bits,
            }
        }
    }
    impl ::bitflags::_core::ops::BitOrAssign for VaultFlags {
        /// Adds the set of flags.
        #[inline]
        fn bitor_assign(&mut self, other: Self) {
            self.bits |= other.bits;
        }
    }
    impl ::bitflags::_core::ops::BitXor for VaultFlags {
        type Output = Self;
        /// Returns the left flags, but with all the right flags toggled.
        #[inline]
        fn bitxor(self, other: Self) -> Self {
            Self {
                bits: self.bits ^ other.bits,
            }
        }
    }
    impl ::bitflags::_core::ops::BitXorAssign for VaultFlags {
        /// Toggles the set of flags.
        #[inline]
        fn bitxor_assign(&mut self, other: Self) {
            self.bits ^= other.bits;
        }
    }
    impl ::bitflags::_core::ops::BitAnd for VaultFlags {
        type Output = Self;
        /// Returns the intersection between the two sets of flags.
        #[inline]
        fn bitand(self, other: Self) -> Self {
            Self {
                bits: self.bits & other.bits,
            }
        }
    }
    impl ::bitflags::_core::ops::BitAndAssign for VaultFlags {
        /// Disables all flags disabled in the set.
        #[inline]
        fn bitand_assign(&mut self, other: Self) {
            self.bits &= other.bits;
        }
    }
    impl ::bitflags::_core::ops::Sub for VaultFlags {
        type Output = Self;
        /// Returns the set difference of the two sets of flags.
        #[inline]
        fn sub(self, other: Self) -> Self {
            Self {
                bits: self.bits & !other.bits,
            }
        }
    }
    impl ::bitflags::_core::ops::SubAssign for VaultFlags {
        /// Disables all flags enabled in the set.
        #[inline]
        fn sub_assign(&mut self, other: Self) {
            self.bits &= !other.bits;
        }
    }
    impl ::bitflags::_core::ops::Not for VaultFlags {
        type Output = Self;
        /// Returns the complement of this set of flags.
        #[inline]
        fn not(self) -> Self {
            Self { bits: !self.bits } & Self::all()
        }
    }
    impl ::bitflags::_core::iter::Extend<VaultFlags> for VaultFlags {
        fn extend<T: ::bitflags::_core::iter::IntoIterator<Item = Self>>(&mut self, iterator: T) {
            for item in iterator {
                self.insert(item)
            }
        }
    }
    impl ::bitflags::_core::iter::FromIterator<VaultFlags> for VaultFlags {
        fn from_iter<T: ::bitflags::_core::iter::IntoIterator<Item = Self>>(iterator: T) -> Self {
            let mut result = Self::empty();
            result.extend(iterator);
            result
        }
    }
    pub struct YieldSourceFlags {
        bits: u16,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for YieldSourceFlags {}
    impl ::core::marker::StructuralPartialEq for YieldSourceFlags {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for YieldSourceFlags {
        #[inline]
        fn eq(&self, other: &YieldSourceFlags) -> bool {
            match *other {
                YieldSourceFlags {
                    bits: ref __self_1_0,
                } => match *self {
                    YieldSourceFlags {
                        bits: ref __self_0_0,
                    } => (*__self_0_0) == (*__self_1_0),
                },
            }
        }
        #[inline]
        fn ne(&self, other: &YieldSourceFlags) -> bool {
            match *other {
                YieldSourceFlags {
                    bits: ref __self_1_0,
                } => match *self {
                    YieldSourceFlags {
                        bits: ref __self_0_0,
                    } => (*__self_0_0) != (*__self_1_0),
                },
            }
        }
    }
    impl ::core::marker::StructuralEq for YieldSourceFlags {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for YieldSourceFlags {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {
            {
                let _: ::core::cmp::AssertParamIsEq<u16>;
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for YieldSourceFlags {
        #[inline]
        fn clone(&self) -> YieldSourceFlags {
            {
                let _: ::core::clone::AssertParamIsClone<u16>;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialOrd for YieldSourceFlags {
        #[inline]
        fn partial_cmp(
            &self,
            other: &YieldSourceFlags,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match *other {
                YieldSourceFlags {
                    bits: ref __self_1_0,
                } => match *self {
                    YieldSourceFlags {
                        bits: ref __self_0_0,
                    } => match ::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0), &(*__self_1_0))
                    {
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                        }
                        cmp => cmp,
                    },
                },
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Ord for YieldSourceFlags {
        #[inline]
        fn cmp(&self, other: &YieldSourceFlags) -> ::core::cmp::Ordering {
            match *other {
                YieldSourceFlags {
                    bits: ref __self_1_0,
                } => match *self {
                    YieldSourceFlags {
                        bits: ref __self_0_0,
                    } => match ::core::cmp::Ord::cmp(&(*__self_0_0), &(*__self_1_0)) {
                        ::core::cmp::Ordering::Equal => ::core::cmp::Ordering::Equal,
                        cmp => cmp,
                    },
                },
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::hash::Hash for YieldSourceFlags {
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            match *self {
                YieldSourceFlags {
                    bits: ref __self_0_0,
                } => ::core::hash::Hash::hash(&(*__self_0_0), state),
            }
        }
    }
    impl ::bitflags::_core::fmt::Debug for YieldSourceFlags {
        fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter) -> ::bitflags::_core::fmt::Result {
            #[allow(non_snake_case)]
            trait __BitFlags {
                #[inline]
                fn SOLEND(&self) -> bool {
                    false
                }
                #[inline]
                fn PORT(&self) -> bool {
                    false
                }
                #[inline]
                fn JET(&self) -> bool {
                    false
                }
            }
            #[allow(non_snake_case)]
            impl __BitFlags for YieldSourceFlags {
                #[allow(deprecated)]
                #[inline]
                fn SOLEND(&self) -> bool {
                    if Self::SOLEND.bits == 0 && self.bits != 0 {
                        false
                    } else {
                        self.bits & Self::SOLEND.bits == Self::SOLEND.bits
                    }
                }
                #[allow(deprecated)]
                #[inline]
                fn PORT(&self) -> bool {
                    if Self::PORT.bits == 0 && self.bits != 0 {
                        false
                    } else {
                        self.bits & Self::PORT.bits == Self::PORT.bits
                    }
                }
                #[allow(deprecated)]
                #[inline]
                fn JET(&self) -> bool {
                    if Self::JET.bits == 0 && self.bits != 0 {
                        false
                    } else {
                        self.bits & Self::JET.bits == Self::JET.bits
                    }
                }
            }
            let mut first = true;
            if <Self as __BitFlags>::SOLEND(self) {
                if !first {
                    f.write_str(" | ")?;
                }
                first = false;
                f.write_str("SOLEND")?;
            }
            if <Self as __BitFlags>::PORT(self) {
                if !first {
                    f.write_str(" | ")?;
                }
                first = false;
                f.write_str("PORT")?;
            }
            if <Self as __BitFlags>::JET(self) {
                if !first {
                    f.write_str(" | ")?;
                }
                first = false;
                f.write_str("JET")?;
            }
            let extra_bits = self.bits & !Self::all().bits();
            if extra_bits != 0 {
                if !first {
                    f.write_str(" | ")?;
                }
                first = false;
                f.write_str("0x")?;
                ::bitflags::_core::fmt::LowerHex::fmt(&extra_bits, f)?;
            }
            if first {
                f.write_str("(empty)")?;
            }
            Ok(())
        }
    }
    impl ::bitflags::_core::fmt::Binary for YieldSourceFlags {
        fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter) -> ::bitflags::_core::fmt::Result {
            ::bitflags::_core::fmt::Binary::fmt(&self.bits, f)
        }
    }
    impl ::bitflags::_core::fmt::Octal for YieldSourceFlags {
        fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter) -> ::bitflags::_core::fmt::Result {
            ::bitflags::_core::fmt::Octal::fmt(&self.bits, f)
        }
    }
    impl ::bitflags::_core::fmt::LowerHex for YieldSourceFlags {
        fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter) -> ::bitflags::_core::fmt::Result {
            ::bitflags::_core::fmt::LowerHex::fmt(&self.bits, f)
        }
    }
    impl ::bitflags::_core::fmt::UpperHex for YieldSourceFlags {
        fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter) -> ::bitflags::_core::fmt::Result {
            ::bitflags::_core::fmt::UpperHex::fmt(&self.bits, f)
        }
    }
    #[allow(dead_code)]
    impl YieldSourceFlags {
        pub const SOLEND: Self = Self { bits: 1 << 0 };
        pub const PORT: Self = Self { bits: 1 << 1 };
        pub const JET: Self = Self { bits: 1 << 2 };
        /// Returns an empty set of flags.
        #[inline]
        pub const fn empty() -> Self {
            Self { bits: 0 }
        }
        /// Returns the set containing all flags.
        #[inline]
        pub const fn all() -> Self {
            #[allow(non_snake_case)]
            trait __BitFlags {
                const SOLEND: u16 = 0;
                const PORT: u16 = 0;
                const JET: u16 = 0;
            }
            #[allow(non_snake_case)]
            impl __BitFlags for YieldSourceFlags {
                #[allow(deprecated)]
                const SOLEND: u16 = Self::SOLEND.bits;
                #[allow(deprecated)]
                const PORT: u16 = Self::PORT.bits;
                #[allow(deprecated)]
                const JET: u16 = Self::JET.bits;
            }
            Self {
                bits: <Self as __BitFlags>::SOLEND
                    | <Self as __BitFlags>::PORT
                    | <Self as __BitFlags>::JET,
            }
        }
        /// Returns the raw value of the flags currently stored.
        #[inline]
        pub const fn bits(&self) -> u16 {
            self.bits
        }
        /// Convert from underlying bit representation, unless that
        /// representation contains bits that do not correspond to a flag.
        #[inline]
        pub const fn from_bits(bits: u16) -> ::bitflags::_core::option::Option<Self> {
            if (bits & !Self::all().bits()) == 0 {
                ::bitflags::_core::option::Option::Some(Self { bits })
            } else {
                ::bitflags::_core::option::Option::None
            }
        }
        /// Convert from underlying bit representation, dropping any bits
        /// that do not correspond to flags.
        #[inline]
        pub const fn from_bits_truncate(bits: u16) -> Self {
            Self {
                bits: bits & Self::all().bits,
            }
        }
        /// Convert from underlying bit representation, preserving all
        /// bits (even those not corresponding to a defined flag).
        ///
        /// # Safety
        ///
        /// The caller of the `bitflags!` macro can chose to allow or
        /// disallow extra bits for their bitflags type.
        ///
        /// The caller of `from_bits_unchecked()` has to ensure that
        /// all bits correspond to a defined flag or that extra bits
        /// are valid for this bitflags type.
        #[inline]
        pub const unsafe fn from_bits_unchecked(bits: u16) -> Self {
            Self { bits }
        }
        /// Returns `true` if no flags are currently stored.
        #[inline]
        pub const fn is_empty(&self) -> bool {
            self.bits() == Self::empty().bits()
        }
        /// Returns `true` if all flags are currently set.
        #[inline]
        pub const fn is_all(&self) -> bool {
            Self::all().bits | self.bits == self.bits
        }
        /// Returns `true` if there are flags common to both `self` and `other`.
        #[inline]
        pub const fn intersects(&self, other: Self) -> bool {
            !(Self {
                bits: self.bits & other.bits,
            })
            .is_empty()
        }
        /// Returns `true` if all of the flags in `other` are contained within `self`.
        #[inline]
        pub const fn contains(&self, other: Self) -> bool {
            (self.bits & other.bits) == other.bits
        }
        /// Inserts the specified flags in-place.
        #[inline]
        pub fn insert(&mut self, other: Self) {
            self.bits |= other.bits;
        }
        /// Removes the specified flags in-place.
        #[inline]
        pub fn remove(&mut self, other: Self) {
            self.bits &= !other.bits;
        }
        /// Toggles the specified flags in-place.
        #[inline]
        pub fn toggle(&mut self, other: Self) {
            self.bits ^= other.bits;
        }
        /// Inserts or removes the specified flags depending on the passed value.
        #[inline]
        pub fn set(&mut self, other: Self, value: bool) {
            if value {
                self.insert(other);
            } else {
                self.remove(other);
            }
        }
        /// Returns the intersection between the flags in `self` and
        /// `other`.
        ///
        /// Specifically, the returned set contains only the flags which are
        /// present in *both* `self` *and* `other`.
        ///
        /// This is equivalent to using the `&` operator (e.g.
        /// [`ops::BitAnd`]), as in `flags & other`.
        ///
        /// [`ops::BitAnd`]: https://doc.rust-lang.org/std/ops/trait.BitAnd.html
        #[inline]
        #[must_use]
        pub const fn intersection(self, other: Self) -> Self {
            Self {
                bits: self.bits & other.bits,
            }
        }
        /// Returns the union of between the flags in `self` and `other`.
        ///
        /// Specifically, the returned set contains all flags which are
        /// present in *either* `self` *or* `other`, including any which are
        /// present in both (see [`Self::symmetric_difference`] if that
        /// is undesirable).
        ///
        /// This is equivalent to using the `|` operator (e.g.
        /// [`ops::BitOr`]), as in `flags | other`.
        ///
        /// [`ops::BitOr`]: https://doc.rust-lang.org/std/ops/trait.BitOr.html
        #[inline]
        #[must_use]
        pub const fn union(self, other: Self) -> Self {
            Self {
                bits: self.bits | other.bits,
            }
        }
        /// Returns the difference between the flags in `self` and `other`.
        ///
        /// Specifically, the returned set contains all flags present in
        /// `self`, except for the ones present in `other`.
        ///
        /// It is also conceptually equivalent to the "bit-clear" operation:
        /// `flags & !other` (and this syntax is also supported).
        ///
        /// This is equivalent to using the `-` operator (e.g.
        /// [`ops::Sub`]), as in `flags - other`.
        ///
        /// [`ops::Sub`]: https://doc.rust-lang.org/std/ops/trait.Sub.html
        #[inline]
        #[must_use]
        pub const fn difference(self, other: Self) -> Self {
            Self {
                bits: self.bits & !other.bits,
            }
        }
        /// Returns the [symmetric difference][sym-diff] between the flags
        /// in `self` and `other`.
        ///
        /// Specifically, the returned set contains the flags present which
        /// are present in `self` or `other`, but that are not present in
        /// both. Equivalently, it contains the flags present in *exactly
        /// one* of the sets `self` and `other`.
        ///
        /// This is equivalent to using the `^` operator (e.g.
        /// [`ops::BitXor`]), as in `flags ^ other`.
        ///
        /// [sym-diff]: https://en.wikipedia.org/wiki/Symmetric_difference
        /// [`ops::BitXor`]: https://doc.rust-lang.org/std/ops/trait.BitXor.html
        #[inline]
        #[must_use]
        pub const fn symmetric_difference(self, other: Self) -> Self {
            Self {
                bits: self.bits ^ other.bits,
            }
        }
        /// Returns the complement of this set of flags.
        ///
        /// Specifically, the returned set contains all the flags which are
        /// not set in `self`, but which are allowed for this type.
        ///
        /// Alternatively, it can be thought of as the set difference
        /// between [`Self::all()`] and `self` (e.g. `Self::all() - self`)
        ///
        /// This is equivalent to using the `!` operator (e.g.
        /// [`ops::Not`]), as in `!flags`.
        ///
        /// [`Self::all()`]: Self::all
        /// [`ops::Not`]: https://doc.rust-lang.org/std/ops/trait.Not.html
        #[inline]
        #[must_use]
        pub const fn complement(self) -> Self {
            Self::from_bits_truncate(!self.bits)
        }
    }
    impl ::bitflags::_core::ops::BitOr for YieldSourceFlags {
        type Output = Self;
        /// Returns the union of the two sets of flags.
        #[inline]
        fn bitor(self, other: YieldSourceFlags) -> Self {
            Self {
                bits: self.bits | other.bits,
            }
        }
    }
    impl ::bitflags::_core::ops::BitOrAssign for YieldSourceFlags {
        /// Adds the set of flags.
        #[inline]
        fn bitor_assign(&mut self, other: Self) {
            self.bits |= other.bits;
        }
    }
    impl ::bitflags::_core::ops::BitXor for YieldSourceFlags {
        type Output = Self;
        /// Returns the left flags, but with all the right flags toggled.
        #[inline]
        fn bitxor(self, other: Self) -> Self {
            Self {
                bits: self.bits ^ other.bits,
            }
        }
    }
    impl ::bitflags::_core::ops::BitXorAssign for YieldSourceFlags {
        /// Toggles the set of flags.
        #[inline]
        fn bitxor_assign(&mut self, other: Self) {
            self.bits ^= other.bits;
        }
    }
    impl ::bitflags::_core::ops::BitAnd for YieldSourceFlags {
        type Output = Self;
        /// Returns the intersection between the two sets of flags.
        #[inline]
        fn bitand(self, other: Self) -> Self {
            Self {
                bits: self.bits & other.bits,
            }
        }
    }
    impl ::bitflags::_core::ops::BitAndAssign for YieldSourceFlags {
        /// Disables all flags disabled in the set.
        #[inline]
        fn bitand_assign(&mut self, other: Self) {
            self.bits &= other.bits;
        }
    }
    impl ::bitflags::_core::ops::Sub for YieldSourceFlags {
        type Output = Self;
        /// Returns the set difference of the two sets of flags.
        #[inline]
        fn sub(self, other: Self) -> Self {
            Self {
                bits: self.bits & !other.bits,
            }
        }
    }
    impl ::bitflags::_core::ops::SubAssign for YieldSourceFlags {
        /// Disables all flags enabled in the set.
        #[inline]
        fn sub_assign(&mut self, other: Self) {
            self.bits &= !other.bits;
        }
    }
    impl ::bitflags::_core::ops::Not for YieldSourceFlags {
        type Output = Self;
        /// Returns the complement of this set of flags.
        #[inline]
        fn not(self) -> Self {
            Self { bits: !self.bits } & Self::all()
        }
    }
    impl ::bitflags::_core::iter::Extend<YieldSourceFlags> for YieldSourceFlags {
        fn extend<T: ::bitflags::_core::iter::IntoIterator<Item = Self>>(&mut self, iterator: T) {
            for item in iterator {
                self.insert(item)
            }
        }
    }
    impl ::bitflags::_core::iter::FromIterator<YieldSourceFlags> for YieldSourceFlags {
        fn from_iter<T: ::bitflags::_core::iter::IntoIterator<Item = Self>>(iterator: T) -> Self {
            let mut result = Self::empty();
            result.extend(iterator);
            result
        }
    }
    #[allow(unknown_lints, eq_op)]
    const _: [(); 0 - !{
        const ASSERT: bool = 0 == std::mem::size_of::<Allocations>() % 8;
        ASSERT
    } as usize] = [];
    #[allow(unknown_lints, eq_op)]
    const _: [(); 0 - !{
        const ASSERT: bool = 72usize == std::mem::size_of::<Allocations>();
        ASSERT
    } as usize] = [];
    #[repr(C, align(8))]
    pub struct Allocations {
        pub solend: SlotTrackedValue,
        pub port: SlotTrackedValue,
        pub jet: SlotTrackedValue,
    }
    impl borsh::de::BorshDeserialize for Allocations
    where
        SlotTrackedValue: borsh::BorshDeserialize,
        SlotTrackedValue: borsh::BorshDeserialize,
        SlotTrackedValue: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                solend: borsh::BorshDeserialize::deserialize(buf)?,
                port: borsh::BorshDeserialize::deserialize(buf)?,
                jet: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl borsh::ser::BorshSerialize for Allocations
    where
        SlotTrackedValue: borsh::ser::BorshSerialize,
        SlotTrackedValue: borsh::ser::BorshSerialize,
        SlotTrackedValue: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.solend, writer)?;
            borsh::BorshSerialize::serialize(&self.port, writer)?;
            borsh::BorshSerialize::serialize(&self.jet, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Allocations {
        #[inline]
        fn clone(&self) -> Allocations {
            {
                let _: ::core::clone::AssertParamIsClone<SlotTrackedValue>;
                let _: ::core::clone::AssertParamIsClone<SlotTrackedValue>;
                let _: ::core::clone::AssertParamIsClone<SlotTrackedValue>;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Allocations {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Allocations {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Allocations {
                    solend: ref __self_0_0,
                    port: ref __self_0_1,
                    jet: ref __self_0_2,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "Allocations");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "solend",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "port",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "jet",
                        &&(*__self_0_2),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::default::Default for Allocations {
        #[inline]
        fn default() -> Allocations {
            Allocations {
                solend: ::core::default::Default::default(),
                port: ::core::default::Default::default(),
                jet: ::core::default::Default::default(),
            }
        }
    }
    impl core::ops::Index<Provider> for Allocations {
        type Output = SlotTrackedValue;
        fn index(&self, provider: Provider) -> &Self::Output {
            match provider {
                Provider::Solend => &self.solend,
                Provider::Port => &self.port,
                Provider::Jet => &self.jet,
            }
        }
    }
    impl core::ops::IndexMut<Provider> for Allocations {
        fn index_mut(&mut self, provider: Provider) -> &mut Self::Output {
            match provider {
                Provider::Solend => &mut self.solend,
                Provider::Port => &mut self.port,
                Provider::Jet => &mut self.jet,
            }
        }
    }
    impl Allocations {
        pub fn from_container(c: AssetContainer<u64>, slot: u64) -> Self {
            Provider::iter().fold(Self::default(), |mut acc, provider| {
                match c[provider] {
                    Some(v) => acc[provider].update(v, slot),
                    None => {}
                };
                acc
            })
        }
    }
    #[repr(C, align(8))]
    pub struct SlotTrackedValue {
        pub value: u64,
        pub last_update: LastUpdate,
    }
    impl borsh::de::BorshDeserialize for SlotTrackedValue
    where
        u64: borsh::BorshDeserialize,
        LastUpdate: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                value: borsh::BorshDeserialize::deserialize(buf)?,
                last_update: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl borsh::ser::BorshSerialize for SlotTrackedValue
    where
        u64: borsh::ser::BorshSerialize,
        LastUpdate: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.value, writer)?;
            borsh::BorshSerialize::serialize(&self.last_update, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for SlotTrackedValue {
        #[inline]
        fn clone(&self) -> SlotTrackedValue {
            {
                let _: ::core::clone::AssertParamIsClone<u64>;
                let _: ::core::clone::AssertParamIsClone<LastUpdate>;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for SlotTrackedValue {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for SlotTrackedValue {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                SlotTrackedValue {
                    value: ref __self_0_0,
                    last_update: ref __self_0_1,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "SlotTrackedValue");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "value",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "last_update",
                        &&(*__self_0_1),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::default::Default for SlotTrackedValue {
        #[inline]
        fn default() -> SlotTrackedValue {
            SlotTrackedValue {
                value: ::core::default::Default::default(),
                last_update: ::core::default::Default::default(),
            }
        }
    }
    impl SlotTrackedValue {
        pub fn update(&mut self, value: u64, slot: u64) {
            self.value = value;
            self.last_update.update_slot(slot);
        }
        pub fn reset(&mut self) {
            self.value = 0;
            self.last_update.mark_stale();
        }
    }
    /// Number of slots to consider stale after
    pub const STALE_AFTER_SLOTS_ELAPSED: u64 = 2;
    #[allow(unknown_lints, eq_op)]
    const _: [(); 0 - !{
        const ASSERT: bool = 0 == std::mem::size_of::<LastUpdate>() % 8;
        ASSERT
    } as usize] = [];
    #[allow(unknown_lints, eq_op)]
    const _: [(); 0 - !{
        const ASSERT: bool = 16usize == std::mem::size_of::<LastUpdate>();
        ASSERT
    } as usize] = [];
    #[repr(C, align(8))]
    pub struct LastUpdate {
        pub slot: u64,
        pub stale: bool,
        _padding: [u8; 7],
    }
    impl borsh::de::BorshDeserialize for LastUpdate
    where
        u64: borsh::BorshDeserialize,
        bool: borsh::BorshDeserialize,
        [u8; 7]: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                slot: borsh::BorshDeserialize::deserialize(buf)?,
                stale: borsh::BorshDeserialize::deserialize(buf)?,
                _padding: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl borsh::ser::BorshSerialize for LastUpdate
    where
        u64: borsh::ser::BorshSerialize,
        bool: borsh::ser::BorshSerialize,
        [u8; 7]: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.slot, writer)?;
            borsh::BorshSerialize::serialize(&self.stale, writer)?;
            borsh::BorshSerialize::serialize(&self._padding, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for LastUpdate {
        #[inline]
        fn clone(&self) -> LastUpdate {
            {
                let _: ::core::clone::AssertParamIsClone<u64>;
                let _: ::core::clone::AssertParamIsClone<bool>;
                let _: ::core::clone::AssertParamIsClone<[u8; 7]>;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for LastUpdate {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for LastUpdate {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                LastUpdate {
                    slot: ref __self_0_0,
                    stale: ref __self_0_1,
                    _padding: ref __self_0_2,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "LastUpdate");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "slot",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "stale",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "_padding",
                        &&(*__self_0_2),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::default::Default for LastUpdate {
        #[inline]
        fn default() -> LastUpdate {
            LastUpdate {
                slot: ::core::default::Default::default(),
                stale: ::core::default::Default::default(),
                _padding: ::core::default::Default::default(),
            }
        }
    }
    impl LastUpdate {
        /// Create new last update
        pub fn new(slot: u64) -> Self {
            Self {
                slot,
                stale: true,
                _padding: [0_u8; 7],
            }
        }
        /// Return slots elapsed since given slot
        pub fn slots_elapsed(&self, slot: u64) -> Result<u64, ProgramError> {
            slot.checked_sub(self.slot)
                .ok_or_else(|| ErrorCode::MathError.into())
        }
        /// Set last update slot
        pub fn update_slot(&mut self, slot: u64) {
            self.slot = slot;
            self.stale = false;
        }
        /// Set stale to true
        pub fn mark_stale(&mut self) {
            self.stale = true;
        }
        /// Check if marked stale or last update slot is too long ago
        pub fn is_stale(&self, slot: u64) -> Result<bool, ProgramError> {
            #[cfg(feature = "debug")]
            {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Last updated slot: "],
                        &match (&self.slot,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Current slot: "],
                        &match (&slot,) {
                            _args => [::core::fmt::ArgumentV1::new(
                                _args.0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                });
            }
            Ok(self.stale || self.slots_elapsed(slot)? >= STALE_AFTER_SLOTS_ELAPSED)
        }
    }
    impl PartialEq for LastUpdate {
        fn eq(&self, other: &Self) -> bool {
            self.slot == other.slot
        }
    }
    impl PartialOrd for LastUpdate {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            self.slot.partial_cmp(&other.slot)
        }
    }
}
use adapters::*;
use instructions::*;
/// The static program ID
pub static ID: anchor_lang::solana_program::pubkey::Pubkey =
    anchor_lang::solana_program::pubkey::Pubkey::new_from_array([
        57u8, 192u8, 89u8, 98u8, 211u8, 38u8, 203u8, 254u8, 231u8, 146u8, 15u8, 45u8, 142u8, 9u8,
        203u8, 35u8, 248u8, 226u8, 46u8, 130u8, 126u8, 105u8, 89u8, 32u8, 64u8, 130u8, 64u8, 194u8,
        67u8, 0u8, 56u8, 32u8,
    ]);
/// Confirms that a given pubkey is equivalent to the program ID
pub fn check_id(id: &anchor_lang::solana_program::pubkey::Pubkey) -> bool {
    id == &ID
}
/// Returns the program ID
pub fn id() -> anchor_lang::solana_program::pubkey::Pubkey {
    ID
}
use castle_vault::*;
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
    let (program_id, accounts, instruction_data) =
        unsafe { ::solana_program::entrypoint::deserialize(input) };
    match entry(&program_id, &accounts, &instruction_data) {
        Ok(()) => ::solana_program::entrypoint::SUCCESS,
        Err(error) => error.into(),
    }
}
/// The Anchor codegen exposes a programming model where a user defines
/// a set of methods inside of a `#[program]` module in a way similar
/// to writing RPC request handlers. The macro then generates a bunch of
/// code wrapping these user defined methods into something that can be
/// executed on Solana.
///
/// These methods fall into one of three categories, each of which
/// can be considered a different "namespace" of the program.
///
/// 1) Global methods - regular methods inside of the `#[program]`.
/// 2) State methods - associated methods inside a `#[state]` struct.
/// 3) Interface methods - methods inside a strait struct's
///    implementation of an `#[interface]` trait.
///
/// Care must be taken by the codegen to prevent collisions between
/// methods in these different namespaces. For this reason, Anchor uses
/// a variant of sighash to perform method dispatch, rather than
/// something like a simple enum variant discriminator.
///
/// The execution flow of the generated code can be roughly outlined:
///
/// * Start program via the entrypoint.
/// * Strip method identifier off the first 8 bytes of the instruction
///   data and invoke the identified method. The method identifier
///   is a variant of sighash. See docs.rs for `anchor_lang` for details.
/// * If the method identifier is an IDL identifier, execute the IDL
///   instructions, which are a special set of hardcoded instructions
///   baked into every Anchor program. Then exit.
/// * Otherwise, the method identifier is for a user defined
///   instruction, i.e., one of the methods in the user defined
///   `#[program]` module. Perform method dispatch, i.e., execute the
///   big match statement mapping method identifier to method handler
///   wrapper.
/// * Run the method handler wrapper. This wraps the code the user
///   actually wrote, deserializing the accounts, constructing the
///   context, invoking the user's code, and finally running the exit
///   routine, which typically persists account changes.
///
/// The `entry` function here, defines the standard entry to a Solana
/// program, where execution begins.
#[cfg(not(feature = "no-entrypoint"))]
pub fn entry(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 8 {
        return Err(anchor_lang::__private::ErrorCode::InstructionMissing.into());
    }
    dispatch(program_id, accounts, data).map_err(|e| {
        ::solana_program::log::sol_log(&e.to_string());
        e
    })
}
pub mod program {
    use super::*;
    /// Type representing the program.
    pub struct CastleVault;
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for CastleVault {
        #[inline]
        fn clone(&self) -> CastleVault {
            match *self {
                CastleVault => CastleVault,
            }
        }
    }
    impl anchor_lang::AccountDeserialize for CastleVault {
        fn try_deserialize(
            buf: &mut &[u8],
        ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
        {
            CastleVault::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(
            _buf: &mut &[u8],
        ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError>
        {
            Ok(CastleVault)
        }
    }
    impl anchor_lang::Id for CastleVault {
        fn id() -> Pubkey {
            ID
        }
    }
}
/// Performs method dispatch.
///
/// Each method in an anchor program is uniquely defined by a namespace
/// and a rust identifier (i.e., the name given to the method). These
/// two pieces can be combined to creater a method identifier,
/// specifically, Anchor uses
///
/// Sha256("<namespace>::<rust-identifier>")[..8],
///
/// where the namespace can be one of three types. 1) "global" for a
/// regular instruction, 2) "state" for a state struct instruction
/// handler and 3) a trait namespace (used in combination with the
/// `#[interface]` attribute), which is defined by the trait name, e..
/// `MyTrait`.
///
/// With this 8 byte identifier, Anchor performs method dispatch,
/// matching the given 8 byte identifier to the associated method
/// handler, which leads to user defined code being eventually invoked.
fn dispatch(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let mut ix_data: &[u8] = data;
    let sighash: [u8; 8] = {
        let mut sighash: [u8; 8] = [0; 8];
        sighash.copy_from_slice(&ix_data[..8]);
        ix_data = &ix_data[8..];
        sighash
    };
    if true {
        if sighash == anchor_lang::idl::IDL_IX_TAG.to_le_bytes() {
            return __private::__idl::__idl_dispatch(program_id, accounts, &ix_data);
        }
    }
    match sighash {
        [175, 175, 109, 31, 13, 152, 155, 237] => {
            __private::__global::initialize(program_id, accounts, ix_data)
        }
        [84, 10, 199, 148, 78, 63, 149, 39] => {
            __private::__global::initialize_jet(program_id, accounts, ix_data)
        }
        [35, 3, 184, 145, 21, 213, 152, 109] => {
            __private::__global::initialize_port(program_id, accounts, ix_data)
        }
        [185, 227, 121, 13, 217, 201, 27, 36] => {
            __private::__global::initialize_solend(program_id, accounts, ix_data)
        }
        [135, 111, 214, 209, 130, 125, 169, 244] => {
            __private::__global::update_halt_flags(program_id, accounts, ix_data)
        }
        [29, 158, 252, 191, 10, 83, 219, 99] => {
            __private::__global::update_config(program_id, accounts, ix_data)
        }
        [242, 35, 198, 137, 82, 225, 242, 182] => {
            __private::__global::deposit(program_id, accounts, ix_data)
        }
        [183, 18, 70, 156, 148, 109, 161, 34] => {
            __private::__global::withdraw(program_id, accounts, ix_data)
        }
        [108, 158, 77, 9, 210, 52, 88, 62] => {
            __private::__global::rebalance(program_id, accounts, ix_data)
        }
        [249, 145, 195, 29, 167, 123, 181, 142] => {
            __private::__global::refresh_solend(program_id, accounts, ix_data)
        }
        [161, 165, 121, 69, 157, 174, 193, 128] => {
            __private::__global::refresh_port(program_id, accounts, ix_data)
        }
        [151, 205, 134, 75, 164, 166, 93, 197] => {
            __private::__global::refresh_jet(program_id, accounts, ix_data)
        }
        [233, 95, 93, 170, 142, 189, 141, 255] => {
            __private::__global::consolidate_refresh(program_id, accounts, ix_data)
        }
        [23, 202, 38, 224, 16, 174, 102, 245] => {
            __private::__global::reconcile_solend(program_id, accounts, ix_data)
        }
        [54, 119, 231, 143, 70, 171, 255, 248] => {
            __private::__global::reconcile_port(program_id, accounts, ix_data)
        }
        [244, 245, 131, 5, 111, 215, 148, 35] => {
            __private::__global::reconcile_jet(program_id, accounts, ix_data)
        }
        _ => Err(anchor_lang::__private::ErrorCode::InstructionFallbackNotFound.into()),
    }
}
/// Create a private module to not clutter the program's namespace.
/// Defines an entrypoint for each individual instruction handler
/// wrapper.
mod __private {
    use super::*;
    /// __idl mod defines handlers for injected Anchor IDL instructions.
    pub mod __idl {
        use super::*;
        #[inline(never)]
        #[cfg(not(feature = "no-idl"))]
        pub fn __idl_dispatch(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            idl_ix_data: &[u8],
        ) -> ProgramResult {
            let mut accounts = accounts;
            let mut data: &[u8] = idl_ix_data;
            let ix = anchor_lang::idl::IdlInstruction::deserialize(&mut data)
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            match ix {
                anchor_lang::idl::IdlInstruction::Create { data_len } => {
                    let mut accounts = anchor_lang::idl::IdlCreateAccounts::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                    )?;
                    __idl_create_account(program_id, &mut accounts, data_len)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::CreateBuffer => {
                    let mut accounts = anchor_lang::idl::IdlCreateBuffer::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                    )?;
                    __idl_create_buffer(program_id, &mut accounts)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::Write { data } => {
                    let mut accounts = anchor_lang::idl::IdlAccounts::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                    )?;
                    __idl_write(program_id, &mut accounts, data)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::SetAuthority { new_authority } => {
                    let mut accounts = anchor_lang::idl::IdlAccounts::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                    )?;
                    __idl_set_authority(program_id, &mut accounts, new_authority)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::SetBuffer => {
                    let mut accounts = anchor_lang::idl::IdlSetBuffer::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                    )?;
                    __idl_set_buffer(program_id, &mut accounts)?;
                    accounts.exit(program_id)?;
                }
            }
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_create_account(
            program_id: &Pubkey,
            accounts: &mut anchor_lang::idl::IdlCreateAccounts,
            data_len: u64,
        ) -> ProgramResult {
            if program_id != accounts.program.key {
                return Err(anchor_lang::__private::ErrorCode::IdlInstructionInvalidProgram.into());
            }
            let from = accounts.from.key;
            let (base, nonce) = Pubkey::find_program_address(&[], program_id);
            let seed = anchor_lang::idl::IdlAccount::seed();
            let owner = accounts.program.key;
            let to = Pubkey::create_with_seed(&base, seed, owner).unwrap();
            let space = 8 + 32 + 4 + data_len as usize;
            let rent = Rent::get()?;
            let lamports = rent.minimum_balance(space);
            let seeds = &[&[nonce][..]];
            let ix = anchor_lang::solana_program::system_instruction::create_account_with_seed(
                from,
                &to,
                &base,
                seed,
                lamports,
                space as u64,
                owner,
            );
            anchor_lang::solana_program::program::invoke_signed(
                &ix,
                &[
                    accounts.from.clone(),
                    accounts.to.clone(),
                    accounts.base.clone(),
                    accounts.system_program.clone(),
                ],
                &[seeds],
            )?;
            let mut idl_account = {
                let mut account_data = accounts.to.try_borrow_data()?;
                let mut account_data_slice: &[u8] = &account_data;
                anchor_lang::idl::IdlAccount::try_deserialize_unchecked(&mut account_data_slice)?
            };
            idl_account.authority = *accounts.from.key;
            let mut data = accounts.to.try_borrow_mut_data()?;
            let dst: &mut [u8] = &mut data;
            let mut cursor = std::io::Cursor::new(dst);
            idl_account.try_serialize(&mut cursor)?;
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_create_buffer(
            program_id: &Pubkey,
            accounts: &mut anchor_lang::idl::IdlCreateBuffer,
        ) -> ProgramResult {
            let mut buffer = &mut accounts.buffer;
            buffer.authority = *accounts.authority.key;
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_write(
            program_id: &Pubkey,
            accounts: &mut anchor_lang::idl::IdlAccounts,
            idl_data: Vec<u8>,
        ) -> ProgramResult {
            let mut idl = &mut accounts.idl;
            idl.data.extend(idl_data);
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_set_authority(
            program_id: &Pubkey,
            accounts: &mut anchor_lang::idl::IdlAccounts,
            new_authority: Pubkey,
        ) -> ProgramResult {
            accounts.idl.authority = new_authority;
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_set_buffer(
            program_id: &Pubkey,
            accounts: &mut anchor_lang::idl::IdlSetBuffer,
        ) -> ProgramResult {
            accounts.idl.data = accounts.buffer.data.clone();
            Ok(())
        }
    }
    /// __state mod defines wrapped handlers for state instructions.
    pub mod __state {
        use super::*;
    }
    /// __interface mod defines wrapped handlers for `#[interface]` trait
    /// implementations.
    pub mod __interface {
        use super::*;
    }
    /// __global mod defines wrapped handlers for global instructions.
    pub mod __global {
        use super::*;
        #[inline(never)]
        pub fn initialize(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::Initialize::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::Initialize { _bumps, config } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                Initialize::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::initialize(
                Context::new(program_id, &mut accounts, remaining_accounts),
                _bumps,
                config,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn initialize_jet(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::InitializeJet::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::InitializeJet { bump } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                InitializeJet::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::initialize_jet(
                Context::new(program_id, &mut accounts, remaining_accounts),
                bump,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn initialize_port(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::InitializePort::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::InitializePort { bump } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                InitializePort::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::initialize_port(
                Context::new(program_id, &mut accounts, remaining_accounts),
                bump,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn initialize_solend(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::InitializeSolend::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::InitializeSolend { bump } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                InitializeSolend::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::initialize_solend(
                Context::new(program_id, &mut accounts, remaining_accounts),
                bump,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn update_halt_flags(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::UpdateHaltFlags::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::UpdateHaltFlags { flags } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                UpdateHaltFlags::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::update_halt_flags(
                Context::new(program_id, &mut accounts, remaining_accounts),
                flags,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn update_config(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::UpdateConfig::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::UpdateConfig { new_config } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                UpdateConfig::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::update_config(
                Context::new(program_id, &mut accounts, remaining_accounts),
                new_config,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn deposit(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::Deposit::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::Deposit {
                reserve_token_amount,
            } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts = Deposit::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::deposit(
                Context::new(program_id, &mut accounts, remaining_accounts),
                reserve_token_amount,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn withdraw(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::Withdraw::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::Withdraw { lp_token_amount } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                Withdraw::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::withdraw(
                Context::new(program_id, &mut accounts, remaining_accounts),
                lp_token_amount,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn rebalance(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::Rebalance::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::Rebalance { proposed_weights } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                Rebalance::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::rebalance(
                Context::new(program_id, &mut accounts, remaining_accounts),
                proposed_weights,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn refresh_solend(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::RefreshSolend::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::RefreshSolend = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                RefreshSolend::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::refresh_solend(Context::new(
                program_id,
                &mut accounts,
                remaining_accounts,
            ))?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn refresh_port(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::RefreshPort::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::RefreshPort = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                RefreshPort::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::refresh_port(Context::new(
                program_id,
                &mut accounts,
                remaining_accounts,
            ))?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn refresh_jet(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::RefreshJet::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::RefreshJet = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                RefreshJet::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::refresh_jet(Context::new(program_id, &mut accounts, remaining_accounts))?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn consolidate_refresh(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::ConsolidateRefresh::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::ConsolidateRefresh = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                ConsolidateRefresh::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::consolidate_refresh(Context::new(
                program_id,
                &mut accounts,
                remaining_accounts,
            ))?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn reconcile_solend(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::ReconcileSolend::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::ReconcileSolend { withdraw_option } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                SolendAccounts::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::reconcile_solend(
                Context::new(program_id, &mut accounts, remaining_accounts),
                withdraw_option,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn reconcile_port(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::ReconcilePort::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::ReconcilePort { withdraw_option } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                PortAccounts::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::reconcile_port(
                Context::new(program_id, &mut accounts, remaining_accounts),
                withdraw_option,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn reconcile_jet(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::ReconcileJet::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::ReconcileJet { withdraw_option } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                JetAccounts::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            castle_vault::reconcile_jet(
                Context::new(program_id, &mut accounts, remaining_accounts),
                withdraw_option,
            )?;
            accounts.exit(program_id)
        }
    }
}
pub mod castle_vault {
    use super::*;
    pub fn initialize(
        ctx: Context<Initialize>,
        _bumps: InitBumpSeeds,
        config: VaultConfigArg,
    ) -> ProgramResult {
        instructions::init_vault::handler(ctx, _bumps, config)
    }
    pub fn initialize_jet<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeJet<'info>>,
        bump: u8,
    ) -> ProgramResult {
        instructions::init_yield_source::handler(ctx, bump)
    }
    pub fn initialize_port<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializePort<'info>>,
        bump: u8,
    ) -> ProgramResult {
        instructions::init_yield_source::handler(ctx, bump)
    }
    pub fn initialize_solend<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeSolend<'info>>,
        bump: u8,
    ) -> ProgramResult {
        instructions::init_yield_source::handler(ctx, bump)
    }
    pub fn update_halt_flags(ctx: Context<UpdateHaltFlags>, flags: u16) -> ProgramResult {
        instructions::update_halt_flags::handler(ctx, flags)
    }
    pub fn update_config(ctx: Context<UpdateConfig>, new_config: VaultConfigArg) -> ProgramResult {
        instructions::update_config::handler(ctx, new_config)
    }
    pub fn deposit(ctx: Context<Deposit>, reserve_token_amount: u64) -> ProgramResult {
        instructions::deposit::handler(ctx, reserve_token_amount)
    }
    pub fn withdraw(ctx: Context<Withdraw>, lp_token_amount: u64) -> ProgramResult {
        instructions::withdraw::handler(ctx, lp_token_amount)
    }
    pub fn rebalance(
        ctx: Context<Rebalance>,
        proposed_weights: StrategyWeightsArg,
    ) -> ProgramResult {
        instructions::rebalance::handler(ctx, proposed_weights)
    }
    pub fn refresh_solend<'info>(
        ctx: Context<'_, '_, '_, 'info, RefreshSolend<'info>>,
    ) -> ProgramResult {
        instructions::refresh::handler(ctx)
    }
    pub fn refresh_port<'info>(
        ctx: Context<'_, '_, '_, 'info, RefreshPort<'info>>,
    ) -> ProgramResult {
        instructions::refresh::handler(ctx)
    }
    pub fn refresh_jet<'info>(ctx: Context<'_, '_, '_, 'info, RefreshJet<'info>>) -> ProgramResult {
        instructions::refresh::handler(ctx)
    }
    pub fn consolidate_refresh<'info>(
        ctx: Context<'_, '_, '_, 'info, ConsolidateRefresh<'info>>,
    ) -> ProgramResult {
        instructions::consolidate_refresh::handler(ctx)
    }
    pub fn reconcile_solend(ctx: Context<SolendAccounts>, withdraw_option: u64) -> ProgramResult {
        instructions::reconcile::handler(ctx, withdraw_option)
    }
    pub fn reconcile_port(ctx: Context<PortAccounts>, withdraw_option: u64) -> ProgramResult {
        instructions::reconcile::handler(ctx, withdraw_option)
    }
    pub fn reconcile_jet(ctx: Context<JetAccounts>, withdraw_option: u64) -> ProgramResult {
        instructions::reconcile::handler(ctx, withdraw_option)
    }
}
/// An Anchor generated module containing the program's set of
/// instructions, where each method handler in the `#[program]` mod is
/// associated with a struct defining the input arguments to the
/// method. These should be used directly, when one wants to serialize
/// Anchor instruction data, for example, when speciying
/// instructions on a client.
pub mod instruction {
    use super::*;
    /// Instruction struct definitions for `#[state]` methods.
    pub mod state {
        use super::*;
    }
    /// Instruction.
    pub struct Initialize {
        pub _bumps: InitBumpSeeds,
        pub config: VaultConfigArg,
    }
    impl borsh::ser::BorshSerialize for Initialize
    where
        InitBumpSeeds: borsh::ser::BorshSerialize,
        VaultConfigArg: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self._bumps, writer)?;
            borsh::BorshSerialize::serialize(&self.config, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for Initialize
    where
        InitBumpSeeds: borsh::BorshDeserialize,
        VaultConfigArg: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                _bumps: borsh::BorshDeserialize::deserialize(buf)?,
                config: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for Initialize {
        fn data(&self) -> Vec<u8> {
            let mut d = [175, 175, 109, 31, 13, 152, 155, 237].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct InitializeJet {
        pub bump: u8,
    }
    impl borsh::ser::BorshSerialize for InitializeJet
    where
        u8: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.bump, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for InitializeJet
    where
        u8: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                bump: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for InitializeJet {
        fn data(&self) -> Vec<u8> {
            let mut d = [84, 10, 199, 148, 78, 63, 149, 39].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct InitializePort {
        pub bump: u8,
    }
    impl borsh::ser::BorshSerialize for InitializePort
    where
        u8: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.bump, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for InitializePort
    where
        u8: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                bump: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for InitializePort {
        fn data(&self) -> Vec<u8> {
            let mut d = [35, 3, 184, 145, 21, 213, 152, 109].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct InitializeSolend {
        pub bump: u8,
    }
    impl borsh::ser::BorshSerialize for InitializeSolend
    where
        u8: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.bump, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for InitializeSolend
    where
        u8: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                bump: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for InitializeSolend {
        fn data(&self) -> Vec<u8> {
            let mut d = [185, 227, 121, 13, 217, 201, 27, 36].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct UpdateHaltFlags {
        pub flags: u16,
    }
    impl borsh::ser::BorshSerialize for UpdateHaltFlags
    where
        u16: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.flags, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for UpdateHaltFlags
    where
        u16: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                flags: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for UpdateHaltFlags {
        fn data(&self) -> Vec<u8> {
            let mut d = [135, 111, 214, 209, 130, 125, 169, 244].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct UpdateConfig {
        pub new_config: VaultConfigArg,
    }
    impl borsh::ser::BorshSerialize for UpdateConfig
    where
        VaultConfigArg: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.new_config, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for UpdateConfig
    where
        VaultConfigArg: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                new_config: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for UpdateConfig {
        fn data(&self) -> Vec<u8> {
            let mut d = [29, 158, 252, 191, 10, 83, 219, 99].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct Deposit {
        pub reserve_token_amount: u64,
    }
    impl borsh::ser::BorshSerialize for Deposit
    where
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.reserve_token_amount, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for Deposit
    where
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                reserve_token_amount: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for Deposit {
        fn data(&self) -> Vec<u8> {
            let mut d = [242, 35, 198, 137, 82, 225, 242, 182].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct Withdraw {
        pub lp_token_amount: u64,
    }
    impl borsh::ser::BorshSerialize for Withdraw
    where
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.lp_token_amount, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for Withdraw
    where
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                lp_token_amount: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for Withdraw {
        fn data(&self) -> Vec<u8> {
            let mut d = [183, 18, 70, 156, 148, 109, 161, 34].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct Rebalance {
        pub proposed_weights: StrategyWeightsArg,
    }
    impl borsh::ser::BorshSerialize for Rebalance
    where
        StrategyWeightsArg: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.proposed_weights, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for Rebalance
    where
        StrategyWeightsArg: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                proposed_weights: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for Rebalance {
        fn data(&self) -> Vec<u8> {
            let mut d = [108, 158, 77, 9, 210, 52, 88, 62].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct RefreshSolend;
    impl borsh::ser::BorshSerialize for RefreshSolend {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for RefreshSolend {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {})
        }
    }
    impl anchor_lang::InstructionData for RefreshSolend {
        fn data(&self) -> Vec<u8> {
            let mut d = [249, 145, 195, 29, 167, 123, 181, 142].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct RefreshPort;
    impl borsh::ser::BorshSerialize for RefreshPort {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for RefreshPort {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {})
        }
    }
    impl anchor_lang::InstructionData for RefreshPort {
        fn data(&self) -> Vec<u8> {
            let mut d = [161, 165, 121, 69, 157, 174, 193, 128].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct RefreshJet;
    impl borsh::ser::BorshSerialize for RefreshJet {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for RefreshJet {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {})
        }
    }
    impl anchor_lang::InstructionData for RefreshJet {
        fn data(&self) -> Vec<u8> {
            let mut d = [151, 205, 134, 75, 164, 166, 93, 197].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct ConsolidateRefresh;
    impl borsh::ser::BorshSerialize for ConsolidateRefresh {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for ConsolidateRefresh {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {})
        }
    }
    impl anchor_lang::InstructionData for ConsolidateRefresh {
        fn data(&self) -> Vec<u8> {
            let mut d = [233, 95, 93, 170, 142, 189, 141, 255].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct ReconcileSolend {
        pub withdraw_option: u64,
    }
    impl borsh::ser::BorshSerialize for ReconcileSolend
    where
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.withdraw_option, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for ReconcileSolend
    where
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                withdraw_option: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for ReconcileSolend {
        fn data(&self) -> Vec<u8> {
            let mut d = [23, 202, 38, 224, 16, 174, 102, 245].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct ReconcilePort {
        pub withdraw_option: u64,
    }
    impl borsh::ser::BorshSerialize for ReconcilePort
    where
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.withdraw_option, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for ReconcilePort
    where
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                withdraw_option: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for ReconcilePort {
        fn data(&self) -> Vec<u8> {
            let mut d = [54, 119, 231, 143, 70, 171, 255, 248].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct ReconcileJet {
        pub withdraw_option: u64,
    }
    impl borsh::ser::BorshSerialize for ReconcileJet
    where
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.withdraw_option, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for ReconcileJet
    where
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                withdraw_option: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for ReconcileJet {
        fn data(&self) -> Vec<u8> {
            let mut d = [244, 245, 131, 5, 111, 215, 148, 35].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
}
/// An Anchor generated module, providing a set of structs
/// mirroring the structs deriving `Accounts`, where each field is
/// a `Pubkey`. This is useful for specifying accounts for a client.
pub mod accounts {
    pub use crate::__client_accounts_initialize::*;
    pub use crate::__client_accounts_consolidate_refresh::*;
    pub use crate::__client_accounts_port_accounts::*;
    pub use crate::__client_accounts_jet_accounts::*;
    pub use crate::__client_accounts_initialize_port::*;
    pub use crate::__client_accounts_initialize_solend::*;
    pub use crate::__client_accounts_refresh_solend::*;
    pub use crate::__client_accounts_refresh_port::*;
    pub use crate::__client_accounts_refresh_jet::*;
    pub use crate::__client_accounts_deposit::*;
    pub use crate::__client_accounts_update_halt_flags::*;
    pub use crate::__client_accounts_update_config::*;
    pub use crate::__client_accounts_solend_accounts::*;
    pub use crate::__client_accounts_withdraw::*;
    pub use crate::__client_accounts_rebalance::*;
    pub use crate::__client_accounts_initialize_jet::*;
}
#[allow(dead_code)]
#[no_mangle]
pub static security_txt : & str = "=======BEGIN SECURITY.TXT V1=======\u{0}name\u{0}Castle Vault\u{0}project_url\u{0}https://castle.finance\u{0}contacts\u{0}telegram: @charlie_you, email:charlie@castle.finance\u{0}policy\u{0}https://docs.castle.finance/security-policy\u{0}preferred_languages\u{0}en\u{0}source_code\u{0}https://github.com/castle-finance/castle-vault/\u{0}encryption\u{0}\n-----BEGIN PGP PUBLIC KEY BLOCK-----\n\nmDMEYmQ/fRYJKwYBBAHaRw8BAQdA1biTOwYiyo7PNZATqAFXD3Ve1q0aG9wOHljo\n2akWnRK0JENoYXJsaWUgWW91IDxjaGFybGllQGNhc3RsZS5maW5hbmNlPoiTBBMW\nCgA7FiEEPUI91YfryrzyxGV2FoBM/GlFSGoFAmJkP30CGwMFCwkIBwICIgIGFQoJ\nCAsCBBYCAwECHgcCF4AACgkQFoBM/GlFSGq0sgEA0ANICcpzevxdMDOCKIO50w3j\nBZTSdVvh6coWL8JPiJgA/11V1Hdb/wFznAWLmJgHos3cSJwOoRf6a0pd82drqgMA\nuDgEYmQ/fRIKKwYBBAGXVQEFAQEHQO5aM48xdchjyIc3q3Bu3uE73DV6l8wrdDCn\n0sYB71QiAwEIB4h4BBgWCgAgFiEEPUI91YfryrzyxGV2FoBM/GlFSGoFAmJkP30C\nGwwACgkQFoBM/GlFSGpZnAEAlxxgUQR4Y6q3zmfPW+S+qneZnMj4p8JdzD8B4/aO\nNAgBAJzbmnb6RpW+5zMjjxFKJRjAelqCkuyBUO4Vk5GHaUAO\n=P067\n-----END PGP PUBLIC KEY BLOCK-----\n\u{0}auditors\u{0}Bramah Systems\u{0}acknowledgements\u{0}\u{0}=======END SECURITY.TXT V1=======\u{0}" ;
