use crate::errors::TimeError;
use crate::prelude::*;
use core::fmt::Display;
use core::ops::{Add, Sub};
use core::time::Duration;
use lcp_proto::google::protobuf::Timestamp;
use lcp_proto::protobuf::Protobuf;
use serde::{Deserialize, Serialize};
use time::macros::{datetime, offset};
use time::{OffsetDateTime, PrimitiveDateTime};

/// The maximum Unix timestamp in nanoseconds that can be represented in `Time`.
/// i.e., 9999-12-31T23:59:59.999999999Z
pub const MAX_UNIX_TIMESTAMP_NANOS: u128 = 253_402_300_799_999_999_999;

/// A newtype wrapper over `PrimitiveDateTime` which serves as the foundational
/// basis for capturing timestamps. It is used directly to keep track of host
/// timestamps.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "Timestamp", into = "Timestamp")]
pub struct Time(PrimitiveDateTime);

impl Protobuf<Timestamp> for Time {}

impl TryFrom<Timestamp> for Time {
    type Error = TimeError;

    fn try_from(value: Timestamp) -> Result<Self, TimeError> {
        let nanos = value.nanos.try_into()?;
        Self::from_unix_timestamp(value.seconds, nanos)
    }
}

impl From<Time> for Timestamp {
    fn from(value: Time) -> Self {
        let t = value.0.assume_utc();
        let seconds = t.unix_timestamp();
        // Safe to convert to i32 because .nanosecond()
        // is guaranteed to return a value in 0..1_000_000_000 range.
        let nanos = t.nanosecond() as i32;
        Timestamp { seconds, nanos }
    }
}

impl Time {
    /// Returns the current time as a `Time`.
    #[cfg(feature = "std")]
    pub fn now() -> Self {
        OffsetDateTime::now_utc()
            .try_into()
            .expect("now is in the range of 0..=9999 years")
    }

    /// Returns the Unix epoch as a `Time`.
    pub fn unix_epoch() -> Self {
        Self(datetime!(1970-01-01 00:00:00))
    }

    /// Constructs a `Time` from a Unix timestamp in nanoseconds.
    pub fn from_unix_timestamp_nanos(nanoseconds: u128) -> Result<Self, TimeError> {
        if nanoseconds > MAX_UNIX_TIMESTAMP_NANOS {
            return Err(TimeError::invalid_date());
        }
        // Safe to convert to i128 because nanoseconds is guaranteed to be in range 0..=MAX_UNIX_TIMESTAMP_NANOS.
        let odt = OffsetDateTime::from_unix_timestamp_nanos(nanoseconds as i128)
            .map_err(TimeError::component_range)?;
        Self::from_utc(odt)
    }

    /// Constructs a `Time` from a Unix timestamp in seconds and nanoseconds.
    pub fn from_unix_timestamp(secs: i64, nanos: u32) -> Result<Self, TimeError> {
        if secs < 0 {
            return Err(TimeError::invalid_date());
        }
        if nanos > 999_999_999 {
            return Err(TimeError::invalid_date());
        }
        let total_nanos = secs as u128 * 1_000_000_000 + nanos as u128;
        Self::from_unix_timestamp_nanos(total_nanos)
    }

    /// Internal helper to produce a `Timestamp` value validated with regard to
    /// the date range allowed in protobuf timestamps. The source
    /// `OffsetDateTime` value must have the zero UTC offset.
    fn from_utc(t: OffsetDateTime) -> Result<Self, TimeError> {
        debug_assert_eq!(t.offset(), offset!(UTC));
        match t.year() {
            1970..=9999 => Ok(Self(PrimitiveDateTime::new(t.date(), t.time()))),
            _ => Err(TimeError::invalid_date()),
        }
    }

    /// Computes the duration difference of another `Timestamp` from the current
    /// one. Returns the difference in time as an [`core::time::Duration`].
    pub fn duration_since(&self, other: &Self) -> Option<Duration> {
        let duration = self.0.assume_utc() - other.0.assume_utc();
        duration.try_into().ok()
    }

    /// Returns the `Timestamp` as a Unix timestamp in seconds.
    pub fn as_unix_timestamp_secs(&self) -> u64 {
        self.0.assume_utc().unix_timestamp().try_into().unwrap()
    }

    /// Return a Unix timestamp in nanoseconds.
    pub fn as_unix_timestamp_nanos(&self) -> u128 {
        self.0
            .assume_utc()
            .unix_timestamp_nanos()
            .try_into()
            .unwrap()
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<OffsetDateTime> for Time {
    type Error = TimeError;

    fn try_from(t: OffsetDateTime) -> Result<Time, TimeError> {
        Self::from_utc(t.to_offset(offset!(UTC)))
    }
}

impl Add<Duration> for Time {
    type Output = Result<Self, TimeError>;

    fn add(self, rhs: Duration) -> Self::Output {
        let duration = rhs
            .try_into()
            .map_err(|_| TimeError::duration_out_of_range())?;
        let t = self
            .0
            .checked_add(duration)
            .ok_or_else(TimeError::duration_out_of_range)?;
        Self::from_utc(t.assume_utc())
    }
}

impl Sub<Duration> for Time {
    type Output = Result<Self, TimeError>;

    fn sub(self, rhs: Duration) -> Self::Output {
        let duration = rhs
            .try_into()
            .map_err(|_| TimeError::duration_out_of_range())?;
        let t = self
            .0
            .checked_sub(duration)
            .ok_or_else(TimeError::duration_out_of_range)?;
        Self::from_utc(t.assume_utc())
    }
}

#[cfg(feature = "ibc")]
impl From<Time> for ibc::timestamp::Timestamp {
    fn from(value: Time) -> Self {
        // Safe to convert to u64 because as_unix_timestamp_nanos() returns a value in 0..=MAX_UNIX_TIMESTAMP_NANOS.
        ibc::timestamp::Timestamp::from_nanoseconds(
            value.as_unix_timestamp_nanos().try_into().unwrap(),
        )
        .unwrap()
    }
}

#[cfg(feature = "ibc")]
impl TryFrom<ibc::timestamp::Timestamp> for Time {
    type Error = TimeError;

    fn try_from(value: ibc::timestamp::Timestamp) -> Result<Self, Self::Error> {
        match value.into_datetime() {
            Some(datetime) => Ok(datetime.try_into()?),
            None => Err(TimeError::invalid_date()),
        }
    }
}

/// Converts a duration in nanoseconds to a `core::time::Duration`.
pub fn nanos_to_duration(nanos: u128) -> Result<Duration, TimeError> {
    let secs = (nanos / 1_000_000_000).try_into()?;
    let nanos = (nanos % 1_000_000_000) as u32;
    Ok(Duration::new(secs, nanos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_time_from_unix_timestamp_nanos(timestamp in ..=MAX_UNIX_TIMESTAMP_NANOS) {
            let time = Time::from_unix_timestamp_nanos(timestamp);
            assert!(time.is_ok());
            let time = time.unwrap();
            assert_eq!(time.as_unix_timestamp_nanos(), timestamp);
            assert_eq!(time.as_unix_timestamp_secs(), (timestamp / 1_000_000_000) as u64);
        }

        #[test]
        fn test_time_from_unix_timestamp_nanos_overflow(timestamp in MAX_UNIX_TIMESTAMP_NANOS + 1..) {
            assert!(Time::from_unix_timestamp_nanos(timestamp).is_err());
            assert!(nanos_to_duration(timestamp).is_err());
        }
    }

    #[test]
    fn test_time_range() {
        assert!(Time::from_unix_timestamp_nanos(MAX_UNIX_TIMESTAMP_NANOS).is_ok());
        assert!(Time::from_unix_timestamp_nanos(MAX_UNIX_TIMESTAMP_NANOS + 1).is_err());
        let max_time = Time::from_unix_timestamp_nanos(MAX_UNIX_TIMESTAMP_NANOS).unwrap();
        assert_eq!(max_time.as_unix_timestamp_nanos(), MAX_UNIX_TIMESTAMP_NANOS);
        assert!((max_time + Duration::new(0, 1)).is_err());

        assert!(Time::from_unix_timestamp(0, 999_999_999).is_ok());
        assert!(Time::from_unix_timestamp(i64::MAX, 0).is_err());
        assert!(Time::from_unix_timestamp(0, 1_000_000_000).is_err());

        assert!(Time::from_unix_timestamp(0, 0).is_ok());
        assert!(Time::from_unix_timestamp(-1, 0).is_err());
        assert!(Time::from_unix_timestamp(i64::MIN, 0).is_err());
    }
}
