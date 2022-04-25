use solana_maths::Rate;

use super::BackendContainer;

impl From<BackendContainer<u16>> for BackendContainer<Rate> {
    fn from(c: BackendContainer<u16>) -> Self {
        c.apply(|_provider, v| Rate::from_bips(*v as u64))
    }
}
