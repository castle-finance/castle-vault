use anchor_lang::prelude::*;

use crate::state::{Allocations, Provider};

// TODO might be able to delete since sim isn't done anymore 
#[event]
pub struct RebalanceEvent {
    pub solend: u64,
    pub port: u64,
    pub jet: u64,
}

// TODO connect this to same indexing?
impl From<&Allocations> for RebalanceEvent {
    fn from(allocations: &Allocations) -> Self {
        RebalanceEvent {
            solend: allocations[Provider::Solend].value,
            port: allocations[Provider::Port].value,
            jet: allocations[Provider::Jet].value,
        }
    }
}
