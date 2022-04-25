use anchor_lang::prelude::ProgramError;
use solana_maths::Rate;

use crate::{errors::ErrorCode, instructions::Reserves, state::StrategyType};

use super::BackendContainer;

impl BackendContainer<Reserves> {
    fn calculate_weights_max_yield(&self) -> Result<BackendContainer<Rate>, ProgramError> {
        self.into_iter()
            .max_by(|(_prov_x, alloc_x), (_prov_y, alloc_y)| {
                // TODO: can we remove the unwrap() in any way?
                self.compare(*alloc_x, *alloc_y).unwrap()
            })
            .map(|(max_yielding_provider, _a)| {
                self.apply(|provider, _v| {
                    if provider == max_yielding_provider {
                        Rate::one()
                    } else {
                        Rate::zero()
                    }
                })
            })
            // TODO make this error handling more granular and informative
            .ok_or(ErrorCode::StrategyError)
            .map_err(Into::into)
    }

    pub fn calculate_weights(
        &self,
        stype: StrategyType,
    ) -> Result<BackendContainer<Rate>, ProgramError> {
        match stype {
            StrategyType::MaxYield => self.calculate_weights_max_yield(),
            StrategyType::EqualAllocation => todo!(),
        }
    }
}
