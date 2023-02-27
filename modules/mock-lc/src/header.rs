use crate::errors::Error;
use core::ops::Deref;
use ibc::mock::header::{MockHeader, MOCK_HEADER_TYPE_URL};
use ibc_proto::google::protobuf::Any as IBCAny;
use lcp_types::Any;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Header(pub(crate) MockHeader);

impl Deref for Header {
    type Target = MockHeader;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<MockHeader> for Header {
    fn from(value: MockHeader) -> Self {
        Self(value)
    }
}

impl TryFrom<Any> for Header {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        let any: IBCAny = value.into();
        if any.type_url == MOCK_HEADER_TYPE_URL {
            Ok(Self(MockHeader::try_from(any).map_err(Error::ics02)?))
        } else {
            Err(Error::unexpected_client_type(any.type_url))
        }
    }
}

impl From<Header> for Any {
    fn from(value: Header) -> Self {
        IBCAny::from(value.0).into()
    }
}
