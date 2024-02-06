use crate::prelude::*;
use crate::{Error, EthABIEncoder};
use alloy_sol_types::{sol, SolValue};
use core::{fmt::Display, time::Duration};
use lcp_types::{nanos_to_duration, Time};
use serde::{Deserialize, Serialize};

pub const VALIDATION_CONTEXT_TYPE_EMPTY_EMPTY: u16 = 0;
pub const VALIDATION_CONTEXT_TYPE_EMPTY_WITHIN_TRUSTING_PERIOD: u16 = 1;
pub const VALIDATION_CONTEXT_HEADER_SIZE: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationContext {
    Empty,
    TrustingPeriod(TrustingPeriodContext),
}

impl ValidationContext {
    pub fn validate(&self, current_timestamp: Time) -> Result<(), Error> {
        match self {
            ValidationContext::Empty => Ok(()),
            ValidationContext::TrustingPeriod(ctx) => ctx.validate(current_timestamp),
        }
    }

    // MSB first
    // 0-1:  type
    // 2-31: reserved
    pub fn header(&self) -> [u8; VALIDATION_CONTEXT_HEADER_SIZE] {
        let mut header = [0u8; VALIDATION_CONTEXT_HEADER_SIZE];

        match self {
            ValidationContext::Empty => {
                header[0..=1].copy_from_slice(&VALIDATION_CONTEXT_TYPE_EMPTY_EMPTY.to_be_bytes());
            }
            ValidationContext::TrustingPeriod(_) => {
                header[0..=1].copy_from_slice(
                    &VALIDATION_CONTEXT_TYPE_EMPTY_WITHIN_TRUSTING_PERIOD.to_be_bytes(),
                );
            }
        }
        header
    }

    pub fn aggregate(self, other: Self) -> Result<Self, Error> {
        match (self, other) {
            (Self::Empty, Self::Empty) => Ok(Self::Empty),
            (Self::Empty, Self::TrustingPeriod(ctx)) => Ok(Self::TrustingPeriod(ctx)),
            (Self::TrustingPeriod(ctx), Self::Empty) => Ok(Self::TrustingPeriod(ctx)),
            (Self::TrustingPeriod(ctx1), Self::TrustingPeriod(ctx2)) => {
                Ok(Self::TrustingPeriod(ctx1.aggregate(ctx2)?))
            }
        }
    }

    fn parse_context_type_from_header(header_bytes: &[u8]) -> Result<u16, Error> {
        if header_bytes.len() != VALIDATION_CONTEXT_HEADER_SIZE {
            return Err(Error::invalid_validation_context_header(format!(
                "invalid validation context header length: expected={} actual={}",
                VALIDATION_CONTEXT_HEADER_SIZE,
                header_bytes.len()
            )));
        }

        let mut header = [0u8; VALIDATION_CONTEXT_HEADER_SIZE];
        header.copy_from_slice(header_bytes);

        Ok(u16::from_be_bytes([header[0], header[1]]))
    }
}

impl EthABIEncoder for ValidationContext {
    fn ethabi_encode(self) -> Vec<u8> {
        let header = self.header().as_ref().try_into().unwrap();
        match self {
            ValidationContext::Empty => EthABIValidationContext {
                header,
                context_bytes: vec![],
            }
            .abi_encode(),
            ValidationContext::TrustingPeriod(ctx) => EthABIValidationContext {
                header,
                context_bytes: ctx.ethabi_encode(),
            }
            .abi_encode(),
        }
    }
    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        let EthABIValidationContext {
            header,
            context_bytes,
        } = EthABIValidationContext::abi_decode(bz, true)?;

        match ValidationContext::parse_context_type_from_header(header.as_slice())? {
            VALIDATION_CONTEXT_TYPE_EMPTY_EMPTY => {
                assert!(context_bytes.is_empty());
                Ok(ValidationContext::Empty)
            }
            VALIDATION_CONTEXT_TYPE_EMPTY_WITHIN_TRUSTING_PERIOD => {
                let ctx = TrustingPeriodContext::ethabi_decode(&context_bytes)?;
                Ok(ValidationContext::TrustingPeriod(ctx))
            }
            type_ => Err(Error::invalid_validation_context_header(format!(
                "unknown validation context type: {}",
                type_
            ))),
        }
    }
}

sol! {
    struct EthABIValidationContext {
        bytes32 header;
        bytes context_bytes;
    }
}

impl Default for ValidationContext {
    fn default() -> Self {
        ValidationContext::Empty
    }
}

impl Display for ValidationContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ValidationContext::Empty => write!(f, "Empty"),
            ValidationContext::TrustingPeriod(ctx) => write!(f, "TrustingPeriod {{{}}}", ctx),
        }
    }
}

/// NOTE: time precision is in seconds (i.e. nanoseconds are truncated)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustingPeriodContext {
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

impl TrustingPeriodContext {
    pub fn new(
        trusting_period: Duration,
        clock_drift: Duration,
        untrusted_header_timestamp: Time,
        trusted_state_timestamp: Time,
    ) -> Self {
        Self {
            trusting_period,
            clock_drift,
            untrusted_header_timestamp,
            trusted_state_timestamp,
        }
    }

    pub fn validate(&self, current_timestamp: Time) -> Result<(), Error> {
        // ensure that trusted consensus state's timestamp hasn't passed the trusting period
        Self::ensure_within_trust_period(
            current_timestamp,
            self.trusted_state_timestamp,
            self.trusting_period,
        )?;

        // ensure the header isn't from a future time
        Self::ensure_header_from_past(
            current_timestamp,
            self.untrusted_header_timestamp,
            self.clock_drift,
        )?;

        Ok(())
    }

    pub fn aggregate(self, other: Self) -> Result<Self, Error> {
        if self.trusting_period != other.trusting_period {
            return Err(Error::context_aggregation_failed(format!(
                "trusting_period mismatch: self={:?} other={:?}",
                self.trusting_period, other.trusting_period,
            )));
        }
        if self.clock_drift != other.clock_drift {
            return Err(Error::context_aggregation_failed(format!(
                "clock_drift mismatch: self={:?} other={:?}",
                self.clock_drift, other.clock_drift
            )));
        }
        Ok(Self {
            trusting_period: self.trusting_period,
            clock_drift: self.clock_drift,
            untrusted_header_timestamp: if self.untrusted_header_timestamp
                > other.untrusted_header_timestamp
            {
                self.untrusted_header_timestamp
            } else {
                other.untrusted_header_timestamp
            },
            trusted_state_timestamp: if self.trusted_state_timestamp < other.trusted_state_timestamp
            {
                self.trusted_state_timestamp
            } else {
                other.trusted_state_timestamp
            },
        })
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
            Err(Error::out_of_trusting_period(now, trusting_period_end))
        }
    }

    fn ensure_header_from_past(
        now: Time,
        untrusted_header_time: Time,
        clock_drift: Duration,
    ) -> Result<(), Error> {
        let current = (now + clock_drift)?;
        if current > untrusted_header_time {
            Ok(())
        } else {
            Err(Error::header_from_future(now, untrusted_header_time))
        }
    }
}

impl Display for TrustingPeriodContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "trusting_period={} clock_drift={} untrusted_header_timestamp={} trusted_state_timestamp={}",
            self.trusting_period.as_nanos(), self.clock_drift.as_nanos(), self.untrusted_header_timestamp.as_unix_timestamp_nanos(), self.trusted_state_timestamp.as_unix_timestamp_nanos()
        )
    }
}

impl EthABIEncoder for TrustingPeriodContext {
    fn ethabi_encode(self) -> Vec<u8> {
        let mut timestamps = [0u8; 32];
        timestamps[0..=15].copy_from_slice(
            &self
                .untrusted_header_timestamp
                .as_unix_timestamp_nanos()
                .to_be_bytes(),
        );
        timestamps[16..=31].copy_from_slice(
            &self
                .trusted_state_timestamp
                .as_unix_timestamp_nanos()
                .to_be_bytes(),
        );
        let mut params = [0u8; 32];
        params[0..=15].copy_from_slice(&self.trusting_period.as_nanos().to_be_bytes());
        params[16..=31].copy_from_slice(&self.clock_drift.as_nanos().to_be_bytes());
        EthABITrustingPeriodContext {
            timestamps: timestamps.into(),
            params: params.into(),
        }
        .abi_encode()
    }
    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        let c = EthABITrustingPeriodContext::abi_decode(bz, true)?;
        let trusting_period =
            nanos_to_duration(u128::from_be_bytes(c.params[0..=15].try_into().unwrap()))?;
        let clock_drift =
            nanos_to_duration(u128::from_be_bytes(c.params[16..=31].try_into().unwrap()))?;
        let untrusted_header_timestamp = Time::from_unix_timestamp_nanos(u128::from_be_bytes(
            c.timestamps[0..=15].try_into().unwrap(),
        ))?;
        let trusted_state_timestamp = Time::from_unix_timestamp_nanos(u128::from_be_bytes(
            c.timestamps[16..=31].try_into().unwrap(),
        ))?;
        Ok(Self {
            trusting_period,
            clock_drift,
            untrusted_header_timestamp,
            trusted_state_timestamp,
        })
    }
}

impl From<TrustingPeriodContext> for ValidationContext {
    fn from(ctx: TrustingPeriodContext) -> Self {
        ValidationContext::TrustingPeriod(ctx)
    }
}

sol! {
    struct EthABITrustingPeriodContext {
        /// MSB first
        /// 0-15: untrusted_header_timestamp
        /// 16-31: trusted_state_timestamp
        bytes32 timestamps;
        /// 0-15: trusting_period
        /// 16-31: clock_drift
        bytes32 params;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorDetail;
    use lcp_types::MAX_UNIX_TIMESTAMP_NANOS;
    use proptest::prelude::*;
    use time::{macros::datetime, OffsetDateTime};

    proptest! {
        #[test]
        fn pt_trusting_period_context(
            trusting_period in ..=MAX_UNIX_TIMESTAMP_NANOS,
            clock_drift in ..=MAX_UNIX_TIMESTAMP_NANOS,
            untrusted_header_timestamp in ..=MAX_UNIX_TIMESTAMP_NANOS,
            trusted_state_timestamp in ..=MAX_UNIX_TIMESTAMP_NANOS
        ) {
            let ctx: ValidationContext = TrustingPeriodContext::new(
                nanos_to_duration(trusting_period).unwrap(),
                nanos_to_duration(clock_drift).unwrap(),
                Time::from_unix_timestamp_nanos(untrusted_header_timestamp).unwrap(),
                Time::from_unix_timestamp_nanos(trusted_state_timestamp).unwrap(),
            ).into();
            let bz = ctx.clone().ethabi_encode();
            let ctx2 = ValidationContext::ethabi_decode(&bz).unwrap();
            assert_eq!(ctx, ctx2);
        }
    }

    #[test]
    fn test_empty_context_serialization() {
        let ctx = ValidationContext::Empty;
        let bz = ctx.clone().ethabi_encode();
        let ctx2 = ValidationContext::ethabi_decode(&bz).unwrap();
        assert_eq!(ctx, ctx2);
    }

    #[test]
    fn test_trusting_period_context_serialization() {
        let ctx = ValidationContext::TrustingPeriod(TrustingPeriodContext::new(
            Duration::new(60 * 60 * 24, 0),
            Duration::new(60 * 60, 0),
            Time::now(),
            Time::now(),
        ));
        let bz = ctx.clone().ethabi_encode();
        let ctx2 = ValidationContext::ethabi_decode(&bz).unwrap();
        assert_eq!(ctx, ctx2);
    }

    #[test]
    fn test_context_header() {
        let ctx = ValidationContext::Empty;
        let header = ctx.header();
        assert_eq!(
            VALIDATION_CONTEXT_TYPE_EMPTY_EMPTY,
            ValidationContext::parse_context_type_from_header(&header).unwrap()
        );

        let ctx = ValidationContext::TrustingPeriod(TrustingPeriodContext::new(
            Duration::new(60 * 60 * 24, 0),
            Duration::new(60 * 60, 0),
            Time::now(),
            Time::now(),
        ));
        let header = ctx.header();
        assert_eq!(
            VALIDATION_CONTEXT_TYPE_EMPTY_WITHIN_TRUSTING_PERIOD,
            ValidationContext::parse_context_type_from_header(&header).unwrap()
        );
    }

    fn build_trusting_period_context(
        trusting_period_nanos: u128,
        clock_drift_nanos: u128,
        untrusted_header_timestamp: OffsetDateTime,
        trusted_state_timestamp: OffsetDateTime,
    ) -> TrustingPeriodContext {
        TrustingPeriodContext::new(
            nanos_to_duration(trusting_period_nanos).unwrap(),
            nanos_to_duration(clock_drift_nanos).unwrap(),
            Time::from_unix_timestamp_nanos(
                untrusted_header_timestamp.unix_timestamp_nanos() as u128
            )
            .unwrap(),
            Time::from_unix_timestamp_nanos(trusted_state_timestamp.unix_timestamp_nanos() as u128)
                .unwrap(),
        )
    }

    fn validate_and_assert_no_error(ctx: TrustingPeriodContext, current_timestamp: OffsetDateTime) {
        let res = ctx.validate(
            Time::from_unix_timestamp_nanos(current_timestamp.unix_timestamp_nanos() as u128)
                .unwrap(),
        );
        assert!(res.is_ok(), "{:?}", res);
    }

    fn validate_and_assert_trusting_period_error(
        ctx: TrustingPeriodContext,
        current_timestamp: OffsetDateTime,
    ) {
        let res = ctx.validate(
            Time::from_unix_timestamp_nanos(current_timestamp.unix_timestamp_nanos() as u128)
                .unwrap(),
        );
        assert!(res.is_err());
        if let ErrorDetail::OutOfTrustingPeriod(_) = res.as_ref().err().unwrap().detail() {
        } else {
            panic!("{:?}", res);
        }
    }

    fn validate_and_assert_clock_drift_error(
        ctx: TrustingPeriodContext,
        current_timestamp: OffsetDateTime,
    ) {
        let res = ctx.validate(
            Time::from_unix_timestamp_nanos(current_timestamp.unix_timestamp_nanos() as u128)
                .unwrap(),
        );
        assert!(res.is_err());
        if let ErrorDetail::HeaderFromFuture(_) = res.as_ref().err().unwrap().detail() {
        } else {
            panic!("{:?}", res);
        }
    }

    #[test]
    fn test_trusting_period_context() {
        {
            let current_timestamp = datetime!(2023-08-20 0:00 UTC);
            let untrusted_header_timestamp = datetime!(2023-08-20 0:00 UTC);
            let trusted_state_timestamp = datetime!(2023-08-20 0:00 UTC);
            let ctx = build_trusting_period_context(
                1,
                1,
                untrusted_header_timestamp,
                trusted_state_timestamp,
            );
            validate_and_assert_no_error(ctx, current_timestamp);
        }

        // trusting_period
        {
            let current_timestamp = datetime!(2023-08-20 0:00 UTC);
            let untrusted_header_timestamp = current_timestamp - Duration::new(0, 1);
            let trusted_state_timestamp = untrusted_header_timestamp - Duration::new(0, 1);

            let ctx = build_trusting_period_context(
                1,
                0,
                untrusted_header_timestamp,
                trusted_state_timestamp,
            );
            validate_and_assert_trusting_period_error(ctx, current_timestamp);

            let ctx = build_trusting_period_context(
                2,
                0,
                untrusted_header_timestamp,
                trusted_state_timestamp,
            );
            validate_and_assert_trusting_period_error(ctx, current_timestamp);

            let ctx = build_trusting_period_context(
                3,
                0,
                untrusted_header_timestamp,
                trusted_state_timestamp,
            );
            validate_and_assert_no_error(ctx, current_timestamp);
        }

        // clock drift
        {
            let current_timestamp = datetime!(2023-08-20 0:00 UTC);
            let untrusted_header_timestamp = current_timestamp + Duration::new(0, 1);
            let trusted_state_timestamp = current_timestamp;
            let ctx = build_trusting_period_context(
                1,
                0,
                untrusted_header_timestamp,
                trusted_state_timestamp,
            );
            validate_and_assert_clock_drift_error(ctx, current_timestamp);
            let ctx = build_trusting_period_context(
                1,
                1,
                untrusted_header_timestamp,
                trusted_state_timestamp,
            );
            validate_and_assert_clock_drift_error(ctx, current_timestamp);
            let ctx = build_trusting_period_context(
                1,
                2,
                untrusted_header_timestamp,
                trusted_state_timestamp,
            );
            validate_and_assert_no_error(ctx, current_timestamp);
        }
    }

    #[test]
    fn test_validation_context_aggregation() {
        {
            let ctx0 = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-19 0:00 UTC),
                datetime!(2023-08-19 0:00 UTC),
            ));
            let ctx1 = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-20 0:00 UTC),
            ));
            let expected = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-19 0:00 UTC),
            ));
            let res = ctx0.aggregate(ctx1);
            if let Ok(ctx) = res {
                assert_eq!(ctx, expected);
            } else {
                panic!("{:?}", res);
            }
        }

        {
            let ctx0 = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-20 0:00 UTC),
            ));
            let ctx1 = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-19 0:00 UTC),
                datetime!(2023-08-19 0:00 UTC),
            ));
            let expected = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-19 0:00 UTC),
            ));
            let res = ctx0.aggregate(ctx1);
            if let Ok(ctx) = res {
                assert_eq!(ctx, expected);
            } else {
                panic!("{:?}", res);
            }
        }

        {
            let ctx0 = ValidationContext::from(build_trusting_period_context(
                2,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-20 0:00 UTC),
            ));
            let ctx1 = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-20 0:00 UTC),
            ));
            let res = ctx0.aggregate(ctx1);
            assert!(res.is_err());
        }

        {
            let ctx0 = ValidationContext::from(build_trusting_period_context(
                1,
                2,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-20 0:00 UTC),
            ));
            let ctx1 = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-20 0:00 UTC),
            ));
            let res = ctx0.aggregate(ctx1);
            assert!(res.is_err());
        }
    }

    #[test]
    fn test_validation_context_and_empty_aggregation() {
        {
            let ctx0 = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-20 0:00 UTC),
            ));
            let ctx1 = ValidationContext::Empty;
            let expected = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-20 0:00 UTC),
            ));
            let res = ctx0.aggregate(ctx1);
            if let Ok(ctx) = res {
                assert_eq!(ctx, expected);
            } else {
                panic!("{:?}", res);
            }
        }

        {
            let ctx0 = ValidationContext::Empty;
            let ctx1 = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-20 0:00 UTC),
            ));
            let expected = ValidationContext::from(build_trusting_period_context(
                1,
                1,
                datetime!(2023-08-20 0:00 UTC),
                datetime!(2023-08-20 0:00 UTC),
            ));
            let res = ctx0.aggregate(ctx1);
            if let Ok(ctx) = res {
                assert_eq!(ctx, expected);
            } else {
                panic!("{:?}", res);
            }
        }
    }

    #[test]
    fn test_empty_context_aggregation() {
        let ctx0 = ValidationContext::Empty;
        let ctx1 = ValidationContext::Empty;
        let res = ctx0.aggregate(ctx1);
        if let Ok(ctx) = res {
            assert_eq!(ctx, ValidationContext::Empty);
        } else {
            panic!("{:?}", res);
        }
    }
}
