use crate::prelude::*;
use crate::{Error, EthABIEncoder};
use core::{fmt::Display, time::Duration};
use lcp_types::Time;
use serde::{Deserialize, Serialize};

pub const COMMITMENT_CONTEXT_TYPE_NONE: u16 = 0;
pub const COMMITMENT_CONTEXT_TYPE_WITHIN_TRUSTING_PERIOD: u16 = 1;
pub const COMMITMENT_CONTEXT_HEADER_SIZE: usize = 32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommitmentContext {
    None,
    WithinTrustingPeriod(WithinTrustingPeriodContext),
}

impl CommitmentContext {
    pub fn validate(&self, current_timestamp: Time) -> Result<(), Error> {
        match self {
            CommitmentContext::None => Ok(()),
            CommitmentContext::WithinTrustingPeriod(ctx) => ctx.validate(current_timestamp),
        }
    }

    // MSB first
    // 0-1:  type
    // 2-31: reserved
    pub fn header(&self) -> [u8; COMMITMENT_CONTEXT_HEADER_SIZE] {
        let mut header = [0u8; COMMITMENT_CONTEXT_HEADER_SIZE];

        match self {
            CommitmentContext::None => {
                header[0..=1].copy_from_slice(&COMMITMENT_CONTEXT_TYPE_NONE.to_be_bytes());
            }
            CommitmentContext::WithinTrustingPeriod(_) => {
                header[0..=1]
                    .copy_from_slice(&COMMITMENT_CONTEXT_TYPE_WITHIN_TRUSTING_PERIOD.to_be_bytes());
            }
        }
        header
    }

    fn parse_context_type_from_header(header_bytes: &[u8]) -> Result<u16, Error> {
        if header_bytes.len() != COMMITMENT_CONTEXT_HEADER_SIZE {
            return Err(Error::invalid_commitment_context_header(format!(
                "invalid commitment context header length: expected={} actual={}",
                COMMITMENT_CONTEXT_HEADER_SIZE,
                header_bytes.len()
            )));
        }

        let mut header = [0u8; COMMITMENT_CONTEXT_HEADER_SIZE];
        header.copy_from_slice(header_bytes);

        Ok(u16::from_be_bytes([header[0], header[1]]))
    }
}

impl EthABIEncoder for CommitmentContext {
    fn ethabi_encode(self) -> Vec<u8> {
        let header = self.header().as_ref().try_into().unwrap();
        match self {
            CommitmentContext::None => EthABICommitmentContext {
                header,
                context_bytes: vec![],
            }
            .encode(),
            CommitmentContext::WithinTrustingPeriod(ctx) => EthABICommitmentContext {
                header,
                context_bytes: ctx.ethabi_encode(),
            }
            .encode(),
        }
    }
    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        let EthABICommitmentContext {
            header,
            context_bytes,
        } = EthABICommitmentContext::decode(bz)?;

        match CommitmentContext::parse_context_type_from_header(&header)? {
            COMMITMENT_CONTEXT_TYPE_NONE => {
                assert!(context_bytes.is_empty());
                Ok(CommitmentContext::None)
            }
            COMMITMENT_CONTEXT_TYPE_WITHIN_TRUSTING_PERIOD => {
                let ctx = WithinTrustingPeriodContext::ethabi_decode(&context_bytes)?;
                Ok(CommitmentContext::WithinTrustingPeriod(ctx))
            }
            type_ => Err(Error::invalid_commitment_context_header(format!(
                "unknown commitment context type: {}",
                type_
            ))),
        }
    }
}

pub(crate) struct EthABICommitmentContext {
    header: ethabi::FixedBytes,   // bytes32
    context_bytes: ethabi::Bytes, // bytes
}

impl EthABICommitmentContext {
    fn encode(&self) -> Vec<u8> {
        use ethabi::Token;
        ethabi::encode(&[Token::Tuple(vec![
            Token::FixedBytes(self.header.clone()),
            Token::Bytes(self.context_bytes.clone()),
        ])])
    }
    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        use ethabi::ParamType;
        let tuple = ethabi::decode(
            &[ParamType::Tuple(vec![
                ParamType::FixedBytes(32),
                ParamType::Bytes,
            ])],
            bytes,
        )?
        .into_iter()
        .next()
        .unwrap()
        .into_tuple()
        .unwrap();

        // if the decoding is successful, the length of the tuple should be 2
        assert!(tuple.len() == 2);
        let mut values = tuple.into_iter();
        Ok(Self {
            header: values.next().unwrap().into_fixed_bytes().unwrap(),
            context_bytes: values.next().unwrap().into_bytes().unwrap(),
        })
    }
}

impl Default for CommitmentContext {
    fn default() -> Self {
        CommitmentContext::None
    }
}

impl Display for CommitmentContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CommitmentContext::None => write!(f, "None"),
            CommitmentContext::WithinTrustingPeriod(ctx) => write!(f, "TrustingPeriod {{{}}}", ctx),
        }
    }
}

/// NOTE: time precision is in seconds (i.e. nanoseconds are truncated)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithinTrustingPeriodContext {
    /// How long a validator set is trusted for (must be shorter than the chain's
    /// unbonding period)
    trusting_period: Duration,

    /// Correction parameter dealing with only approximately synchronized clocks.
    /// The local clock should always be ahead of timestamps from the blockchain; this
    /// is the maximum amount that the local clock may drift behind a timestamp from the
    /// blockchain.
    clock_drift: Duration,

    /// The timestamp of the untrusted header
    /// NOTE: The header is used to update the state of the light client.
    untrusted_header_timestamp: Time,

    /// The timestamp of the trusted state
    /// NOTE: The state is a previously verified state of the light client.
    trusted_state_timestamp: Time,
}

impl WithinTrustingPeriodContext {
    pub fn new(
        trusting_period: Duration,
        clock_drift: Duration,
        untrusted_header_timestamp: Time,
        trusted_state_timestamp: Time,
    ) -> Result<Self, Error> {
        // truncate nanoseconds from each parameter
        let trusting_period = Duration::from_secs(trusting_period.as_secs());
        let clock_drift = Duration::from_secs(clock_drift.as_secs());
        let untrusted_header_timestamp =
            Time::from_unix_timestamp_secs(untrusted_header_timestamp.as_unix_timestamp_secs())?;
        let trusted_state_timestamp =
            Time::from_unix_timestamp_secs(trusted_state_timestamp.as_unix_timestamp_secs())?;
        Ok(Self {
            trusting_period,
            clock_drift,
            untrusted_header_timestamp,
            trusted_state_timestamp,
        })
    }

    pub fn validate(&self, current_timestamp: Time) -> Result<(), Error> {
        // truncate nanoseconds from current timestamp
        let current_timestamp =
            Time::from_unix_timestamp_secs(current_timestamp.as_unix_timestamp_secs())?;

        // ensure that trusted consensus state's timestamp hasn't passed the trusting period
        Self::ensure_within_trust_period(
            current_timestamp,
            self.trusted_state_timestamp,
            self.trusting_period,
        )?;

        // ensure the header isn't from a future time
        Self::ensure_header_from_past(
            self.untrusted_header_timestamp,
            self.clock_drift,
            current_timestamp,
        )?;

        Ok(())
    }

    fn ensure_within_trust_period(
        now: Time,
        trusted_state_time: Time,
        trusting_period: Duration,
    ) -> Result<(), Error> {
        let trusting_period_end = (trusted_state_time + trusting_period)?;
        if trusting_period_end > now {
            Ok(())
        } else {
            Err(Error::out_of_trusting_period(
                now.as_unix_timestamp_secs(),
                trusting_period_end.as_unix_timestamp_secs(),
            ))
        }
    }

    fn ensure_header_from_past(
        untrusted_header_time: Time,
        clock_drift: Duration,
        now: Time,
    ) -> Result<(), Error> {
        let current = (now + clock_drift)?;
        if current > untrusted_header_time {
            Ok(())
        } else {
            Err(Error::header_from_future(
                now.as_unix_timestamp_secs(),
                untrusted_header_time.as_unix_timestamp_secs(),
            ))
        }
    }
}

impl Display for WithinTrustingPeriodContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "trusting_period={} clock_drift={} untrusted_header_timestamp={} trusted_state_timestamp={}",
            self.trusting_period.as_secs(), self.clock_drift.as_secs(), self.untrusted_header_timestamp, self.trusted_state_timestamp
        )
    }
}

impl EthABIEncoder for WithinTrustingPeriodContext {
    fn ethabi_encode(self) -> Vec<u8> {
        let mut params = [0u8; 32];
        params[0..=7].copy_from_slice(&self.trusting_period.as_secs().to_be_bytes());
        params[8..=15].copy_from_slice(&self.clock_drift.as_secs().to_be_bytes());
        params[16..=23].copy_from_slice(
            &self
                .untrusted_header_timestamp
                .as_unix_timestamp_secs()
                .to_be_bytes(),
        );
        params[24..=31].copy_from_slice(
            &self
                .trusted_state_timestamp
                .as_unix_timestamp_secs()
                .to_be_bytes(),
        );
        EthABIWithinTrustingPeriodContext {
            params: params.to_vec(),
        }
        .encode()
    }
    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        let c = EthABIWithinTrustingPeriodContext::decode(bz)?;
        let params = c.params;
        let trusting_period =
            Duration::from_secs(u64::from_be_bytes(params[0..=7].try_into().unwrap()));
        let clock_drift =
            Duration::from_secs(u64::from_be_bytes(params[8..=15].try_into().unwrap()));
        let untrusted_header_timestamp = Time::from_unix_timestamp_secs(u64::from_be_bytes(
            params[16..=23].try_into().unwrap(),
        ))?;
        let trusted_state_timestamp = Time::from_unix_timestamp_secs(u64::from_be_bytes(
            params[24..=31].try_into().unwrap(),
        ))?;
        Ok(Self {
            trusting_period,
            clock_drift,
            untrusted_header_timestamp,
            trusted_state_timestamp,
        })
    }
}

impl From<WithinTrustingPeriodContext> for CommitmentContext {
    fn from(ctx: WithinTrustingPeriodContext) -> Self {
        CommitmentContext::WithinTrustingPeriod(ctx)
    }
}

pub(crate) struct EthABIWithinTrustingPeriodContext {
    // bytes32 in solidity
    // MSB first
    // 0-7: trusting_period
    // 8-15: clock_drift
    // 16-23: untrusted_header_timestamp
    // 24-31: trusted_state_timestamp
    pub params: ethabi::FixedBytes,
}

impl EthABIWithinTrustingPeriodContext {
    fn encode(self) -> Vec<u8> {
        use ethabi::Token;
        ethabi::encode(&[Token::FixedBytes(self.params)])
    }
    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        use ethabi::ParamType;
        let tuple = ethabi::decode(&[ParamType::FixedBytes(32)], bytes)?;
        assert!(tuple.len() == 1);
        let params = tuple
            .into_iter()
            .next()
            .unwrap()
            .into_fixed_bytes()
            .unwrap();
        Ok(Self { params })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lcp_types::MAX_UNIX_TIMESTAMP_SECS;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn pt_trusting_period_context(
            trusting_period in any::<u64>(),
            clock_drift in any::<u64>(),
            untrusted_header_timestamp in 0u64..=MAX_UNIX_TIMESTAMP_SECS,
            trusted_state_timestamp in 0u64..=MAX_UNIX_TIMESTAMP_SECS
        ) {
            let ctx: CommitmentContext = WithinTrustingPeriodContext::new(
                Duration::from_secs(trusting_period),
                Duration::from_secs(clock_drift),
                Time::from_unix_timestamp_secs(untrusted_header_timestamp).unwrap(),
                Time::from_unix_timestamp_secs(trusted_state_timestamp).unwrap(),
            ).unwrap().into();
            let bz = ctx.clone().ethabi_encode();
            let ctx2 = CommitmentContext::ethabi_decode(&bz).unwrap();
            assert_eq!(ctx, ctx2);
        }
    }

    #[test]
    fn test_none_context_serialization() {
        let ctx = CommitmentContext::None;
        let bz = ctx.clone().ethabi_encode();
        let ctx2 = CommitmentContext::ethabi_decode(&bz).unwrap();
        assert_eq!(ctx, ctx2);
    }

    #[test]
    fn test_trusting_period_context_serialization() {
        let ctx = CommitmentContext::WithinTrustingPeriod(
            WithinTrustingPeriodContext::new(
                Duration::new(60 * 60 * 24, 0),
                Duration::new(60 * 60, 0),
                Time::now(),
                Time::now(),
            )
            .unwrap(),
        );
        let bz = ctx.clone().ethabi_encode();
        let ctx2 = CommitmentContext::ethabi_decode(&bz).unwrap();
        assert_eq!(ctx, ctx2);
    }

    #[test]
    fn test_context_header() {
        let ctx = CommitmentContext::None;
        let header = ctx.header();
        assert_eq!(
            COMMITMENT_CONTEXT_TYPE_NONE,
            CommitmentContext::parse_context_type_from_header(&header).unwrap()
        );

        let ctx = CommitmentContext::WithinTrustingPeriod(
            WithinTrustingPeriodContext::new(
                Duration::new(60 * 60 * 24, 0),
                Duration::new(60 * 60, 0),
                Time::now(),
                Time::now(),
            )
            .unwrap(),
        );
        let header = ctx.header();
        assert_eq!(
            COMMITMENT_CONTEXT_TYPE_WITHIN_TRUSTING_PERIOD,
            CommitmentContext::parse_context_type_from_header(&header).unwrap()
        );
    }

    #[test]
    fn test_eth_abi_encoding() {
        use ethabi::Token;
        let v = ethabi::encode(&[Token::FixedBytes([0u8; 32].to_vec())]);
        assert_eq!(v.len(), 32);
    }
}