use crate::errors::TypeError;
use crate::prelude::*;
use core::cmp::Ordering;
use core::str::FromStr;
use lcp_proto::ibc::core::client::v1::Height as RawHeight;
use lcp_proto::protobuf::Protobuf;
use serde::{Deserialize, Serialize};

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Height {
    /// Previously known as "epoch"
    revision_number: u64,

    /// The height of a block
    revision_height: u64,
}

impl Height {
    pub fn new(revision_number: u64, revision_height: u64) -> Self {
        Self {
            revision_number,
            revision_height,
        }
    }

    pub fn zero() -> Self {
        Height::new(0, 0)
    }

    pub fn revision_number(&self) -> u64 {
        self.revision_number
    }

    pub fn revision_height(&self) -> u64 {
        self.revision_height
    }

    pub fn add(&self, delta: u64) -> Result<Height, TypeError> {
        Ok(Height {
            revision_number: self.revision_number,
            revision_height: self
                .revision_height
                .checked_add(delta)
                .ok_or_else(TypeError::invalid_height_result)?,
        })
    }

    pub fn sub(&self, delta: u64) -> Result<Height, TypeError> {
        Ok(Height {
            revision_number: self.revision_number,
            revision_height: self
                .revision_height
                .checked_sub(delta)
                .ok_or_else(TypeError::invalid_height_result)?,
        })
    }

    pub fn is_zero(&self) -> bool {
        self.revision_number == 0 && self.revision_height == 0
    }
}

impl PartialOrd for Height {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Height {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.revision_number < other.revision_number {
            Ordering::Less
        } else if self.revision_number > other.revision_number {
            Ordering::Greater
        } else if self.revision_height < other.revision_height {
            Ordering::Less
        } else if self.revision_height > other.revision_height {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

impl Protobuf<RawHeight> for Height {}

impl From<RawHeight> for Height {
    fn from(raw_height: RawHeight) -> Self {
        Height::new(raw_height.revision_number, raw_height.revision_height)
    }
}

impl From<Height> for RawHeight {
    fn from(ics_height: Height) -> Self {
        RawHeight {
            revision_number: ics_height.revision_number,
            revision_height: ics_height.revision_height,
        }
    }
}

impl core::fmt::Debug for Height {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        f.debug_struct("Height")
            .field("revision", &self.revision_number)
            .field("height", &self.revision_height)
            .finish()
    }
}

/// Custom debug output to omit the packet data
impl core::fmt::Display for Height {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(f, "{}-{}", self.revision_number, self.revision_height)
    }
}

impl TryFrom<&str> for Height {
    type Error = TypeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let split: Vec<&str> = value.split('-').collect();

        let revision_number = split[0]
            .parse::<u64>()
            .map_err(|_| TypeError::height_conversion(value.to_owned()))?;
        let revision_height = split[1]
            .parse::<u64>()
            .map_err(|_| TypeError::height_conversion(value.to_owned()))?;

        Ok(Height::new(revision_number, revision_height))
    }
}

impl FromStr for Height {
    type Err = TypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Height::try_from(s)
    }
}

#[cfg(any(test, feature = "ibc"))]
impl TryFrom<Height> for ibc::core::ics02_client::height::Height {
    type Error = ibc::core::ics02_client::error::ClientError;

    fn try_from(height: Height) -> Result<Self, Self::Error> {
        if height.revision_height() == 0 {
            // XXX this is a trick for height type conversion due to ics02 height doesn't allow "zero" height
            Ok(serde_json::from_slice(&serde_json::to_vec(&height).unwrap()).unwrap())
        } else {
            Self::new(height.revision_number(), height.revision_height())
        }
    }
}

#[cfg(any(test, feature = "ibc"))]
impl From<ibc::core::ics02_client::height::Height> for Height {
    fn from(height: ibc::core::ics02_client::height::Height) -> Self {
        Height::new(height.revision_number(), height.revision_height())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ibc::core::ics02_client::height::Height as ICS02Height;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_height_str_conversion(rev_num: u64, rev_height: u64) {
            let h = Height::new(rev_num, rev_height);
            let h2 = Height::try_from(h.to_string().as_str());
            assert!(h2.is_ok());
            assert_eq!(h, h2.unwrap());
        }
    }

    #[test]
    fn test_height() {
        let h = Height::zero();
        assert!(h.is_zero());
        assert!(h.sub(1).is_err());
        let res: Result<ICS02Height, _> = h.try_into();
        assert!(res.is_ok());
        let ibc_height = res.unwrap();
        assert_eq!(ibc_height.revision_number(), 0);
        assert_eq!(ibc_height.revision_height(), 0);

        let h = Height::new(u64::MAX, u64::MAX);
        assert!(h.add(1).is_err());
        let res: Result<ICS02Height, _> = h.try_into();
        assert!(res.is_ok());
        let ibc_height = res.unwrap();
        assert_eq!(ibc_height.revision_number(), u64::MAX);
        assert_eq!(ibc_height.revision_height(), u64::MAX);
    }

    #[test]
    fn test_height_ordering() {
        let h1 = Height::new(0, 0);
        let h2 = Height::new(0, 1);
        let h3 = Height::new(1, 0);
        let h4 = Height::new(1, 1);

        assert!(h1 < h2);
        assert!(h1 < h3);
        assert!(h1 < h4);

        assert!(h2 < h3);
        assert!(h2 < h4);

        assert!(h3 < h4);
    }

    #[test]
    fn test_height_add_sub() {
        let h1 = Height::new(0, 0);
        let h2 = Height::new(0, 1);
        let h3 = Height::new(1, 0);
        let h4 = Height::new(1, 1);

        assert_eq!(h1.add(0).unwrap(), h1);
        assert_eq!(h1.sub(0).unwrap(), h1);

        assert_eq!(h1.add(1).unwrap(), h2);
        assert_eq!(h2.sub(1).unwrap(), h1);

        assert_eq!(h3.add(0).unwrap(), h3);
        assert_eq!(h3.sub(0).unwrap(), h3);

        assert_eq!(h3.add(1).unwrap(), h4);
        assert_eq!(h4.sub(1).unwrap(), h3);
    }
}
