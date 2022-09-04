use crate::prelude::*;
use core::ops::Deref;
use core::ops::{Add, Sub};
use core::time::Duration;
use ibc::timestamp::Timestamp;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(feature = "sgx")]
use std::untrusted::time::SystemTimeEx;
use tendermint::Error;
use tendermint::Time as TmTime;

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Time(TmTime);

impl Time {
    pub fn now() -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        Time(TmTime::from_unix_timestamp(now.as_secs() as i64, now.subsec_nanos()).unwrap())
    }

    pub fn from_unix_timestamp_nanos(timestamp: u128) -> Result<Self, Error> {
        let ut = TmTime::from_unix_timestamp(
            (timestamp / 1_000_000_000) as i64,
            (timestamp % 1_000_000_000) as u32,
        )?;
        Ok(Time(ut))
    }

    pub fn from_unix_timestamp_secs(timestamp: u64) -> Result<Self, Error> {
        let ut = TmTime::from_unix_timestamp(timestamp as i64, 0)?;
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
    type Output = Result<Self, Error>;

    fn add(self, rhs: Duration) -> Self::Output {
        Ok(Self((*self + rhs)?))
    }
}

impl Sub<Duration> for Time {
    type Output = Result<Self, Error>;

    fn sub(self, rhs: Duration) -> Self::Output {
        Ok(Self((*self - rhs)?))
    }
}
