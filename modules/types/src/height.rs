use crate::errors::TypeError;
use crate::prelude::*;
use core::cmp::Ordering;
use core::str::FromStr;
use ibc::core::ics02_client::error::ClientError as ICS02Error;
use ibc::core::ics02_client::height::Height as ICS02Height;
use ibc::core::ics02_client::height::HeightError;
use ibc_proto::ibc::core::client::v1::Height as RawHeight;
use ibc_proto::protobuf::Protobuf;
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

    pub fn add(&self, delta: u64) -> Height {
        Height {
            revision_number: self.revision_number,
            revision_height: self.revision_height + delta,
        }
    }

    pub fn increment(&self) -> Height {
        self.add(1)
    }

    pub fn sub(&self, delta: u64) -> Result<Height, ICS02Error> {
        if self.revision_height <= delta {
            return Err(ICS02Error::InvalidHeightResult);
        }

        Ok(Height {
            revision_number: self.revision_number,
            revision_height: self.revision_height - delta,
        })
    }

    pub fn decrement(&self) -> Result<Height, ICS02Error> {
        self.sub(1)
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
    type Error = HeightError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let split: Vec<&str> = value.split('-').collect();

        let revision_number =
            split[0]
                .parse::<u64>()
                .map_err(|error| HeightError::HeightConversion {
                    height: value.to_owned(),
                    error,
                })?;
        let revision_height =
            split[1]
                .parse::<u64>()
                .map_err(|error| HeightError::HeightConversion {
                    height: value.to_owned(),
                    error,
                })?;

        Ok(Height::new(revision_number, revision_height))
    }
}

impl From<Height> for String {
    fn from(height: Height) -> Self {
        format!("{}-{}", height.revision_number, height.revision_height)
    }
}

impl FromStr for Height {
    type Err = HeightError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Height::try_from(s)
    }
}

impl TryFrom<Height> for ICS02Height {
    type Error = ICS02Error;

    fn try_from(height: Height) -> Result<Self, Self::Error> {
        if height.revision_height() == 0 {
            // XXX this is a trick for height type conversion due to ics02 height doesn't allow "zero" height
            Ok(serde_json::from_slice(&serde_json::to_vec(&height).unwrap()).unwrap())
        } else {
            ICS02Height::new(height.revision_number(), height.revision_height())
        }
    }
}

impl From<ICS02Height> for Height {
    fn from(height: ICS02Height) -> Self {
        Height::new(height.revision_number(), height.revision_height())
    }
}

impl From<Height> for Vec<u8> {
    fn from(height: Height) -> Self {
        let mut bz: [u8; 16] = Default::default();
        bz[..8].copy_from_slice(&height.revision_number().to_be_bytes());
        bz[8..].copy_from_slice(&height.revision_height().to_be_bytes());
        bz.to_vec()
    }
}

impl TryFrom<&[u8]> for Height {
    type Error = TypeError;
    fn try_from(bz: &[u8]) -> Result<Self, Self::Error> {
        if bz.len() != 16 {
            return Err(TypeError::height_bytes_conversion(bz.into()));
        }
        let mut ar: [u8; 8] = Default::default();
        ar.copy_from_slice(&bz[..8]);
        let revision_number = u64::from_be_bytes(ar);
        ar.copy_from_slice(&bz[8..]);
        let revision_height = u64::from_be_bytes(ar);
        Ok(Height::new(revision_number, revision_height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_height_conversion() {
        let h = Height::zero();
        let res: Result<ICS02Height, _> = h.try_into();
        assert!(res.is_ok());
        let ibc_height = res.unwrap();
        assert_eq!(ibc_height.revision_number(), 0);
    }
}
