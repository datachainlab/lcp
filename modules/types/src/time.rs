use crate::errors::TimeError;
use crate::prelude::*;
use core::fmt::Display;
use core::ops::Deref;
use core::ops::{Add, Sub};
use core::time::Duration;
use ibc::timestamp::Timestamp;
use serde::{Deserialize, Serialize};
use tendermint::Time as TmTime;

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Time(TmTime);

impl Time {
    #[cfg(feature = "std")]
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        Time(TmTime::from_unix_timestamp(now.as_secs() as i64, now.subsec_nanos()).unwrap())
    }

    #[cfg(feature = "sgx")]
    pub fn now() -> Self {
        use sgx_tstd::time::{SystemTime, UNIX_EPOCH};
        use sgx_tstd::untrusted::time::SystemTimeEx;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        Time(TmTime::from_unix_timestamp(now.as_secs() as i64, now.subsec_nanos()).unwrap())
    }

    pub fn unix_epoch() -> Self {
        Time(TmTime::unix_epoch())
    }

    pub fn from_unix_timestamp_nanos(timestamp: u128) -> Result<Self, TimeError> {
        let ut = TmTime::from_unix_timestamp(
            (timestamp / 1_000_000_000) as i64,
            (timestamp % 1_000_000_000) as u32,
        )
        .map_err(TimeError::tendermint)?;
        Ok(Time(ut))
    }

    pub fn from_unix_timestamp_secs(timestamp: u64) -> Result<Self, TimeError> {
        let ut = TmTime::from_unix_timestamp(timestamp as i64, 0).map_err(TimeError::tendermint)?;
        Ok(Time(ut))
    }

    pub fn as_unix_timestamp_nanos(&self) -> u128 {
        self.0
            .duration_since(TmTime::unix_epoch())
            .unwrap()
            .as_nanos()
    }

    pub fn as_unix_timestamp_secs(&self) -> u64 {
        self.0
            .duration_since(TmTime::unix_epoch())
            .unwrap()
            .as_secs()
    }
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

impl From<Time> for Timestamp {
    fn from(value: Time) -> Self {
        value.0.into()
    }
}

impl From<Timestamp> for Time {
    fn from(value: Timestamp) -> Self {
        Self(value.into_tm_time().unwrap())
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
