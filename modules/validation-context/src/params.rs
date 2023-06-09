use crate::prelude::*;
use crate::tendermint::TendermintValidationParams;
use core::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationParams {
    Empty,
    Tendermint(TendermintValidationParams),
}

impl Default for ValidationParams {
    fn default() -> Self {
        Self::Empty
    }
}

impl Display for ValidationParams {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::Tendermint(params) => write!(f, "{}", params),
        }
    }
}

impl ValidationParams {
    pub fn to_vec(&self) -> Vec<u8> {
        use ValidationParams::*;

        match self {
            Empty => vec![0],
            Tendermint(params) => {
                let mut bz = Vec::new();
                bz.push(1);
                bz.extend(params.to_vec());
                bz
            }
        }
    }

    pub fn from_bytes(bz: &[u8]) -> Self {
        use ValidationParams::*;
        assert!(!bz.is_empty());
        match bz[0] {
            0 => Empty,
            1 => Tendermint(TendermintValidationParams::from_bytes(&bz[1..])),
            id => panic!("unknown type: {}", id),
        }
    }
}
