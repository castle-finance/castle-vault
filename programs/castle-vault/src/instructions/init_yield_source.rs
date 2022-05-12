use anchor_lang::prelude::*;

pub trait YieldSourceInitializer<'info> {
    fn initialize_yield_source(&mut self) -> ProgramResult;
}

pub fn handler<'info, T: YieldSourceInitializer<'info>>(
    ctx: Context<'_, '_, '_, 'info, T>,
    bump: u8,
) -> ProgramResult {
    ctx.accounts.initialize_yield_source()
}
