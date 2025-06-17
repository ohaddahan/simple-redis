use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize, Debug)]
pub struct Prefix(pub String);
#[derive(Serialize, Deserialize, Debug)]
pub struct Key(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct Namespace(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub enum EvictionPolicy {
    NoEviction,
    AllKeysLRU,
    AllKeysLFU,
    AllKeysRandom,
    VolatileLRU,
    VolatileLFU,
    VolatileRandom,
    VolatileTTL,
}

impl Default for EvictionPolicy {
    fn default() -> Self {
        Self::NoEviction
    }
}

impl Display for EvictionPolicy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEviction => write!(f, "noeviction"),
            Self::AllKeysLRU => write!(f, "allkeys-lru"),
            Self::AllKeysLFU => write!(f, "allkeys-lfu"),
            Self::AllKeysRandom => write!(f, "allkeys-random"),
            Self::VolatileLRU => write!(f, "volatile-lru"),
            Self::VolatileLFU => write!(f, "volatile-lfu"),
            Self::VolatileRandom => write!(f, "volatile-random"),
            Self::VolatileTTL => write!(f, "volatile-ttl"),
        }
    }
}
