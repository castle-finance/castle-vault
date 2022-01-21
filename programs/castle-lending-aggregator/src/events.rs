use anchor_lang::prelude::*;

#[event]
pub struct RebalanceEvent {
    pub solend: u64,
    pub port: u64,
    pub jet: u64,
}
