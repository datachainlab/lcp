use crate::errors::TimeError;
use crate::prelude::*;
use core::fmt::Display;
use core::ops::Deref;
use core::ops::{Add, Sub};
use core::time::Duration;
use serde::{Deserialize, Serialize};
use tendermint::Time as TmTime;

// NOTE: This value is limited by "tendermint/time" crate
// i.e. 9999-12-31T23:59:59.999999999Z
pub const MAX_UNIX_TIMESTAMP_NANOS: u128 = 253_402_300_799_999_999_999;

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Time(TmTime);

impl Time {
    #[cfg(feature = "std")]
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        Time(TmTime::from_unix_timestamp(now.as_secs() as i64, now.subsec_nanos()).unwrap())
    }

    pub fn unix_epoch() -> Self {
        Time(TmTime::unix_epoch())
    }

    pub fn from_unix_timestamp_nanos(timestamp: u128) -> Result<Self, TimeError> {
        let d = nanos_to_duration(timestamp)?;
        let ut = TmTime::from_unix_timestamp(d.as_secs().try_into()?, d.subsec_nanos())
            .map_err(TimeError::tendermint)?;
        Ok(Time(ut))
    }

    pub fn as_unix_timestamp_secs(&self) -> u64 {
        self.0
            .duration_since(TmTime::unix_epoch())
            .unwrap()
            .as_secs()
    }

    pub fn as_unix_timestamp_nanos(&self) -> u128 {
        self.0
            .duration_since(TmTime::unix_epoch())
            .unwrap()
            .as_nanos()
    }
}

pub fn nanos_to_duration(nanos: u128) -> Result<Duration, TimeError> {
    let secs = (nanos / 1_000_000_000).try_into()?;
    let nanos = (nanos % 1_000_000_000) as u32;
    Ok(Duration::new(secs, nanos))
}

impl Deref for Time {
    type Target = TmTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<TmTime> for Time {
    fn from(value: TmTime) -> Self {
        Self(value)
    }
}

impl Add<Duration> for Time {
    type Output = Result<Self, TimeError>;

    fn add(self, rhs: Duration) -> Self::Output {
        Ok(Self((*self + rhs).map_err(TimeError::tendermint)?))
    }
}

impl Sub<Duration> for Time {
    type Output = Result<Self, TimeError>;

    fn sub(self, rhs: Duration) -> Self::Output {
        Ok(Self((*self - rhs).map_err(TimeError::tendermint)?))
    }
}

#[cfg(feature = "ibc")]
impl From<Time> for ibc::timestamp::Timestamp {
    fn from(value: Time) -> Self {
        value.0.into()
    }
}

#[cfg(feature = "ibc")]
impl From<ibc::timestamp::Timestamp> for Time {
    fn from(value: ibc::timestamp::Timestamp) -> Self {
        Self(value.into_tm_time().unwrap())
    }
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
            assert_eq!(time.unwrap().as_unix_timestamp_nanos(), timestamp);
        }

        #[test]
        fn test_time_from_unix_timestamp_nanos_overflow(timestamp in MAX_UNIX_TIMESTAMP_NANOS + 1..) {
            assert!(Time::from_unix_timestamp_nanos(timestamp).is_err());
            assert!(nanos_to_duration(timestamp).is_err());
        }
    }

    #[test]
    fn test_max_time() {
        assert!(Time::from_unix_timestamp_nanos(MAX_UNIX_TIMESTAMP_NANOS).is_ok());
        assert!(Time::from_unix_timestamp_nanos(MAX_UNIX_TIMESTAMP_NANOS + 1).is_err());
        let max_time = Time::from_unix_timestamp_nanos(MAX_UNIX_TIMESTAMP_NANOS).unwrap();
        assert_eq!(max_time.as_unix_timestamp_nanos(), MAX_UNIX_TIMESTAMP_NANOS);
        assert!((max_time + Duration::new(0, 1)).is_err());
    }
}
