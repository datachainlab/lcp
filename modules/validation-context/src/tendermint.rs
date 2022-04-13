#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{params::ValidationParams, ValidationContext, ValidationPredicate};
use core::time::Duration;
use rlp::{Rlp, RlpStream};
use serde::{Deserialize, Serialize};
use std::vec::Vec;
use tendermint_light_client_verifier::{options::Options, types::TrustThreshold};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TendermintValidationParams {
    pub options: Options,
    pub header_timestamp: u64,
    pub trusted_consensus_state_timestamp: u64,
}

impl TendermintValidationParams {
    pub fn to_vec(&self) -> Vec<u8> {
        let mut s = RlpStream::new_list(3);
        s.begin_list(4)
            .append(&self.options.trust_threshold.numerator())
            .append(&self.options.trust_threshold.denominator())
            .append(&self.options.trusting_period.as_nanos())
            .append(&self.options.clock_drift.as_nanos());
        s.append(&self.header_timestamp);
        s.append(&self.trusted_consensus_state_timestamp);
        s.out().to_vec()
    }

    pub fn from_bytes(bz: &[u8]) -> Self {
        let root = Rlp::new(bz);
        let options = root.at(0).unwrap();

        Self {
            options: Options {
                trust_threshold: TrustThreshold::new(
                    options.val_at(0).unwrap(),
                    options.val_at(1).unwrap(),
                )
                .unwrap(),
                trusting_period: Duration::from_nanos(options.val_at(2).unwrap()),
                clock_drift: Duration::from_nanos(options.val_at(3).unwrap()),
            },
            header_timestamp: root.val_at(1).unwrap(),
            trusted_consensus_state_timestamp: root.val_at(2).unwrap(),
        }
    }
}

pub struct TendermintValidationPredicate;

impl ValidationPredicate for TendermintValidationPredicate {
    fn predicate(vctx: &ValidationContext, params: &ValidationParams) -> Result<bool, ()> {
        let params = match params {
            ValidationParams::Tendermint(params) => params,
            _ => unreachable!(),
        };
        todo!()
    }
}

#[cfg(all(test, not(feature = "sgx")))]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn serialization_validation_params() {
        let current_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let header_timestamp = current_timestamp.as_nanos() as u64;
        let trusted_consensus_state_timestamp = current_timestamp.as_nanos() as u64;

        let params = TendermintValidationParams {
            options: Options {
                trust_threshold: TrustThreshold::ONE_THIRD,
                trusting_period: Duration::new(60 * 60 * 24, 0),
                clock_drift: Duration::new(60 * 60, 0),
            },
            header_timestamp,
            trusted_consensus_state_timestamp,
        };
        let bz = params.to_vec();
        let new_params = TendermintValidationParams::from_bytes(&bz);
        assert!(params == new_params);
    }
}
