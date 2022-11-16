use crate::prelude::*;
use crate::{params::ValidationParams, ValidationContext, ValidationPredicate};
use core::fmt::Display;
use core::time::Duration;
use lcp_types::Time;
use rlp::{Rlp, RlpStream};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TendermintValidationParams {
    pub options: TendermintValidationOptions,
    pub untrusted_header_timestamp: Time,
    pub trusted_consensus_state_timestamp: Time,
}

impl Display for TendermintValidationParams {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "options={:?} untrusted_header_timestamp={} trusted_consensus_state_timestamp={}",
            self.options, self.untrusted_header_timestamp, self.trusted_consensus_state_timestamp
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TendermintValidationOptions {
    /// How long a validator set is trusted for (must be shorter than the chain's
    /// unbonding period)
    pub trusting_period: Duration,

    /// Correction parameter dealing with only approximately synchronized clocks.
    /// The local clock should always be ahead of timestamps from the blockchain; this
    /// is the maximum amount that the local clock may drift behind a timestamp from the
    /// blockchain.
    pub clock_drift: Duration,
}

impl TendermintValidationParams {
    pub fn to_vec(&self) -> Vec<u8> {
        let mut s = RlpStream::new_list(3);
        s.begin_list(2)
            .append(&self.options.trusting_period.as_nanos())
            .append(&self.options.clock_drift.as_nanos());
        s.append(&self.untrusted_header_timestamp.as_unix_timestamp_nanos());
        s.append(
            &self
                .trusted_consensus_state_timestamp
                .as_unix_timestamp_nanos(),
        );
        s.out().to_vec()
    }

    pub fn from_bytes(bz: &[u8]) -> Self {
        let root = Rlp::new(bz);
        let options = root.at(0).unwrap();

        Self {
            options: TendermintValidationOptions {
                trusting_period: Duration::from_nanos(options.val_at(0).unwrap()),
                clock_drift: Duration::from_nanos(options.val_at(1).unwrap()),
            },
            untrusted_header_timestamp: Time::from_unix_timestamp_nanos(root.val_at(1).unwrap())
                .unwrap(),
            trusted_consensus_state_timestamp: Time::from_unix_timestamp_nanos(
                root.val_at(2).unwrap(),
            )
            .unwrap(),
        }
    }
}

pub struct TendermintValidationPredicate;

impl TendermintValidationPredicate {
    fn is_within_trust_period(
        trusted_state_time: Time,
        trusting_period: Duration,
        now: Time,
    ) -> bool {
        (trusted_state_time + trusting_period).unwrap() > now
    }

    fn is_header_from_past(untrusted_header_time: Time, clock_drift: Duration, now: Time) -> bool {
        (now + clock_drift).unwrap() > untrusted_header_time
    }
}

impl ValidationPredicate for TendermintValidationPredicate {
    fn predicate(vctx: &ValidationContext, params: &ValidationParams) -> Result<bool, ()> {
        let params = match params {
            ValidationParams::Tendermint(params) => params,
            _ => unreachable!(),
        };

        // TODO return an error instead of assertion

        // ensure that trusted consensus state's timestamp hasn't passed the trusting period
        assert!(Self::is_within_trust_period(
            params.trusted_consensus_state_timestamp,
            params.options.trusting_period,
            vctx.current_timestamp.into(),
        ));

        // ensure the header isn't from a future time
        assert!(Self::is_header_from_past(
            params.untrusted_header_timestamp,
            params.options.clock_drift,
            vctx.current_timestamp.into(),
        ));

        Ok(true)
    }
}

#[cfg(all(test, not(feature = "sgx")))]
mod tests {
    use super::*;

    #[test]
    fn serialization_validation_params() {
        let current_timestamp = Time::now();
        let untrusted_header_timestamp = current_timestamp;
        let trusted_consensus_state_timestamp = current_timestamp;

        let params = TendermintValidationParams {
            options: TendermintValidationOptions {
                trusting_period: Duration::new(60 * 60 * 24, 0),
                clock_drift: Duration::new(60 * 60, 0),
            },
            untrusted_header_timestamp,
            trusted_consensus_state_timestamp,
        };
        let bz = params.to_vec();
        let new_params = TendermintValidationParams::from_bytes(&bz);
        assert!(params == new_params);
    }
}
