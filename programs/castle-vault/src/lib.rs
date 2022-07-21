use anchor_lang::prelude::*;

pub mod adapters;
pub mod asset_container;
pub mod errors;
pub mod instructions;
pub mod math;
pub mod reserves;
pub mod state;

use adapters::*;
use instructions::*;

#[cfg(not(feature = "devnet-castle-addr"))]
declare_id!("Cast1eoVj8hwfKKRPji4cqX7WFgcnYz3um7TTgnaJKFn");

#[cfg(feature = "devnet-castle-addr")]
declare_id!("4tSMVfVbnwZcDwZB1M1j27dx9hdjL72VR9GM8AykpAvK");

#[program]
pub mod castle_vault {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        authority_bump: u8,
        config: VaultConfigArg,
    ) -> Result<()> {
        instructions::init_vault::handler(ctx, authority_bump, config)
    }

    pub fn initialize_port_additional_state(
        ctx: Context<InitializePortAdditionalState>,
    ) -> Result<()> {
        instructions::init_port_additional_state::handler(ctx)
    }

    pub fn initialize_port_reward_accounts(
        ctx: Context<InitializePortRewardAccounts>,
        sub_reward_available: bool,
    ) -> Result<()> {
        instructions::init_port_reward_accounts::handler(
            ctx,
            sub_reward_available,
        )
    }

    pub fn initialize_port<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializePort<'info>>,
    ) -> Result<()> {
        instructions::init_yield_source::handler(ctx)
    }

    pub fn initialize_solend<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeSolend<'info>>,
    ) -> Result<()> {
        instructions::init_yield_source::handler(ctx)
    }

    pub fn update_halt_flags(ctx: Context<UpdateHaltFlags>, flags: u16) -> Result<()> {
        instructions::update_halt_flags::handler(ctx, flags)
    }

    pub fn update_yield_source_flags(
        ctx: Context<UpdateYieldSourceFlags>,
        flags: u16,
    ) -> Result<()> {
        instructions::update_yield_source_flags::handler(ctx, flags)
    }

    pub fn update_config(ctx: Context<UpdateConfig>, new_config: VaultConfigArg) -> Result<()> {
        instructions::update_config::handler(ctx, new_config)
    }

    pub fn deposit(ctx: Context<Deposit>, reserve_token_amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, reserve_token_amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, lp_token_amount: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, lp_token_amount)
    }

    pub fn rebalance(ctx: Context<Rebalance>, proposed_weights: StrategyWeightsArg) -> Result<()> {
        instructions::rebalance::handler(ctx, proposed_weights)
    }

    pub fn refresh_solend<'info>(
        ctx: Context<'_, '_, '_, 'info, RefreshSolend<'info>>,
    ) -> Result<()> {
        instructions::refresh::handler(ctx)
    }

    pub fn refresh_port<'info>(ctx: Context<'_, '_, '_, 'info, RefreshPort<'info>>) -> Result<()> {
        instructions::refresh::handler(ctx)
    }

    pub fn consolidate_refresh<'info>(
        ctx: Context<'_, '_, '_, 'info, ConsolidateRefresh<'info>>,
    ) -> Result<()> {
        instructions::consolidate_refresh::handler(ctx)
    }

    pub fn reconcile_solend(ctx: Context<SolendAccounts>, withdraw_option: u64) -> Result<()> {
        instructions::reconcile::handler(ctx, withdraw_option)
    }

    pub fn reconcile_port(ctx: Context<PortAccounts>, withdraw_option: u64) -> Result<()> {
        instructions::reconcile::handler(ctx, withdraw_option)
    }

    pub fn claim_port_reward(ctx: Context<ClaimPortReward>) -> Result<()> {
        instructions::claim_port_reward::handler(ctx)
    }
}

solana_security_txt::security_txt! {
    name: "Castle Vault",
    project_url: "https://castle.finance",
    contacts: "telegram: @charlie_you, email:charlie@castle.finance",
    policy: "https://docs.castle.finance/security-policy",
    preferred_languages: "en",
    source_code: "https://github.com/castle-finance/castle-vault/",
    encryption: "
-----BEGIN PGP PUBLIC KEY BLOCK-----

mDMEYmQ/fRYJKwYBBAHaRw8BAQdA1biTOwYiyo7PNZATqAFXD3Ve1q0aG9wOHljo
2akWnRK0JENoYXJsaWUgWW91IDxjaGFybGllQGNhc3RsZS5maW5hbmNlPoiTBBMW
CgA7FiEEPUI91YfryrzyxGV2FoBM/GlFSGoFAmJkP30CGwMFCwkIBwICIgIGFQoJ
CAsCBBYCAwECHgcCF4AACgkQFoBM/GlFSGq0sgEA0ANICcpzevxdMDOCKIO50w3j
BZTSdVvh6coWL8JPiJgA/11V1Hdb/wFznAWLmJgHos3cSJwOoRf6a0pd82drqgMA
uDgEYmQ/fRIKKwYBBAGXVQEFAQEHQO5aM48xdchjyIc3q3Bu3uE73DV6l8wrdDCn
0sYB71QiAwEIB4h4BBgWCgAgFiEEPUI91YfryrzyxGV2FoBM/GlFSGoFAmJkP30C
GwwACgkQFoBM/GlFSGpZnAEAlxxgUQR4Y6q3zmfPW+S+qneZnMj4p8JdzD8B4/aO
NAgBAJzbmnb6RpW+5zMjjxFKJRjAelqCkuyBUO4Vk5GHaUAO
=P067
-----END PGP PUBLIC KEY BLOCK-----
",
    auditors: "Bramah Systems",
    acknowledgements: ""
}
