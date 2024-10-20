use alloc::string::ToString;
use core::fmt::{Display, Error as FmtError, Formatter};

use ibc_core_client::types::Height;
use ibc_core_host_types::error::DecodingError;
use ibc_primitives::proto::{Any, Protobuf};
use ibc_primitives::Timestamp;

use crate::ibc::proto::Header as RawMockHeader;

pub const MOCK_HEADER_TYPE_URL: &str = "/ibc.mock.Header";

#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MockHeader {
    pub height: Height,
    pub timestamp: Timestamp,
}

impl Default for MockHeader {
    fn default() -> Self {
        Self {
            height: Height::min(0),
            timestamp: year_2023(),
        }
    }
}

impl Display for MockHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(
            f,
            "MockHeader {{ height: {}, timestamp: {} }}",
            self.height, self.timestamp
        )
    }
}

impl Protobuf<RawMockHeader> for MockHeader {}

impl TryFrom<RawMockHeader> for MockHeader {
    type Error = DecodingError;

    fn try_from(raw: RawMockHeader) -> Result<Self, Self::Error> {
        Ok(Self {
            height: raw
                .height
                .ok_or(DecodingError::missing_raw_data("mock header height"))?
                .try_into()?,
            timestamp: Timestamp::from_nanoseconds(raw.timestamp),
        })
    }
}

impl From<MockHeader> for RawMockHeader {
    fn from(value: MockHeader) -> Self {
        Self {
            height: Some(value.height.into()),
            timestamp: value.timestamp.nanoseconds(),
        }
    }
}

impl MockHeader {
    pub fn height(&self) -> Height {
        self.height
    }

    pub fn new(height: Height) -> Self {
        Self {
            height,
            timestamp: year_2023(),
        }
    }

    // pub fn with_current_timestamp(self) -> Self {
    //     Self {
    //         timestamp: Timestamp::now(),
    //         ..self
    //     }
    // }

    pub fn with_timestamp(self, timestamp: Timestamp) -> Self {
        Self { timestamp, ..self }
    }
}

impl Protobuf<Any> for MockHeader {}

impl TryFrom<Any> for MockHeader {
    type Error = DecodingError;

    fn try_from(raw: Any) -> Result<Self, Self::Error> {
        if let MOCK_HEADER_TYPE_URL = raw.type_url.as_str() {
            Protobuf::<RawMockHeader>::decode_vec(&raw.value).map_err(Into::into)
        } else {
            Err(DecodingError::MismatchedResourceName {
                expected: MOCK_HEADER_TYPE_URL.to_string(),
                actual: raw.type_url,
            })
        }
    }
}

impl From<MockHeader> for Any {
    fn from(header: MockHeader) -> Self {
        Self {
            type_url: MOCK_HEADER_TYPE_URL.to_string(),
            value: Protobuf::<RawMockHeader>::encode_vec(header),
        }
    }
}

/// Returns a `Timestamp` representation of the beginning of year 2023.
///
/// This is introduced to initialize [`StoreGenericTestContext`](crate::context::StoreGenericTestContext)s
/// with the same latest timestamp by default.
/// If two [`StoreGenericTestContext`](crate::context::StoreGenericTestContext)
/// are initialized using [`Timestamp::now()`], the second one will have a greater timestamp than the first one.
/// So, the latest header of the second context cannot be submitted to first one.
/// We can still set a custom timestamp via [`dummy_store_generic_test_context`](crate::fixtures::core::context::dummy_store_generic_test_context).
fn year_2023() -> Timestamp {
    // Sun Jan 01 2023 00:00:00 GMT+0000
    Timestamp::from_unix_timestamp(1_672_531_200, 0).expect("should be a valid time")
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn encode_any() {
        let header = MockHeader::new(Height::new(1, 10).expect("Never fails"));
        let bytes = <MockHeader as Protobuf<Any>>::encode_vec(header);

        assert_eq!(
            &bytes,
            &[
                10, 16, 47, 105, 98, 99, 46, 109, 111, 99, 107, 46, 72, 101, 97, 100, 101, 114, 18,
                16, 10, 4, 8, 1, 16, 10, 16, 128, 128, 136, 158, 189, 200, 129, 155, 23
            ]
        );
    }
}
